//! `gsd/W506` end to end (D-r42-17): a verified phase whose context decision no summary or commit mentions.

use serde_json::json;

use super::cmd_milestone::ready;
use super::harness::Project;

const DECISION: &str = "\ndecision D-09 \"Keep the store in memory\" {\n  phase  v1.0/01\n  area   \"Storage\"\n  text   \"Tasks are kept in one in-memory list\"\n  origin context\n}\n";

fn summary(one_liner: &str) -> String {
    format!(
        "\nsummary 01-01-SUMMARY \"S\" {{\n  plan                   v1.0/01/01-01\n  status                 complete\n  one_liner              \"{one_liner}\"\n  subsystem              \"store\"\n  requirements_completed []\n}}\n"
    )
}

/// A verified one-phase project with a context decision in the phase and a summary that says `one_liner`.
fn verified(one_liner: &str) -> Project {
    let p = ready();
    p.write("spec/decision.spec", DECISION);
    p.write("spec/summary.spec", &summary(one_liner));
    p
}

fn w506(p: &Project) -> Vec<String> {
    p.report_all()
        .into_iter()
        .filter(|(code, _)| code == "gsd/W506")
        .map(|(_, message)| message)
        .collect()
}

#[test]
fn w506_fires_on_a_verified_phase_whose_decision_nothing_shipped() {
    let p = verified("An in-memory store with add and list");
    let found = w506(&p);
    assert_eq!(found.len(), 1, "{found:?} {:?}", p.report_all());
    assert!(found[0].contains("v1.0/01/D-09"), "{found:?}");
}

#[test]
fn w506_is_silent_when_a_summary_names_the_decision() {
    let p = verified("An in-memory store (D-09)");
    assert!(w506(&p).is_empty(), "{:?}", w506(&p));
}

#[test]
fn w506_is_off_with_the_context_coverage_gate_off() {
    let p = verified("An in-memory store with add and list");
    assert_eq!(w506(&p).len(), 1);
    p.settings(json!({ "workflow.context_coverage_gate": false }));
    assert!(w506(&p).is_empty(), "{:?}", w506(&p));
}

#[test]
fn the_twin_reports_no_w506() {
    let p = super::commands_support::twin();
    assert!(w506(&p).is_empty(), "{:?}", w506(&p));
}
