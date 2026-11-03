//! The one node probe of the gsd tests: a test that needs node asks it, and says why it skips without it.

use std::process::Command;
use std::sync::OnceLock;

/// Whether `node --test` runs here with the reporters `@javascript/node-test` passes: `spec` to stdout and `junit` to a
/// `--test-reporter-destination` file, which an older node lacks. Probed once, on one passing test in a temp dir: the
/// run must succeed and the file must hold its `<testcase`.
pub fn node_test_runs() -> bool {
    static RUNS: OnceLock<bool> = OnceLock::new();
    *RUNS.get_or_init(|| {
        let Ok(dir) = tempfile::tempdir() else {
            return false;
        };
        let (test, report) = (
            dir.path().join("probe.test.mjs"),
            dir.path().join("probe.xml"),
        );
        std::fs::write(
            &test,
            "import { test } from 'node:test';\ntest('probe', () => {});\n",
        )
        .is_ok()
            && Command::new("node")
                .args([
                    "--test",
                    "--test-reporter=spec",
                    "--test-reporter-destination=stdout",
                ])
                .arg("--test-reporter=junit")
                .arg(format!("--test-reporter-destination={}", report.display()))
                .arg(&test)
                .current_dir(dir.path())
                .output()
                .is_ok_and(|o| o.status.success())
            && std::fs::read_to_string(&report).is_ok_and(|xml| xml.contains("<testcase"))
    })
}
