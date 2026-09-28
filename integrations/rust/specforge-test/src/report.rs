use crate::ports::RealFs;
use crate::registry::TestRecordEntry;
use serde::Serialize;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Wire schema version of the report JSON.
pub const REPORT_SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Serialize)]
pub struct BinaryReport {
    pub binary_name: String,
    pub entries: Vec<TestRecordEntry>,
}

#[derive(Debug, Serialize)]
struct BinaryReportRef<'a> {
    schema_version: &'a str,
    binary_name: &'a str,
    entries: &'a [TestRecordEntry],
}

/// Serialize the report payload (shared by the port-routed writer).
pub fn serialize_report(
    binary_name: &str,
    entries: &[TestRecordEntry],
) -> serde_json::Result<Vec<u8>> {
    let mut sorted = entries.to_vec();
    sorted.sort_by(|a, b| {
        a.entity_id
            .cmp(&b.entity_id)
            .then(a.test_name.cmp(&b.test_name))
    });

    let report = BinaryReportRef {
        schema_version: REPORT_SCHEMA_VERSION,
        binary_name,
        entries: &sorted,
    };
    let mut bytes = Vec::new();
    serde_json::to_writer(&mut bytes, &report)?;
    Ok(bytes)
}

/// Finalize path routed through the ReportWriter port (C11-08) so hermetic
/// tests can inject a fake.
pub fn write_report_with(
    fs: &dyn crate::ports::ReportWriter,
    dir: &Path,
    binary_name: &str,
    entries: &[TestRecordEntry],
) -> std::io::Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    fs.create_dir_all(dir)?;

    let bytes = serialize_report(binary_name, entries).map_err(std::io::Error::other)?;

    let path = dir.join(format!("{binary_name}.json"));
    // C6-08: temp-file-plus-rename — a crash or full disk mid-write must
    // not leave a truncated {binary_name}.json (same policy as
    // persist_schema_cache).
    let tmp = dir.join(format!(".{binary_name}.json.tmp"));
    // C6-15: compact JSON is streamed straight into a buffered file writer —
    // no intermediate String and no pretty-print whitespace.
    let file = std::fs::File::create(&tmp)?;
    {
        let mut writer = BufWriter::new(file);
        writer.write_all(&bytes)?;
        writer.flush()?;
    }
    std::fs::rename(tmp, path)
}

pub fn write_report(
    dir: &Path,
    binary_name: &str,
    entries: &[TestRecordEntry],
) -> std::io::Result<()> {
    write_report_with(&RealFs, dir, binary_name, entries)
}
