# Contributing

Thanks for your interest in sectorkit. Bug reports, fixes, and small improvements are welcome.

## Reporting issues

Open an issue with a minimal reproduction: the call you made, what you expected, what happened, the crate version, and your OS.

## Pull requests

Keep each change focused on one concern. Run the full local gate before pushing and make sure every check passes: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`, and `cargo audit`. Match the existing code style, and avoid adding a dependency where a few lines would do. Use a Conventional Commits title.

## Data

The bundled parquet under `data/` is refreshed by an automated nightly job. Do not hand-edit it. If a value looks wrong, open an issue.

## License

By contributing you agree that your contributions are dual-licensed under MIT OR Apache-2.0.
