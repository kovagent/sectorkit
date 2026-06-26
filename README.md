# sectorkit

SEC EDGAR SIC industry/sector taxonomy per ticker for Rust. Served from bundled parquet with on-demand fetch and a local cache. No API keys. Offline after the first query.

## Install

```toml
[dependencies]
sectorkit = "0.1"
tokio = { version = "1", features = ["full"] }
```

Until it is published to crates.io, depend on the repository directly:

```toml
sectorkit = { git = "https://github.com/userFRM/sectorkit" }
```

## Quick start

```rust,no_run
use sectorkit::{SecSector, SectorTaxonomyCache, SnapshotSource};

#[tokio::main]
async fn main() -> sectorkit::Result<()> {
    let cache = SectorTaxonomyCache::hydrate(SnapshotSource::LatestFromRepo).await?;

    if let Some(row) = cache.resolve_ticker("AAPL") {
        println!(
            "{} (CIK {}) -> SIC {} {} -> {}",
            row.ticker, row.cik, row.sic_code, row.sic_desc, row.sector
        );
    }

    let financials = cache.resolve_sector(SecSector::FinanceInsuranceRealEstate);
    println!("{} financials tracked", financials.len());

    Ok(())
}
```

## Client pattern

Hydrate a `SectorTaxonomyCache` once from a `SnapshotSource`, then resolve against it in memory. `LatestFromRepo` pulls the most recent committed snapshot, `LocalFile` reads a Parquet or JSON file you point at, and `Embedded` uses the crate-bundled snapshot for offline use.

```rust,no_run
use std::path::PathBuf;
use sectorkit::{SectorTaxonomyCache, SnapshotSource};

async fn load() -> sectorkit::Result<()> {
    // Offline: the crate-embedded snapshot, no network.
    let cache = SectorTaxonomyCache::hydrate(SnapshotSource::Embedded).await?;

    // Or a custom-curated snapshot from disk.
    let cache = SectorTaxonomyCache::hydrate(SnapshotSource::LocalFile(
        PathBuf::from("./data/2026-06-09/sectors.parquet"),
    ))
    .await?;

    // Resolve against the in-memory index.
    let row = cache.require_ticker("MSFT")?;
    println!("{} -> {}", row.ticker, row.sector);

    for (sector, count) in cache.sector_counts() {
        println!("{sector}: {count}");
    }

    Ok(())
}
```

## Sectors

`sectorkit` exposes the 10 top-level SEC SIC divisions as `SecSector`. SIC is in the public domain, published by the US government, and remains the authoritative taxonomy SEC records for every filer's cover-page metadata, so it can be redistributed where the licensed GICS and ICB standards cannot.

| Division | Sector                                  | SIC range |
|----------|-----------------------------------------|-----------|
| A        | AgricultureForestryFishing              | 0100-0999 |
| B        | Mining                                  | 1000-1499 |
| C        | Construction                            | 1500-1799 |
| D        | Manufacturing                           | 2000-3999 |
| E        | TransportationCommunicationsUtilities   | 4000-4999 |
| F        | WholesaleTrade                          | 5000-5199 |
| G        | RetailTrade                             | 5200-5999 |
| H        | FinanceInsuranceRealEstate              | 6000-6799 |
| I        | Services                                | 7000-8999 |
| J        | PublicAdministration                    | 9100-9729 |

Boundaries follow SEC's published list at `https://www.sec.gov/info/edgar/siccodes.htm`.

## CLI

```bash
cargo install sectorkit-cli

sectorkit-cli resolve AAPL
sectorkit-cli list services
sectorkit-cli summary
sectorkit-cli hydrate --limit 100
```

## Data

Each snapshot maps every resolved SEC ticker to its CIK, 4-digit SIC code, SIC description, and top-level division. Snapshots are versioned by date under `data/<YYYY-MM-DD>/` as Parquet and JSON.

The taxonomy is the SEC's Standard Industrial Classification, established 1937 by the US Department of Labor. SIC codes and descriptions are US government public-domain data sourced from SEC EDGAR.

## Cache

`SectorTaxonomyCache` holds the snapshot in memory after `hydrate`, indexed for `O(1)` ticker lookup and grouped by sector. Resolution methods (`resolve_ticker`, `require_ticker`, `resolve_sector`, `sector_counts`, `iter`) run against the in-memory index with no further network access.

## API

Full API reference is on [docs.rs](https://docs.rs/sectorkit).

## License

Dual-licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
