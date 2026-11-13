//! The context a role reads for one entity (ADR 0356): a budgeted read of a plan for its executor.

use crate::support::*;
use serde_json::Value;
use specforge_project::testing::{gsd_context_project, gsd_routed_runtime};
use specforge_test::prelude::*;

fn ids(nodes: &Value) -> Vec<&str> {
    nodes
        .as_array()
        .unwrap_or_else(|| panic!("no nodes in {nodes}"))
        .iter()
        .filter_map(|node| node["id"].as_str())
        .collect()
}

// Flips once the executor's pack serves the read: the read then keeps every task and the phase's context.
#[specforge_test(
    behavior = "serve_context_pack",
    verify = "a budgeted context read of a plan for its executor keeps every core member and cuts only enrich sections"
)]
fn a_1500_token_read_for_a_plans_executor_keeps_its_tasks_and_context() {
    let mut server = Served::initialized(gsd_context_project().serving(gsd_routed_runtime()));
    let (_, read) = resource(
        &mut server,
        "specforge://context/v1.0/02/02-01?max_tokens=1500",
    );
    let truncated = &read["token_budget"]["truncated_entities"];
    assert!(
        truncated
            .as_array()
            .is_some_and(|cut| cut.iter().any(|id| id == "v1.0/02/02-01/T1")),
        "{read}"
    );
    let nodes = ids(&read["nodes"]);
    assert!(!nodes.contains(&"v1.0/02/02-CONTEXT"), "{read}");
    assert!(nodes.contains(&"v1.0/01"), "{read}");
}
