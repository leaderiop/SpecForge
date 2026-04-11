use crate::atexit;
use crate::registry::{self, TestOutcome, TestRecordEntry};

pub struct TestGuard {
    entity_kind: &'static str,
    entity_id: &'static str,
    test_name: &'static str,
    file: &'static str,
    verify: Option<&'static str>,
}

impl TestGuard {
    pub fn new(
        entity_kind: &'static str,
        entity_id: &'static str,
        _module_path: &'static str,
        test_name: &'static str,
        file: &'static str,
        _line: u32,
        verify: Option<&'static str>,
    ) -> Self {
        atexit::ensure_registered();
        Self {
            entity_kind,
            entity_id,
            test_name,
            file,
            verify,
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
            entity_kind: self.entity_kind.to_string(),
            entity_id: self.entity_id.to_string(),
            test_name: self.test_name.to_string(),
            file: self.file.to_string(),
            verify: self.verify.map(|s| s.to_string()),
            outcome,
        });
    }
}
