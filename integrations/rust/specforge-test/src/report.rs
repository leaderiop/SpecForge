use crate::registry::TestRecordEntry;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct BinaryReport {
    pub binary_name: String,
    pub entries: Vec<TestRecordEntry>,
}

pub fn write_report(dir: &Path, binary_name: &str, entries: Vec<TestRecordEntry>) -> std::io::Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    std::fs::create_dir_all(dir)?;

    let report = BinaryReport {
        binary_name: binary_name.to_string(),
        entries,
    };

    let path = dir.join(format!("{binary_name}.json"));
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(path, json)
}
