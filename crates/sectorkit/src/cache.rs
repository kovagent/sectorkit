//! In-memory taxonomy cache loaded from a [`SnapshotSource`].
//!
//! [`SnapshotSource`]: crate::types::SnapshotSource

use crate::embedded::embedded_rows;
use crate::error::{Error, Result};
use crate::fetcher::{resolve_user_agent, REQUEST_TIMEOUT_SECS};
use crate::parquet_io::{read_json, read_parquet};
use crate::sic::SecSector;
use crate::types::{SectorMapping, SnapshotSource};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// GitHub raw-content origin for the published snapshot files.
const REPO_RAW_BASE: &str = "https://raw.githubusercontent.com/userFRM/sectorkit/main/data";

/// In-memory ticker -> sector cache.
///
/// Construct via [`SectorTaxonomyCache::hydrate`]; resolve via
/// [`SectorTaxonomyCache::resolve_ticker`] /
/// [`SectorTaxonomyCache::resolve_sector`].
#[derive(Debug, Clone)]
pub struct SectorTaxonomyCache {
    by_ticker: HashMap<String, SectorMapping>,
    by_sector: HashMap<SecSector, Vec<String>>,
}

impl SectorTaxonomyCache {
    /// Hydrate from the requested [`SnapshotSource`].
    pub async fn hydrate(source: SnapshotSource) -> Result<Self> {
        let rows = match source {
            SnapshotSource::Embedded => embedded_rows(),
            SnapshotSource::LocalFile(path) => load_local(&path)?,
            SnapshotSource::LatestFromRepo => fetch_latest_from_repo().await?,
        };
        Ok(Self::from_rows(rows))
    }

    /// Build a cache directly from rows (escape hatch for testing and custom
    /// data sources).
    pub fn from_rows(rows: Vec<SectorMapping>) -> Self {
        let mut by_ticker = HashMap::with_capacity(rows.len());
        let mut by_sector: HashMap<SecSector, Vec<String>> = HashMap::new();
        for row in rows {
            by_sector
                .entry(row.sector)
                .or_default()
                .push(row.ticker.clone());
            by_ticker.insert(row.ticker.clone(), row);
        }
        for tickers in by_sector.values_mut() {
            tickers.sort();
            tickers.dedup();
        }
        Self {
            by_ticker,
            by_sector,
        }
    }

    /// Number of distinct tickers in the cache.
    pub fn len(&self) -> usize {
        self.by_ticker.len()
    }

    /// True if the cache holds no rows.
    pub fn is_empty(&self) -> bool {
        self.by_ticker.is_empty()
    }

    /// Resolve a single ticker. Case-insensitive on the input.
    pub fn resolve_ticker(&self, ticker: &str) -> Option<&SectorMapping> {
        let key = ticker.to_ascii_uppercase();
        self.by_ticker.get(&key)
    }

    /// Resolve a single ticker or return [`Error::UnknownTicker`].
    pub fn require_ticker(&self, ticker: &str) -> Result<&SectorMapping> {
        self.resolve_ticker(ticker)
            .ok_or_else(|| Error::UnknownTicker(ticker.to_string()))
    }

    /// Tickers grouped under one SEC sector division.
    pub fn resolve_sector(&self, sector: SecSector) -> Vec<&str> {
        self.by_sector
            .get(&sector)
            .map(|v| v.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// All sectors that contain at least one ticker, with their row counts.
    pub fn sector_counts(&self) -> Vec<(SecSector, usize)> {
        let mut counts: Vec<(SecSector, usize)> =
            self.by_sector.iter().map(|(s, v)| (*s, v.len())).collect();
        counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        counts
    }

    /// Iterate over every row in the cache (sorted by ticker for determinism).
    pub fn iter(&self) -> impl Iterator<Item = &SectorMapping> {
        let mut keys: Vec<&String> = self.by_ticker.keys().collect();
        keys.sort();
        keys.into_iter().filter_map(|k| self.by_ticker.get(k))
    }
}

fn load_local(path: &Path) -> Result<Vec<SectorMapping>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "parquet" => read_parquet(path),
        "json" => read_json(path),
        other => Err(Error::ParseFailed(format!(
            "unsupported snapshot extension: .{other}"
        ))),
    }
}

async fn fetch_latest_from_repo() -> Result<Vec<SectorMapping>> {
    let client = reqwest::Client::builder()
        .user_agent(resolve_user_agent())
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()?;
    let date = fetch_latest_date(&client).await?;
    let url = format!("{REPO_RAW_BASE}/{date}/sectors.json");
    let body = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let rows: Vec<SectorMapping> = serde_json::from_slice(&body)?;
    Ok(rows)
}

async fn fetch_latest_date(client: &reqwest::Client) -> Result<String> {
    let url = "https://api.github.com/repos/userFRM/sectorkit/contents/data";
    let entries: serde_json::Value = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut dates: Vec<String> = entries
        .as_array()
        .ok_or_else(|| Error::SnapshotNotFound("data/ listing".into()))?
        .iter()
        .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(String::from))
        .filter(|s| s.len() == 10 && &s[4..5] == "-" && &s[7..8] == "-")
        .collect();
    dates.sort();
    dates
        .pop()
        .ok_or_else(|| Error::SnapshotNotFound("no dated snapshot under data/".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sic::SecSector;

    #[tokio::test]
    async fn embedded_resolves_known_anchors() {
        let cache = SectorTaxonomyCache::hydrate(SnapshotSource::Embedded)
            .await
            .unwrap();

        let aapl = cache.resolve_ticker("aapl").unwrap();
        assert_eq!(aapl.ticker, "AAPL");
        assert_eq!(aapl.sic_code, 3571);
        assert_eq!(aapl.sector, SecSector::Manufacturing);

        let jpm = cache.require_ticker("JPM").unwrap();
        assert_eq!(jpm.sector, SecSector::FinanceInsuranceRealEstate);

        assert!(cache.resolve_ticker("ZZZZNOTREAL").is_none());
        assert!(cache.require_ticker("ZZZZNOTREAL").is_err());
    }

    #[tokio::test]
    async fn embedded_groups_by_sector() {
        let cache = SectorTaxonomyCache::hydrate(SnapshotSource::Embedded)
            .await
            .unwrap();
        let mfg = cache.resolve_sector(SecSector::Manufacturing);
        assert!(mfg.contains(&"AAPL"));
        assert!(mfg.contains(&"BA"));
        let svc = cache.resolve_sector(SecSector::Services);
        assert!(svc.contains(&"MSFT"));
        assert!(svc.contains(&"GOOGL"));
    }

    #[tokio::test]
    async fn embedded_non_empty() {
        let cache = SectorTaxonomyCache::hydrate(SnapshotSource::Embedded)
            .await
            .unwrap();
        assert!(!cache.is_empty());
        assert!(cache.len() >= 50);
    }
}
