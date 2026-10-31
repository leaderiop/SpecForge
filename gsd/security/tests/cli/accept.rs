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

/// REVIEW-security-part2 M1: U4 holds. `at` is a harness field, so an agent cannot write a person's acceptance.
fn agent_accepts(p: &Project, extra: &[&str]) -> harness::Run {
    let mut args = vec![
        "new",
        "accepted_risk",
        "AR2",
        "agent accepts",
        "--audit",
        "v1.0/01/01-SECURITY",
        "--threat",
        "v1.0/01/01-01/T-01-01",
        "--rationale",
        "agent says fine",
        "--accepted_by",
        "alice",
        "--in",
        "spec/security.spec",
    ];
    args.extend_from_slice(extra);
    p.run(&args)
}

fn gate(p: &Project) -> serde_json::Value {
    let run = p.run(&["gsd", "security-gate", "01", "--format", "json"]);
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("JSON ({e}): {}", run.all()))
}

#[test]
fn an_agent_cannot_record_an_accepted_risk() {
    let p = recording();
    let made = p.run(&[
        "new",
        "security_audit",
        "01-SECURITY",
        "Phase 01 security",
        "--phase",
        "v1.0/01",
        "--in",
        "spec/security.spec",
    ]);
    assert_eq!(made.code, 0, "{}", made.all());
    let before = p.read("spec/security.spec");
    // with `--at`: refused, nothing written
    let run = agent_accepts(&p, &["--at", "2026-10-31"]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("harness_field"), "{}", run.all());
    assert_eq!(p.read("spec/security.spec"), before);
    // without `--at`: the entity exists, but is no acceptance, and the gate still blocks
    let run = agent_accepts(&p, &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of("v1.0/01/01-01/T-01-01")["blocking"], true);
    assert_eq!(gate(&p)["block"], true);
}

#[test]
fn a_person_s_acceptance_clears_the_gate() {
    let p = recording();
    assert_eq!(gate(&p)["block"], true);
    assert_eq!(accept(&p, "01-01/T-01-01", "fine", "alice").code, 0);
    assert_eq!(
        p.fields_of("v1.0/01/01-SECURITY/AR1")["at"]
            .as_str()
            .map(str::len),
        Some(10)
    );
    assert_eq!(p.fields_of("v1.0/01/01-01/T-01-01")["blocking"], false);
}
