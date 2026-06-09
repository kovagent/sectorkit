# sectorkit

SEC EDGAR SIC industry/sector taxonomy per ticker for Rust. Companion to [`indexkit`](https://github.com/userFRM/indexkit): where `indexkit` answers "which tickers belong to a given index", this crate answers "which industry / sector does a given ticker belong to" using the only fully-open US public-market sector taxonomy: the SEC's Standard Industrial Classification (SIC) divisions.

## Why SIC

GICS (S&P / MSCI) and ICB (FTSE) are the institutional sector standards but are licensed and cannot be redistributed. SIC is in the public domain, published by the US government, and remains the authoritative taxonomy SEC uses for every filer's cover-page metadata. It is coarser than GICS but is sufficient for breadth indicators, sector-rotation strength, McClellan oscillators, and other market-aggregate analytics that need a stable symbol-to-sector mapping.

## Sectors

`sectorkit` exposes the 10 top-level SEC divisions:

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

## Install

```toml
[dependencies]
sectorkit = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Usage

```rust
use sectorkit::{SectorTaxonomyCache, SnapshotSource, SecSector};

#[tokio::main]
async fn main() -> sectorkit::Result<()> {
    let cache = SectorTaxonomyCache::hydrate(SnapshotSource::LatestFromRepo).await?;

    if let Some(row) = cache.resolve_ticker("AAPL") {
        println!("{} (CIK {}) -> SIC {} {} -> {}",
            row.ticker, row.cik, row.sic_code, row.sic_desc, row.sector);
    }

    let financials = cache.resolve_sector(SecSector::FinanceInsuranceRealEstate);
    println!("{} financials tracked", financials.len());

    Ok(())
}
```

### Offline mode

The crate ships an embedded fallback snapshot of the most-watched US large-caps so that downstream tests and offline scripts work without network access:

```rust
let cache = SectorTaxonomyCache::hydrate(SnapshotSource::Embedded).await?;
```

For a custom-curated snapshot, point at a local Parquet or JSON file:

```rust
use std::path::PathBuf;
let cache = SectorTaxonomyCache::hydrate(
    SnapshotSource::LocalFile(PathBuf::from("./data/2026-06-09/sectors.parquet"))
).await?;
```

## Data refresh

A nightly GitHub Actions workflow (`.github/workflows/nightly-refresh.yml`) walks every SEC ticker via the EDGAR submissions API, materializes the resolved taxonomy to `data/<YYYY-MM-DD>/sectors.{parquet,json}`, and commits the snapshot. Consumers using `SnapshotSource::LatestFromRepo` automatically pick up the most recent committed snapshot.

The walk respects SEC's published policy:

- Every request carries a descriptive `User-Agent` (override via `SECTORKIT_SEC_USER_AGENT`).
- In-flight request concurrency is capped under SEC's published 10 req/s ceiling.

## Methodology

SIC = Standard Industrial Classification (US Department of Labor, OSHA, established 1937). SEC has published the SIC code of every registrant on the cover page of 10-K filings since 1939 and exposes the live value on the JSON submissions endpoint documented at `https://www.sec.gov/cgi-bin/browse-edgar?action=getcurrent`.

Data sources:

- `https://www.sec.gov/files/company_tickers.json` -- ticker -> CIK index.
- `https://data.sec.gov/submissions/CIK<10-digit-padded>.json` -- per-filer `sicCode`, `sicDescription`, `name`.

Division-range mapping follows the SEC SIC list at `https://www.sec.gov/info/edgar/siccodes.htm`.

## CLI

```sh
cargo install sectorkit-cli

sectorkit-cli resolve AAPL
sectorkit-cli list services
sectorkit-cli summary
sectorkit-cli hydrate --limit 100
```

## License

Apache-2.0. See [LICENSE](LICENSE).
