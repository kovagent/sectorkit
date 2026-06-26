# sectorkit

SEC EDGAR SIC industry/sector taxonomy per ticker for Rust, companion to indexkit.

```toml
[dependencies]
sectorkit = "0.1.0"
```

```rust,no_run
use sectorkit::{SectorTaxonomyCache, SnapshotSource};

# async fn run() -> sectorkit::Result<()> {
let cache = SectorTaxonomyCache::hydrate(SnapshotSource::Embedded).await?;
if let Some(row) = cache.resolve_ticker("AAPL") {
    println!("{} -> {}", row.ticker, row.sector);
}
# Ok(())
# }
```

Full documentation: <https://github.com/userFRM/sectorkit>

Licensed under MIT OR Apache-2.0.
