//! Command-line interface for sectorkit.
//!
//! Subcommands:
//!   * `resolve <TICKER>` -- look up one ticker against the latest published snapshot.
//!   * `list <SECTOR>` -- list every ticker in one SEC sector division.
//!   * `hydrate` -- pull the full SEC universe and materialize a fresh snapshot under `data/<date>/`.

use anyhow::Context;
use chrono::Utc;
use clap::{Parser, Subcommand};
use sectorkit::hydration::{hydrate_universe, write_json, write_parquet};
use sectorkit::{SecSector, SectorTaxonomyCache, SnapshotSource};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "sectorkit-cli",
    version,
    about = "SEC EDGAR SIC sector taxonomy"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Resolve one ticker against the latest published snapshot.
    Resolve {
        /// Ticker to look up (case-insensitive).
        ticker: String,
        /// Read from a local snapshot file instead of the published repo.
        #[arg(long)]
        snapshot: Option<PathBuf>,
        /// Use only the crate-embedded fallback snapshot (offline).
        #[arg(long)]
        embedded: bool,
    },
    /// List every ticker in one SEC sector division.
    List {
        /// Sector slug (e.g. `manufacturing`, `finance_insurance_real_estate`, `services`).
        sector: String,
        /// Read from a local snapshot file instead of the published repo.
        #[arg(long)]
        snapshot: Option<PathBuf>,
        /// Use only the crate-embedded fallback snapshot (offline).
        #[arg(long)]
        embedded: bool,
    },
    /// Pull the full SEC universe and materialize a fresh snapshot under `data/<date>/`.
    Hydrate {
        /// Optional cap on the number of tickers to walk (smoke-test mode).
        #[arg(long)]
        limit: Option<usize>,
        /// Output directory root. Defaults to `data/`.
        #[arg(long, default_value = "data")]
        out_dir: PathBuf,
        /// Override the date stamp in the output path (defaults to today, UTC).
        #[arg(long)]
        date: Option<String>,
    },
    /// Print sector-count summary for a snapshot.
    Summary {
        /// Read from a local snapshot file instead of the published repo.
        #[arg(long)]
        snapshot: Option<PathBuf>,
        /// Use only the crate-embedded fallback snapshot (offline).
        #[arg(long)]
        embedded: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Resolve {
            ticker,
            snapshot,
            embedded,
        } => {
            let cache = load_cache(snapshot, embedded).await?;
            match cache.resolve_ticker(&ticker) {
                Some(row) => {
                    println!("{}", serde_json::to_string_pretty(row)?);
                }
                None => {
                    eprintln!("ticker not found: {ticker}");
                    std::process::exit(1);
                }
            }
        }
        Command::List {
            sector,
            snapshot,
            embedded,
        } => {
            let cache = load_cache(snapshot, embedded).await?;
            let sec = SecSector::from_slug(&sector)
                .with_context(|| format!("unknown sector slug: {sector}"))?;
            for ticker in cache.resolve_sector(sec) {
                println!("{ticker}");
            }
        }
        Command::Hydrate {
            limit,
            out_dir,
            date,
        } => {
            let stamp = date.unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
            let snap_dir = out_dir.join(&stamp);
            std::fs::create_dir_all(&snap_dir)?;

            tracing::info!(stamp = %stamp, dir = ?snap_dir, "starting hydration");
            let rows = hydrate_universe(limit).await?;
            tracing::info!(rows = rows.len(), "hydration complete");

            let parquet_path = snap_dir.join("sectors.parquet");
            let json_path = snap_dir.join("sectors.json");
            write_parquet(&parquet_path, &rows)?;
            write_json(&json_path, &rows)?;
            tracing::info!(parquet = ?parquet_path, json = ?json_path, "wrote snapshot");
        }
        Command::Summary { snapshot, embedded } => {
            let cache = load_cache(snapshot, embedded).await?;
            println!("rows: {}", cache.len());
            for (sector, count) in cache.sector_counts() {
                println!("{count:>8}  {sector}");
            }
        }
    }
    Ok(())
}

async fn load_cache(
    snapshot: Option<PathBuf>,
    embedded: bool,
) -> anyhow::Result<SectorTaxonomyCache> {
    let source = match (snapshot, embedded) {
        (Some(p), _) => SnapshotSource::LocalFile(p),
        (None, true) => SnapshotSource::Embedded,
        (None, false) => SnapshotSource::LatestFromRepo,
    };
    Ok(SectorTaxonomyCache::hydrate(source).await?)
}
