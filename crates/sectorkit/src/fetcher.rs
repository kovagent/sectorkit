//! SEC EDGAR submissions API fetcher.
//!
//! Hydrates `(ticker, CIK) -> (sic_code, sic_desc, name)` records by walking
//! `data.sec.gov/submissions/CIK<10-digit>.json` for every ticker in the
//! `company_tickers.json` index.
//!
//! SEC publishes both feeds at:
//!   * `https://www.sec.gov/files/company_tickers.json`
//!   * `https://data.sec.gov/submissions/CIK<10-digit-padded>.json`
//!
//! Per SEC's published policy, every request MUST carry a descriptive
//! `User-Agent` header. SEC enforces a server-side rate ceiling of 10
//! requests per second; this module throttles to 8 req/s to stay safely
//! under that ceiling.

use crate::error::{Error, Result};
use crate::sic::SecSector;
use crate::types::SectorMapping;
use futures::stream::{self, StreamExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

/// SEC-required descriptive identifier for ALL requests against `*.sec.gov`.
///
/// Per `https://www.sec.gov/os/accessing-edgar-data` the User-Agent MUST
/// identify the requesting party and a contact email. SEC's published
/// example follows the bare `<name> <email>` form -- richer User-Agent
/// strings with URLs and parentheses are rejected with HTTP 403.
/// Overridable via the `SECTORKIT_SEC_USER_AGENT` environment variable.
pub const DEFAULT_USER_AGENT: &str = "sectorkit email@email.com";

/// Resolve the User-Agent to send, preferring the env override when set.
pub fn resolve_user_agent() -> String {
    std::env::var("SECTORKIT_SEC_USER_AGENT").unwrap_or_else(|_| DEFAULT_USER_AGENT.to_string())
}

/// Ticker index endpoint published by SEC.
pub const COMPANY_TICKERS_URL: &str = "https://www.sec.gov/files/company_tickers.json";

/// Submissions endpoint base; append `CIK<10-digit-padded>.json`.
pub const SUBMISSIONS_BASE: &str = "https://data.sec.gov/submissions";

/// Concurrency ceiling on submissions fetches. Held at 2 so SEC's
/// burst-detection heuristics see a smoothly paced stream rather than
/// parallel bursts.
const SUBMISSIONS_CONCURRENCY: usize = 2;

/// Minimum inter-request delay enforced client-side. SEC's published
/// ceiling is 10 req/s; 130 ms = ~7.7 req/s effective, comfortably below
/// the threshold and tolerant of jitter / retries.
const REQUEST_DELAY: Duration = Duration::from_millis(130);

/// Per-request HTTP timeout for submissions calls (seconds).
pub const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Per-request HTTP timeout for submissions calls.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(REQUEST_TIMEOUT_SECS);

/// One ticker -> CIK row as published in `company_tickers.json`.
///
/// SEC publishes the JSON as a map of integer-keyed objects.
#[derive(Debug, Deserialize)]
struct TickerIndexRow {
    cik_str: u64,
    ticker: String,
    title: String,
}

/// Minimal subset of the `submissions` JSON document we need.
#[derive(Debug, Deserialize)]
struct SubmissionsDoc {
    #[serde(default)]
    cik: serde_json::Value,
    #[serde(default)]
    name: String,
    #[serde(default, rename = "sicDescription")]
    sic_description: String,
    #[serde(default, rename = "sic")]
    sic: serde_json::Value,
}

/// Build a reusable reqwest client with the SEC-required UA pinned.
fn build_client() -> Result<reqwest::Client> {
    let ua = resolve_user_agent();
    reqwest::Client::builder()
        .user_agent(ua)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(Error::from)
}

/// Fetch the SEC ticker index. ~10 k entries, ~10 MB JSON.
///
/// SEC publishes the JSON with integer-string keys ("0", "1", ...). The
/// returned vector is sorted in ascending key order so a `--limit N` walk
/// hits the first N keys (NVDA at key 0, AAPL at key 1, etc.) rather than
/// the lexicographic order ("0", "1", "10", "100", ...).
pub async fn fetch_ticker_index(client: &reqwest::Client) -> Result<Vec<TickerEntry>> {
    let resp = client
        .get(COMPANY_TICKERS_URL)
        .send()
        .await?
        .error_for_status()?;
    let raw: HashMap<String, TickerIndexRow> = resp.json().await?;
    let mut indexed: Vec<(u64, TickerIndexRow)> = raw
        .into_iter()
        .filter_map(|(k, v)| k.parse::<u64>().ok().map(|n| (n, v)))
        .collect();
    indexed.sort_by_key(|(k, _)| *k);
    Ok(indexed
        .into_iter()
        .map(|(_, row)| TickerEntry {
            ticker: row.ticker.to_uppercase(),
            cik: row.cik_str,
            title: row.title,
        })
        .collect())
}

/// One row from `company_tickers.json` after normalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickerEntry {
    /// Ticker symbol, uppercased.
    pub ticker: String,
    /// SEC CIK (Central Index Key), zero-stripped.
    pub cik: u64,
    /// Registrant title as published by SEC.
    pub title: String,
}

/// Pad a numeric CIK to the 10-digit form used in submissions URLs.
pub fn pad_cik(cik: u64) -> String {
    format!("CIK{cik:010}")
}

/// Fetch one submissions document and extract the SIC fields.
///
/// Retries up to 4 additional times on HTTP 429 (Too Many Requests) with
/// exponential backoff (1 s, 2 s, 4 s, 8 s). All other status errors
/// propagate immediately.
pub async fn fetch_submissions(client: &reqwest::Client, cik: u64) -> Result<SubmissionsRow> {
    let url = format!("{SUBMISSIONS_BASE}/{}.json", pad_cik(cik));
    let mut attempt = 0u32;
    loop {
        let resp = client.get(&url).send().await?;
        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS && attempt < 4 {
            attempt += 1;
            let backoff = Duration::from_secs(1u64 << attempt);
            tokio::time::sleep(backoff).await;
            continue;
        }
        let resp = resp.error_for_status()?;
        let doc: SubmissionsDoc = resp.json().await?;
        let sic_code = parse_sic_value(&doc.sic)?;
        let cik_resolved = parse_cik_value(&doc.cik).unwrap_or(cik);
        return Ok(SubmissionsRow {
            cik: cik_resolved,
            name: doc.name,
            sic_code,
            sic_desc: doc.sic_description,
        });
    }
}

fn parse_sic_value(v: &serde_json::Value) -> Result<u32> {
    match v {
        serde_json::Value::String(s) => s
            .trim()
            .parse::<u32>()
            .map_err(|e| Error::ParseFailed(format!("sic={s}: {e}"))),
        serde_json::Value::Number(n) => n
            .as_u64()
            .map(|x| x as u32)
            .ok_or_else(|| Error::ParseFailed(format!("sic numeric: {n}"))),
        _ => Err(Error::ParseFailed("missing sic field".into())),
    }
}

fn parse_cik_value(v: &serde_json::Value) -> Option<u64> {
    match v {
        serde_json::Value::String(s) => s.trim().parse::<u64>().ok(),
        serde_json::Value::Number(n) => n.as_u64(),
        _ => None,
    }
}

/// Submissions row after extraction.
#[derive(Debug, Clone)]
pub struct SubmissionsRow {
    /// SEC Central Index Key (CIK), zero-stripped.
    pub cik: u64,
    /// Registrant legal name as published in the submissions feed.
    pub name: String,
    /// 4-digit SIC industry code.
    pub sic_code: u32,
    /// Human-readable SIC industry description.
    pub sic_desc: String,
}

/// Hydrate the full universe by walking submissions for every ticker.
///
/// Throttles to [`SUBMISSIONS_CONCURRENCY`] in-flight requests AND inserts
/// a [`REQUEST_DELAY`] gap between dispatches to stay under the SEC's
/// published 10 req/s ceiling. Returns the successfully-resolved rows;
/// failures are logged via `tracing::warn` and dropped (a single missing
/// CIK does not abort the run).
pub async fn hydrate_universe(limit: Option<usize>) -> Result<Vec<SectorMapping>> {
    let client = build_client()?;
    let mut tickers = fetch_ticker_index(&client).await?;
    if let Some(n) = limit {
        tickers.truncate(n);
    }
    let total = tickers.len();

    let dispatches = stream::iter(tickers.into_iter().enumerate()).then(|(i, entry)| {
        let client = client.clone();
        async move {
            // Throttle dispatch: skip on the first call, sleep before each
            // subsequent one so the effective request rate stays under SEC's
            // 10 req/s ceiling.
            if i > 0 {
                tokio::time::sleep(REQUEST_DELAY).await;
            }
            (entry, client)
        }
    });

    let rows: Vec<SectorMapping> = dispatches
        .map(|(entry, client)| async move {
            match fetch_submissions(&client, entry.cik).await {
                Ok(row) => Some(SectorMapping {
                    ticker: entry.ticker,
                    cik: row.cik,
                    sic_code: row.sic_code,
                    sic_desc: row.sic_desc,
                    sector: SecSector::from_sic(row.sic_code),
                    name: if row.name.is_empty() {
                        entry.title
                    } else {
                        row.name
                    },
                }),
                Err(e) => {
                    tracing::warn!(ticker = %entry.ticker, cik = entry.cik, error = %e, "submissions fetch failed");
                    None
                }
            }
        })
        .buffer_unordered(SUBMISSIONS_CONCURRENCY)
        .filter_map(|opt| async move { opt })
        .collect()
        .await;

    tracing::info!(resolved = rows.len(), total, "hydration walk complete");
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_cik_left_zeros() {
        assert_eq!(pad_cik(320193), "CIK0000320193");
        assert_eq!(pad_cik(1), "CIK0000000001");
        assert_eq!(pad_cik(1234567890), "CIK1234567890");
    }

    #[test]
    fn parse_sic_handles_string_and_number() {
        assert_eq!(parse_sic_value(&serde_json::json!("3571")).unwrap(), 3571);
        assert_eq!(parse_sic_value(&serde_json::json!(3571)).unwrap(), 3571);
        assert!(parse_sic_value(&serde_json::json!(null)).is_err());
    }

    #[test]
    fn user_agent_format() {
        let ua = resolve_user_agent();
        assert!(ua.contains("sectorkit"));
        assert!(ua.contains('@') || std::env::var("SECTORKIT_SEC_USER_AGENT").is_ok());
    }
}
