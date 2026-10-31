//! D-K20b-12 (ADR 0318): a command is told the host that calls it.

use serde_json::json;
use specforge_ops::host::Host;
use specforge_test_macros::test as specforge_test;

use crate::gates::{project, run_command_on, served};

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "a command is told the host that calls it, and nothing when none does"
)]
fn a_command_is_told_its_host() {
    for (host, wire) in [(Some(Host::Codex), Some("codex")), (None, None)] {
        let served = served(project("g", "s", json!({})));
        run_command_on(&served, "record_plan", json!({"phase": "03"}), host).expect("it runs");
        let input = served.input("cmd__record_plan");
        assert_eq!(input.get("host").and_then(|h| h.as_str()), wire, "{input}");
    }
}
