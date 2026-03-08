use crate::atexit;
use crate::registry::{self, TestOutcome, TestRecordEntry};

pub struct TestGuard {
    entity_kind: String,
    entity_id: String,
    test_name: String,
    file: String,
    verify: Option<String>,
}

impl TestGuard {
    pub fn new(
        entity_kind: &str,
        entity_id: &str,
        _module_path: &str,
        test_name: &str,
        file: &str,
        _line: u32,
        verify: Option<&str>,
    ) -> Self {
        atexit::ensure_registered();
        Self {
            entity_kind: entity_kind.to_string(),
            entity_id: entity_id.to_string(),
            test_name: test_name.to_string(),
            file: file.to_string(),
            verify: verify.map(|s| s.to_string()),
        }
    }
}

impl Drop for TestGuard {
    fn drop(&mut self) {
        let outcome = if std::thread::panicking() {
            TestOutcome::Fail
        } else {
            TestOutcome::Pass
        };

        registry::record(TestRecordEntry {
            entity_kind: std::mem::take(&mut self.entity_kind),
            entity_id: std::mem::take(&mut self.entity_id),
            test_name: std::mem::take(&mut self.test_name),
            file: std::mem::take(&mut self.file),
            verify: std::mem::take(&mut self.verify),
            outcome,
        });
    }
}
