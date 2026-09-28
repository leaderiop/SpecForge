use crate::atexit;
use crate::registry::{self, TestOutcome, TestRecordEntry};
use std::time::Instant;

pub struct TestGuard {
    entity_kind: &'static str,
    entity_id: &'static str,
    test_name: &'static str,
    file: &'static str,
    verify: Option<&'static str>,
    /// `#[should_panic]` test: a panic is the success path (C11-04).
    expect_panic: bool,
    started: Instant,
    /// `#[ignore]`d test: Skipped is recorded on construction and Drop is a
    /// no-op (C11-04).
    skip: bool,
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
        Self::with_expectations(
            entity_kind,
            entity_id,
            _module_path,
            test_name,
            file,
            _line,
            verify,
            false,
        )
    }

    /// Macro entry point carrying `#[should_panic]` / `#[ignore]`
    /// expectations (C11-04).
    #[allow(clippy::too_many_arguments)]
    pub fn with_expectations(
        entity_kind: &'static str,
        entity_id: &'static str,
        _module_path: &'static str,
        test_name: &'static str,
        file: &'static str,
        _line: u32,
        verify: Option<&'static str>,
        expect_panic: bool,
    ) -> Self {
        atexit::ensure_registered();
        Self {
            entity_kind,
            entity_id,
            test_name,
            file,
            verify,
            expect_panic,
            started: Instant::now(),
            skip: false,
        }
    }

    /// `#[ignore]` entry point: records [`TestOutcome::Skipped`] immediately
    /// (the caller then skips the body), and Drop records nothing.
    pub fn new_skipped(
        entity_kind: &'static str,
        entity_id: &'static str,
        _module_path: &'static str,
        test_name: &'static str,
        file: &'static str,
        _line: u32,
        verify: Option<&'static str>,
    ) -> Self {
        atexit::ensure_registered();
        registry::record(TestRecordEntry {
            entity_kind: entity_kind.to_string(),
            entity_id: entity_id.to_string(),
            test_name: test_name.to_string(),
            file: file.to_string(),
            verify: verify.map(|s| s.to_string()),
            verify_kind: None,
            duration_ms: 0,
            outcome: TestOutcome::Skipped,
        });
        Self {
            entity_kind,
            entity_id,
            test_name,
            file,
            verify,
            expect_panic: false,
            started: Instant::now(),
            skip: true,
        }
    }
}

impl Drop for TestGuard {
    fn drop(&mut self) {
        if self.skip {
            return;
        }
        let outcome = if self.expect_panic {
            // C11-04: a panic under #[should_panic] is the success path —
            // recording Fail made every should_panic test count against
            // coverage.
            if std::thread::panicking() {
                TestOutcome::Pass
            } else {
                // should_panic tests that never panicked are harness
                // failures; the harness itself catches that case, but the
                // guard still ran, so record Fail.
                TestOutcome::Fail
            }
        } else if std::thread::panicking() {
            TestOutcome::Fail
        } else {
            TestOutcome::Pass
        };

        let entry = TestRecordEntry {
            entity_kind: self.entity_kind.to_string(),
            entity_id: self.entity_id.to_string(),
            test_name: self.test_name.to_string(),
            file: self.file.to_string(),
            verify: self.verify.map(|s| s.to_string()),
            verify_kind: None, // stamped at finalize from the exported graph (C11-07)
            duration_ms: u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX),
            outcome,
        };
        // Durability first (C11-06): the JSONL line persists even if the
        // process dies before atexit runs.
        atexit::append_jsonl(&entry);
        registry::record(entry);
    }
}
