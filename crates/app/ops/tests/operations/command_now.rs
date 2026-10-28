//! K14 (ADR 0294): a command input carries `now`, RFC 3339 UTC. Pin for the ticket that delivers it; ignored until
//! then.

use serde_json::json;
use specforge_project::testing::CORE;

use crate::plans::{compiled, project, run_command, serving};

#[test]
#[ignore = "pins K14; T9 un-ignores"]
fn a_command_input_carries_now() {
    let project = project();
    let runtime = serving(&[(CORE, "cmd__quiet", 0, json!([]))]);
    let compiled = compiled(&project, &runtime);
    let ran = run_command(&project, &compiled, &runtime, "core", "quiet");

    let input = ran.input("cmd__quiet");
    let now = input["now"]
        .as_str()
        .unwrap_or_else(|| panic!("no now in {input}"));
    assert!(
        chrono::DateTime::parse_from_rfc3339(now).is_ok() && now.ends_with('Z') && now.len() == 20,
        "{now}"
    );
    assert_eq!(&now[..10], input["today"].as_str().unwrap());
}
