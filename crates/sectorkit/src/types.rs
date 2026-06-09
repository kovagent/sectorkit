//! Public data types: per-ticker sector mapping and snapshot source enum.

use crate::sic::SecSector;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// One resolved row: ticker -> CIK -> SIC -> sector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectorMapping {
    /// Primary listed ticker symbol (uppercased, no exchange prefix).
    pub ticker: String,
    /// SEC Central Index Key (CIK), zero-stripped.
    pub cik: u64,
    /// 4-digit SIC industry code as published by SEC.
    pub sic_code: u32,
    /// SEC-published SIC description string (industry-level, more specific than sector).
    pub sic_desc: String,
    /// Top-level SEC division this SIC code belongs to.
    pub sector: SecSector,
    /// Registrant legal name from SEC submissions.
    pub name: String,
}

/// Where to source the materialized snapshot from.
#[derive(Debug, Clone)]
pub enum SnapshotSource {
    /// Pull the most recent `data/<date>/sectors.parquet` from the
    /// `userFRM/sectorkit` GitHub raw content.
    LatestFromRepo,
    /// Read a Parquet or JSON file from the local filesystem.
    LocalFile(PathBuf),
    /// Fall back to the crate-embedded snapshot for offline use.
    ///
    /// The embedded snapshot covers a curated set of large-cap US tickers
    /// and is intended as a degraded-mode default, not a full universe.
    Embedded,
}
