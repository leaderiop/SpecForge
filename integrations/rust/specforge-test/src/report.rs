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

pub fn write_report(
    dir: &Path,
    binary_name: &str,
    entries: &[TestRecordEntry],
) -> std::io::Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    std::fs::create_dir_all(dir)?;

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
        serde_json::to_writer(&mut writer, &report)?;
        writer.flush()?;
    }
    std::fs::rename(tmp, path)
}
