//! `gsd/W506` end to end: a verified phase whose context decision no complete plan lists.

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
fn w506_is_silent_when_a_complete_plan_lists_the_decision() {
    // A summary that names the decision is no claim.
    let p = verified("An in-memory store (D-09)");
    assert_eq!(w506(&p).len(), 1, "{:?}", p.report_all());
    p.edit(
        "spec/m.spec",
        "  type execute\n  objective \"Do it\"\n",
        "  type execute\n  decisions [D-09]\n  objective \"Do it\"\n",
    );
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

/// A context decision another decision supersedes is not trackable, under every profile: neither E310, W317 nor W506
/// names it, while the decision that supersedes it is listed by the complete plan.
#[test]
fn a_superseded_decision_is_neither_e310_nor_w506_under_any_profile() {
    let p = verified("An in-memory store with add and list");
    p.write(
        "spec/decision.spec",
        &format!(
            "{DECISION}\ndecision D-10 \"Keep the store in a file\" {{\n  phase      v1.0/01\n  area       \"Storage\"\n  text       \"Tasks are kept in one JSON file\"\n  origin     context\n  supersedes [D-09]\n}}\n"
        ),
    );
    p.edit(
        "spec/m.spec",
        "  type execute\n  objective \"Do it\"\n",
        "  type execute\n  decisions [D-10]\n  objective \"Do it\"\n",
    );
    for profile in [None, Some("gsd-strict"), Some("gsd-plus")] {
        if let Some(profile) = profile {
            p.settings(json!({ "profile": profile }));
        }
        let report = p.report_all();
        assert!(
            !report.iter().any(|(code, message)| {
                ["gsd/E310", "gsd/W317", "gsd/W506"].contains(&code.as_str())
                    && message.contains("D-09")
            }),
            "{profile:?}: {report:#?}"
        );
        assert!(
            report
                .iter()
                .all(|(code, _)| code != "gsd/E310" && code != "gsd/W506"),
            "{profile:?}: {report:#?}"
        );
    }
}
