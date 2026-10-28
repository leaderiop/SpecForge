//! What `@gsd/core` declares for the plan kinds (`spec/plan.md`), pinned through the SDK's runtime-free `MockHost`.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{describe, field_of, fields_of};

fn kind_of(kind: &str) -> serde_json::Value {
    describe("entities")["items"]
        .as_array()
        .and_then(|kinds| kinds.iter().find(|k| k["name"] == kind))
        .unwrap_or_else(|| panic!("no kind {kind}"))
        .clone()
}

#[test]
fn the_plan_kind_declares_the_fields_of_the_model() {
    assert_eq!(
        fields_of("plan"),
        [
            "phase",
            "type",
            "depends_on",
            "files_modified",
            "files_deleted",
            "coupling_justified",
            "requirements",
            "wave0",
            "agent_hint",
            "estimate",
            "objective",
            "purpose",
            "output",
            "context_refs",
            "verification",
            "success_criteria",
            "wave",
            "autonomous",
            "status"
        ]
    );
    for required in ["type", "files_modified", "objective"] {
        assert_eq!(field_of("plan", required)["required"], true, "{required}");
    }
    for computed in ["wave", "autonomous", "status"] {
        assert_eq!(
            field_of("plan", computed)["writer"],
            "computed",
            "{computed}"
        );
    }
    // Paths a plan creates are text, never `file_reference` (the host's E016).
    for paths in ["files_modified", "files_deleted", "context_refs"] {
        assert_ne!(field_of("plan", paths)["file_reference"], true, "{paths}");
    }
}

#[test]
fn the_plan_kind_is_scoped_by_its_phase_which_is_optional() {
    assert_eq!(kind_of("plan")["scoped_by"], "phase");
    let phase = field_of("plan", "phase");
    assert_eq!(phase["immutable"], true);
    assert_ne!(phase["required"], true);
}

#[test]
fn a_coupling_peer_is_a_reference_member() {
    let coupling = field_of("plan", "coupling_justified");
    assert_eq!(coupling["field_type"], "block_list");
    let peer = coupling["members"]
        .as_array()
        .and_then(|m| m.iter().find(|m| m["name"] == "peer"))
        .expect("peer");
    assert_eq!(peer["target_kind"], "plan");
    assert_eq!(peer["required"], true);
    let edges = describe("edges");
    assert!(
        edges["items"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|e| e["label"] == "PlanCouplingJustifiedPeer" && e["source_kind"] == "plan"),
        "{edges}"
    );
}
