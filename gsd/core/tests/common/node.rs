//! The one node probe of the gsd tests: a test that needs node asks it, and says why it skips without it.

use std::process::Command;

/// Whether `node` runs here.
pub fn node_runs() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}
