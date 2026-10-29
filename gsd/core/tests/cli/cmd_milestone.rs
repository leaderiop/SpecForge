//! `close_milestone`: gated by its audit, it writes the completion and the retrospective (D-r41-7).

use super::commands_support::*;
use super::harness::{Project, project};

const DEBT_ACCEPTED: &str = "  tech_debt [{ phase v1.0/01 item \"debt\" }]\n  debt_accepted {\n    by \"ada\"\n    at 2026-10-29\n  }\n";

#[test]
fn close_milestone_on_the_twin_is_refused_e106_and_e511() {
    let p = twin();
    let before = bytes(&p);
    let run = gsd(&p, "close-milestone", &["v1.0"]);
    assert_ne!(run.code, 0, "{}", run.all());
    let all = run.all();
    for word in ["E106", "E511", "v1.0/01", "v1.0/02", "v1.0/03", "no audit"] {
        assert!(all.contains(word), "'{word}' not in: {all}");
    }
    unchanged(&p, &before);
}

/// A one-phase milestone whose phase is verified (nothing to verify) and whose audit passes with tech debt a person
/// accepted: nothing for the gates to refuse.
pub(super) fn ready() -> Project {
    let p = project("empty");
    p.write(
        "spec/m.spec",
        "milestone v1.0 \"M\" {\n  name \"M\"\n}\n\nphase 01 \"Core\" {\n  milestone v1.0\n  name \"Core\"\n  slug \"core\"\n  goal \"Do it\"\n  success_criteria [\"It is done\"]\n}\n\nplan 01-01 \"P\" {\n  phase v1.0/01\n  type execute\n  objective \"Do it\"\n}\n\ntruth TR1 \"T\" {\n  plan v1.0/01/01-01\n  text \"It is done\"\n}\n\nverification 01-VERIFICATION \"V\" {\n  phase v1.0/01\n}\n\nmilestone_audit A1 \"A\" {\n  milestone v1.0\n}\n",
    );
    assert_eq!(p.verify_phase("v1.0/01"), 1);
    p
}

#[test]
fn close_milestone_writes_completion_and_the_retrospective() {
    let p = ready();
    let run = gsd(
        &p,
        "close-milestone",
        &["v1.0", "--built", r#"["a store","a cli"]"#],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    let fields = p.fields_of("v1.0");
    assert_eq!(fields["completion"]["audit"], "A1");
    assert_eq!(fields["state"], "shipped");
    let retro = p.fields_of("v1.0/v1.0-RETROSPECTIVE");
    assert_eq!(retro["built"], serde_json::json!(["a store", "a cli"]));
    assert_eq!(retro["milestone"], "v1.0");
}

#[test]
fn a_tech_debt_audit_without_a_persons_acceptance_is_refused() {
    let p = ready();
    p.edit(
        "spec/m.spec",
        "milestone_audit A1 \"A\" {\n  milestone v1.0\n",
        "milestone_audit A1 \"A\" {\n  milestone v1.0\n  tech_debt [{ phase v1.0/01 item \"debt\" }]\n",
    );
    let before = bytes(&p);
    let run = gsd(&p, "close-milestone", &["v1.0"]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E106"), "{}", run.all());
    unchanged(&p, &before);
    p.edit(
        "spec/m.spec",
        "  tech_debt [{ phase v1.0/01 item \"debt\" }]\n",
        DEBT_ACCEPTED,
    );
    let run = gsd(&p, "close-milestone", &["v1.0"]);
    assert_eq!(run.code, 0, "{}", run.all());
}

#[test]
fn a_closed_milestone_is_refused() {
    let p = ready();
    assert_eq!(gsd(&p, "close-milestone", &["v1.0"]).code, 0);
    let before = bytes(&p);
    let run = gsd(&p, "close-milestone", &["v1.0"]);
    refused(&run, "invalid_input", &["already closed"]);
    unchanged(&p, &before);
}

#[test]
fn an_existing_retrospective_is_kept() {
    let p = ready();
    p.write(
        "spec/retro.spec",
        "retrospective v1.0-RETROSPECTIVE \"Retro\" {\n  milestone v1.0\n  built [\"mine\"]\n}\n",
    );
    let run = gsd(&p, "close-milestone", &["v1.0", "--built", r#"["theirs"]"#]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(
        p.fields_of("v1.0/v1.0-RETROSPECTIVE")["built"],
        serde_json::json!(["mine"])
    );
}
