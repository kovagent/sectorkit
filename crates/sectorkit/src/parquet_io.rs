//! Parquet and JSON serialization for `data/<date>/sectors.{parquet,json}`.
//!
//! Snapshot files use a stable column layout so consumers can read with
//! plain Arrow / DuckDB / polars without a schema-evolution layer:
//!
//! | column     | type   | nullable |
//! |------------|--------|----------|
//! | ticker     | utf8   | false    |
//! | cik        | uint64 | false    |
//! | sic_code   | uint32 | false    |
//! | sic_desc   | utf8   | false    |
//! | sector     | utf8   | false    |
//! | name       | utf8   | false    |

use crate::error::{Error, Result};
use crate::sic::SecSector;
use crate::types::SectorMapping;
use arrow::array::{ArrayRef, RecordBatch, StringArray, UInt32Array, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// Arrow schema for `sectors.parquet`.
pub fn schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ticker", DataType::Utf8, false),
        Field::new("cik", DataType::UInt64, false),
        Field::new("sic_code", DataType::UInt32, false),
        Field::new("sic_desc", DataType::Utf8, false),
        Field::new("sector", DataType::Utf8, false),
        Field::new("name", DataType::Utf8, false),
    ]))
}

/// Materialize a slice of [`SectorMapping`] into a single Arrow [`RecordBatch`].
pub fn rows_to_batch(rows: &[SectorMapping]) -> Result<RecordBatch> {
    let tickers: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.ticker.as_str()),
    ));
    let ciks: ArrayRef = Arc::new(UInt64Array::from_iter_values(rows.iter().map(|r| r.cik)));
    let sics: ArrayRef = Arc::new(UInt32Array::from_iter_values(
        rows.iter().map(|r| r.sic_code),
    ));
    let descs: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.sic_desc.as_str()),
    ));
    let sectors: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.sector.as_slug()),
    ));
    let names: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.name.as_str()),
    ));

    RecordBatch::try_new(schema(), vec![tickers, ciks, sics, descs, sectors, names])
        .map_err(Error::from)
}

/// Reverse of [`rows_to_batch`]: a [`RecordBatch`] into [`SectorMapping`] rows.
pub fn batch_to_rows(batch: &RecordBatch) -> Result<Vec<SectorMapping>> {
    let n = batch.num_rows();
    let tickers = column_as_string(batch, 0, "ticker")?;
    let ciks = column_as_u64(batch, 1, "cik")?;
    let sics = column_as_u32(batch, 2, "sic_code")?;
    let descs = column_as_string(batch, 3, "sic_desc")?;
    let sectors = column_as_string(batch, 4, "sector")?;
    let names = column_as_string(batch, 5, "name")?;

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let sector = SecSector::from_slug(sectors.value(i)).unwrap_or(SecSector::Unclassified);
        out.push(SectorMapping {
            ticker: tickers.value(i).to_string(),
            cik: ciks.value(i),
            sic_code: sics.value(i),
            sic_desc: descs.value(i).to_string(),
            sector,
            name: names.value(i).to_string(),
        });
    }
    Ok(out)
}

fn column_as_string<'a>(b: &'a RecordBatch, idx: usize, name: &str) -> Result<&'a StringArray> {
    b.column(idx)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| Error::ParseFailed(format!("column {name} not Utf8")))
}

fn column_as_u64<'a>(b: &'a RecordBatch, idx: usize, name: &str) -> Result<&'a UInt64Array> {
    b.column(idx)
        .as_any()
        .downcast_ref::<UInt64Array>()
        .ok_or_else(|| Error::ParseFailed(format!("column {name} not UInt64")))
}

fn column_as_u32<'a>(b: &'a RecordBatch, idx: usize, name: &str) -> Result<&'a UInt32Array> {
    b.column(idx)
        .as_any()
        .downcast_ref::<UInt32Array>()
        .ok_or_else(|| Error::ParseFailed(format!("column {name} not UInt32")))
}

/// Write a slice of [`SectorMapping`] to a Parquet file at `path`.
pub fn write_parquet(path: &Path, rows: &[SectorMapping]) -> Result<()> {
    let batch = rows_to_batch(rows)?;
    let file = File::create(path)?;
    let props = WriterProperties::builder()
        .set_compression(Compression::ZSTD(Default::default()))
        .build();
    let mut writer = ArrowWriter::try_new(file, schema(), Some(props))?;
    writer.write(&batch)?;
    writer.close()?;
    Ok(())
}

/// Read a Parquet file at `path` into [`SectorMapping`] rows.
pub fn read_parquet(path: &Path) -> Result<Vec<SectorMapping>> {
    let file = File::open(path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
    let reader = builder.build()?;
    let mut out = Vec::new();
    for batch in reader {
        let batch = batch?;
        out.extend(batch_to_rows(&batch)?);
    }
    Ok(out)
}

/// Write rows as pretty-printed JSON suitable for `data/<date>/sectors.json`.
pub fn write_json(path: &Path, rows: &[SectorMapping]) -> Result<()> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, rows)?;
    Ok(())
}

/// Read rows from a JSON snapshot.
pub fn read_json(path: &Path) -> Result<Vec<SectorMapping>> {
    let file = File::open(path)?;
    let rows: Vec<SectorMapping> = serde_json::from_reader(file)?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample() -> Vec<SectorMapping> {
        vec![
            SectorMapping {
                ticker: "AAPL".into(),
                cik: 320193,
                sic_code: 3571,
                sic_desc: "ELECTRONIC COMPUTERS".into(),
                sector: SecSector::Manufacturing,
                name: "Apple Inc.".into(),
            },
            SectorMapping {
                ticker: "JPM".into(),
                cik: 19617,
                sic_code: 6020,
                sic_desc: "STATE COMMERCIAL BANKS".into(),
                sector: SecSector::FinanceInsuranceRealEstate,
                name: "JPMORGAN CHASE & CO".into(),
            },
        ]
    }

    #[test]
    fn parquet_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sectors.parquet");
        let rows = sample();
        write_parquet(&path, &rows).unwrap();
        let back = read_parquet(&path).unwrap();
        assert_eq!(rows, back);
    }

    #[test]
    fn json_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sectors.json");
        let rows = sample();
        write_json(&path, &rows).unwrap();
        let back = read_json(&path).unwrap();
        assert_eq!(rows, back);
    }
}
