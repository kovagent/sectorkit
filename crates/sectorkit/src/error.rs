//! Unified error type for sectorkit.
//!
//! All public methods return `sectorkit::Result<T>` which is
//! `std::result::Result<T, sectorkit::Error>`.

use thiserror::Error;

/// The single unified error type for sectorkit operations.
///
/// Match on this enum when you need to distinguish error kinds; otherwise
/// `?` propagates it through any `Result<_, sectorkit::Error>` context.
#[derive(Debug, Error)]
pub enum Error {
    /// Ticker not present in the loaded sector cache.
    #[error("unknown ticker: {0}")]
    UnknownTicker(String),

    /// Hydration of a snapshot from the network failed.
    #[error("hydration failed: {0}")]
    HydrationFailed(String),

    /// The requested snapshot file is not present on disk or in the repository.
    #[error("snapshot not found: {0}")]
    SnapshotNotFound(String),

    /// Parsing of a snapshot (Parquet or JSON) failed.
    #[error("parse failed: {0}")]
    ParseFailed(String),

    /// Underlying HTTP transport error.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    /// I/O error (file system, tempfile, etc.).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Arrow columnar format error (from parquet reading).
    #[error("arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),

    /// Native parquet crate error.
    #[error("parquet error: {0}")]
    ParquetNative(#[from] parquet::errors::ParquetError),

    /// JSON parse error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Any other error not covered by the specific variants above.
    #[error("{0}")]
    Other(String),
}

/// `Result<T>` alias using [`enum@Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Alias for [`enum@Error`] kept for parity with sibling crates.
///
/// Code that references `sectorkit::SectorkitError` compiles.
pub type SectorkitError = Error;

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::Other(e.to_string())
    }
}
