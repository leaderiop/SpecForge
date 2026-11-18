//! `specforge hook`'s exit codes: exit 2 is the hook's own deny, so the binary's refusals are exit 1.

use crate::support::{Cli, TestProject};

#[specforge_test_macros::test(requirement = "FEATURE-013")]
fn a_usage_error_under_hook_exits_1() {
    let dir = TestProject::new();
    let run = Cli::on(&dir).run(&[
        "hook",
        "t.x",
        "--host",
        "claude-code",
        "--event",
        "pre_tool",
        "--bogus",
    ]);
    run.exits(1);
}
