use crossbeam_queue::SegQueue;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TestRecordEntry {
    pub entity_kind: String,
    pub entity_id: String,
    pub test_name: String,
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify: Option<String>,
    #[serde(rename = "status")]
    pub outcome: TestOutcome,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TestOutcome {
    Pass,
    Fail,
}

static REGISTRY: SegQueue<TestRecordEntry> = SegQueue::new();

pub fn record(entry: TestRecordEntry) {
    REGISTRY.push(entry);
}

pub fn drain() -> Vec<TestRecordEntry> {
    let mut entries = Vec::new();
    while let Some(entry) = REGISTRY.pop() {
        entries.push(entry);
    }
    entries
}
