//! `record_research`, `record_spec` and `record_validation`: the digest of the phase's context, read from the graph
//! the command receives (probe P7, ADR 0299), recorded as `context_digest`.

use super::commands_support::*;
use super::harness::project;

const RESEARCH: &str = "v1.0/02/02-RESEARCH";
const DIGEST: &str = "\"fnv1a64:4312322ac67d01d2\"";
const SCORES: &str =
    r#"{"scores":{"goal":90,"boundary":80,"constraint":70,"acceptance":80,"ambiguity":10}}"#;

/// The twin's research as if it had never been recorded: no `context_digest` in its text and none in the ledger.
fn strip(p: &super::harness::Project) {
    p.edit(
        "spec/discuss.spec",
        "  context_digest     \"fnv1a64:4312322ac67d01d2\"\n",
        "",
    );
    unledger(p, RESEARCH, "context_digest");
}

#[test]
fn record_research_reproduces_the_twins_digest() {
    let p = twin();
    let (ledger, text) = (p.read("specforge-harness"), p.read("spec/discuss.spec"));
    strip(&p);
    assert!(!ledgered(&p, RESEARCH, "context_digest", DIGEST));
    let run = gsd(&p, "record-research", &[RESEARCH]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(
        ledgered(&p, RESEARCH, "context_digest", DIGEST),
        "{}",
        p.read("specforge-harness")
    );
    assert_eq!(p.read("spec/discuss.spec"), text, "the twin's text again");
    let mut was: Vec<&str> = ledger.lines().collect();
    let read = p.read("specforge-harness");
    let mut now: Vec<&str> = read.lines().collect();
    was.sort();
    now.sort();
    assert_eq!(was, now, "the ledger is the twin's again");
}

#[test]
fn recorded_digest_equals_the_pass_digest() {
    let p = project("discuss");
    plant(
        &p,
        "spec/discuss.spec",
        "validation 02-VALIDATION \"V\" {\n  phase v1.0/02\n}\n",
    );
    for (command, entity, kind) in [
        ("record-research", "v1.0/02/02-RESEARCH", "research"),
        ("record-spec", "v1.0/02/02-SPEC", "phase_spec"),
        ("record-validation", "v1.0/02/02-VALIDATION", "validation"),
    ] {
        let run = gsd(&p, command, &[entity]);
        assert_eq!(run.code, 0, "{kind}: {}", run.all());
        assert_eq!(
            p.fields_of(entity)["context_digest"],
            p.fields_of("v1.0/02/02-CONTEXT")["digest"],
            "{kind}"
        );
    }
    assert!(
        !p.codes().contains(&"gsd/W406".to_string()),
        "{:?}",
        p.codes()
    );
}

#[test]
fn record_spec_stales_its_e202_verdict() {
    let p = project("discuss");
    let spec = "v1.0/02/02-SPEC";
    let run = p.record(spec, "E202", "passed", &["--values", SCORES]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.inputs(spec, "E202")["verdict"]["state"], "fresh");
    let run = gsd(&p, "record-spec", &[spec]);
    assert_eq!(run.code, 0, "the write is not refused: {}", run.all());
    assert_eq!(p.inputs(spec, "E202")["verdict"]["state"], "stale");
}

#[test]
fn record_research_stales_the_phases_plan_checks() {
    let p = twin();
    strip(&p);
    let plan = "v1.0/02/02-01";
    let run = p.record(plan, "W311", "passed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.inputs(plan, "W311")["verdict"]["state"], "fresh");
    let run = gsd(&p, "record-research", &[RESEARCH]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.inputs(plan, "W311")["verdict"]["state"], "stale");
}

#[test]
fn a_project_research_is_refused() {
    let p = project("discuss");
    let before = bytes(&p);
    let run = gsd(&p, "record-research", &["RESEARCH"]);
    refused(
        &run,
        "invalid_input",
        &["a project research has no context"],
    );
    unchanged(&p, &before);
}

#[test]
fn no_context_is_refused() {
    let p = project("verify");
    let before = bytes(&p);
    let run = gsd(&p, "record-validation", &["v1.0/01/01-VALIDATION"]);
    refused(
        &run,
        "invalid_input",
        &["the phase has no context", "/gsd-discuss-phase"],
    );
    unchanged(&p, &before);
}
