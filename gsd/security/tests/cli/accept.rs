//! `specforge gsd accept-risk`: a person's record, run at a terminal.

use super::harness::{self, Project};

fn recording() -> Project {
    harness::project("recording").with_extension("@gsd/security", "GSD_SECURITY_WASM")
}

fn accept(p: &Project, threat: &str, reason: &str, by: &str) -> harness::Run {
    p.run_as_person(&["gsd", "accept-risk", threat, reason, by, "--format", "json"])
}

#[test]
fn a_person_accepts_a_threat_and_it_stops_blocking() {
    let p = recording();
    assert_eq!(p.fields_of("v1.0/01/01-01/T-01-01")["blocking"], true);
    let run = accept(&p, "01-01/T-01-01", "Only reachable by the admin", "alice");
    assert_eq!(run.code, 0, "{}", run.all());
    // the audit is created when absent, and the risk is under it
    let risk = p.fields_of("v1.0/01/01-SECURITY/AR1");
    assert_eq!(risk["accepted_by"], "alice", "{risk}");
    assert_eq!(risk["rationale"], "Only reachable by the admin");
    assert!(risk["at"].as_str().is_some_and(|d| d.len() == 10), "{risk}");
    assert_eq!(p.fields_of("v1.0/01/01-SECURITY")["asvs_level"], 1);
    assert_eq!(p.fields_of("v1.0/01/01-01/T-01-01")["blocking"], false);
    let bad: Vec<_> = p
        .report()
        .into_iter()
        .filter(|(c, _)| c.starts_with('E'))
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn an_agent_call_is_refused_human_only_and_writes_nothing() {
    let p = recording();
    let before = p.read("spec/security.spec");
    let run = p.run(&[
        "gsd",
        "accept-risk",
        "01-01/T-01-01",
        "because",
        "alice",
        "--format",
        "json",
    ]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("human_only"), "{}", run.all());
    assert_eq!(p.read("spec/security.spec"), before);
}

#[test]
fn an_empty_rationale_or_name_is_refused() {
    let p = recording();
    for (reason, by) in [
        ("", "alice"),
        ("   ", "alice"),
        ("because", ""),
        ("because", "  "),
    ] {
        let run = accept(&p, "01-01/T-01-01", reason, by);
        assert_ne!(run.code, 0, "{reason:?} {by:?}: {}", run.all());
        assert!(run.all().contains("invalid_input"), "{}", run.all());
    }
    assert_eq!(
        p.run(&["query", "v1.0/01/01-SECURITY", "--depth", "0"])
            .code,
        1,
        "nothing was written"
    );
}

#[test]
fn accepting_twice_is_refused() {
    let p = recording();
    assert_eq!(accept(&p, "01-01/T-01-01", "because", "alice").code, 0);
    let again = accept(&p, "01-01/T-01-01", "again", "bob");
    assert_ne!(again.code, 0, "{}", again.all());
    assert!(again.all().contains("already accepted"), "{}", again.all());
    // a second threat gets the next number
    let other = accept(&p, "01-01/T-01-02", "fine", "bob");
    assert_eq!(other.code, 0, "{}", other.all());
    assert_eq!(p.fields_of("v1.0/01/01-SECURITY/AR2")["accepted_by"], "bob");
}
