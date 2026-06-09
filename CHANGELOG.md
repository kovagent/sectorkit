# Changelog

All notable changes to this project will be documented in this file. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0]

### Added
- `SectorTaxonomyCache` -- in-memory ticker -> SIC -> SEC division resolver.
- `SecSector` -- the 10 SEC SIC divisions (A through J) plus `Unclassified`.
- `SectorMapping` -- one resolved row carrying ticker, CIK, SIC code, SIC description, sector, and registrant name.
- `SnapshotSource` -- `LatestFromRepo`, `LocalFile`, `Embedded`.
- SEC EDGAR hydration pipeline:
  - `company_tickers.json` index fetch.
  - Per-CIK `data.sec.gov/submissions/CIK<10-digit>.json` walk.
  - Concurrency capped under SEC's published 10 req/s policy.
  - Descriptive `User-Agent` header per SEC policy (`SECTORKIT_SEC_USER_AGENT` override).
- Parquet + JSON snapshot serialization under `data/<YYYY-MM-DD>/sectors.{parquet,json}`.
- Crate-embedded fallback snapshot of the most-watched US large-caps.
- `sectorkit-cli` with `resolve`, `list`, `summary`, `hydrate` subcommands.
- GitHub Actions workflows: `ci.yml` (fmt + clippy + tests), `nightly-refresh.yml` (06:00 UTC hydration), `release.yml` (tag-triggered crates.io publish).
