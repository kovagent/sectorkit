//! `sectorkit` -- SEC EDGAR SIC industry/sector taxonomy per ticker.
//!
//! Companion library to [`indexkit`](https://crates.io/crates/indexkit).
//! Where `indexkit` answers "which tickers belong to a given index", this
//! crate answers "which industry / sector does a given ticker belong to",
//! using the only fully-open US public-market sector taxonomy: the SEC's
//! Standard Industrial Classification (SIC) divisions.
//!
//! # Methodology
//!
//! SIC = Standard Industrial Classification, established 1937 by the US
//! Department of Labor / OSHA. SEC has published the SIC code of every
//! registrant on the cover page of 10-K filings since 1939 and exposes
//! the live value on the JSON submissions endpoint at
//! `https://data.sec.gov/submissions/CIK<10-digit>.json`.
//!
//! SIC codes group into 10 top-level **divisions** A-J, which this crate
//! exposes as [`SecSector`]:
//!
//! | Division | Sector                                          | SIC range  |
//! |----------|-------------------------------------------------|------------|
//! | A        | [`SecSector::AgricultureForestryFishing`]       | 0100-0999  |
//! | B        | [`SecSector::Mining`]                           | 1000-1499  |
//! | C        | [`SecSector::Construction`]                     | 1500-1799  |
//! | D        | [`SecSector::Manufacturing`]                    | 2000-3999  |
//! | E        | [`SecSector::TransportationCommunicationsUtilities`] | 4000-4999 |
//! | F        | [`SecSector::WholesaleTrade`]                   | 5000-5199  |
//! | G        | [`SecSector::RetailTrade`]                      | 5200-5999  |
//! | H        | [`SecSector::FinanceInsuranceRealEstate`]       | 6000-6799  |
//! | I        | [`SecSector::Services`]                         | 7000-8999  |
//! | J        | [`SecSector::PublicAdministration`]             | 9100-9729  |
//!
//! Division boundaries follow SEC's published list at
//! `https://www.sec.gov/info/edgar/siccodes.htm`.
//!
//! # Why SIC and not GICS / ICB
//!
//! GICS (S&P / MSCI) and ICB (FTSE) are the institutional standards but
//! are licensed and cannot be redistributed. SIC is in the public domain,
//! published by a US government agency, and remains the authoritative
//! taxonomy SEC uses for filer metadata. It is coarser than GICS but is
//! sufficient for breadth, rotation, and McClellan-style aggregates.
//!
//! # Quick start
//!
//! ```no_run
//! use sectorkit::{SectorTaxonomyCache, SnapshotSource, SecSector};
//!
//! # async fn run() -> sectorkit::Result<()> {
//! let cache = SectorTaxonomyCache::hydrate(SnapshotSource::LatestFromRepo).await?;
//!
//! if let Some(row) = cache.resolve_ticker("AAPL") {
//!     println!("{} -> SIC {} ({}) -> {}", row.ticker, row.sic_code, row.sic_desc, row.sector);
//! }
//!
//! let banks = cache.resolve_sector(SecSector::FinanceInsuranceRealEstate);
//! println!("{} financials tracked", banks.len());
//! # Ok(())
//! # }
//! ```
//!
//! # Offline mode
//!
//! Pass [`SnapshotSource::Embedded`] to use the crate-bundled snapshot of
//! the most-watched ~100 US large-caps. Pass
//! [`SnapshotSource::LocalFile`] to read a previously downloaded
//! `sectors.parquet` or `sectors.json`.
//!
//! # SEC rate limit
//!
//! All requests against `*.sec.gov` carry a descriptive `User-Agent` per
//! SEC policy. The hydration path throttles to under SEC's published
//! 10 req/s ceiling. Override the UA via `SECTORKIT_SEC_USER_AGENT`.
//!
//! # Major types
//!
//! - [`SectorTaxonomyCache`] -- in-memory ticker -> sector cache.
//! - [`SectorMapping`] -- one resolved row.
//! - [`SecSector`] -- the 10 SEC divisions.
//! - [`SnapshotSource`] -- where to load the snapshot from.
//! - [`Error`] -- unified error type.

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

mod cache;
mod embedded;
mod error;
mod fetcher;
mod parquet_io;
mod sic;
mod types;

pub use cache::SectorTaxonomyCache;
pub use error::{Error, Result, SectorkitError};
pub use sic::SecSector;
pub use types::{SectorMapping, SnapshotSource};

/// Re-exports useful to scripts and the CLI.
pub mod hydration {
    pub use crate::fetcher::{
        fetch_ticker_index, hydrate_universe, pad_cik, resolve_user_agent, SubmissionsRow,
        TickerEntry, COMPANY_TICKERS_URL, DEFAULT_USER_AGENT, SUBMISSIONS_BASE,
    };
    pub use crate::parquet_io::{read_json, read_parquet, write_json, write_parquet};
}

/// Number of rows in the crate-embedded fallback snapshot.
///
/// Useful for tests that want to assert a non-empty default snapshot
/// without performing network I/O.
pub const fn embedded_row_count() -> usize {
    embedded::embedded_count()
}
