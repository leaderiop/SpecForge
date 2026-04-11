use crate::registry::TestRecordEntry;
use serde::Serialize;
use std::path::Path;

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

pub fn write_report(dir: &Path, binary_name: &str, entries: &[TestRecordEntry]) -> std::io::Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    std::fs::create_dir_all(dir)?;

    let mut sorted = entries.to_vec();
    sorted.sort_by(|a, b| a.entity_id.cmp(&b.entity_id).then(a.test_name.cmp(&b.test_name)));

    let report = BinaryReportRef {
        schema_version: "1.0",
        binary_name,
        entries: &sorted,
    };

    let path = dir.join(format!("{binary_name}.json"));
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(path, json)
}
