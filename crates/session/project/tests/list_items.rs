//! K13 (ADR 0294): a pass sees a list field as its items. Pins for the ticket that delivers it.

use std::sync::Arc;

use serde_json::json;
use specforge_project::testing::{gsd_facts_project, gsd_facts_runtime, pass_input};

#[test]
fn a_list_item_holding_the_joiner_reaches_a_pass_whole() {
    let project = gsd_facts_project();
    let runtime = Arc::new(gsd_facts_runtime());
    let _compiled = project.compile_in(runtime.clone());

    let input = pass_input(&runtime, "lists");
    let phase = input["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "p1")
        .unwrap();
    assert_eq!(phase["values"]["criteria"], json!(["ships, fast", "works"]));
    // The joined text stays for a 1.10 guest.
    assert_eq!(phase["fields"]["criteria"], "ships, fast, works");
}
