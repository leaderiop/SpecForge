use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TestRecordEntry {
    pub entity_kind: String,
    pub entity_id: String,
    pub test_name: String,
    pub file: String,
    #[serde(rename = "status")]
    pub outcome: TestOutcome,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TestOutcome {
    Pass,
    Fail,
}

static REGISTRY: Mutex<Vec<TestRecordEntry>> = Mutex::new(Vec::new());

pub fn record(entry: TestRecordEntry) {
    REGISTRY.lock().unwrap().push(entry);
}

pub fn drain() -> Vec<TestRecordEntry> {
    let mut lock = REGISTRY.lock().unwrap();
    std::mem::take(&mut *lock)
}
