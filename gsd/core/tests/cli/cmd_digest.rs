//! `record_research`, `record_spec` and `record_validation`: the host records, in the entity's `context_pack`, the pack
//! of `gsd/phase-context` (a quick research: `gsd/quick-context`) with its digest and budget (ADR 0356).

use super::commands_support::*;
use super::harness::{Project, project};

const RESEARCH: &str = "v1.0/02/02-RESEARCH";
const SCORES: &str =
    r#"{"scores":{"goal":90,"boundary":80,"constraint":70,"acceptance":80,"ambiguity":10}}"#;

/// The `context_pack` block of the twin's research, as the fixture holds it (the lines from `context_pack` to `}`).
fn twins_record(p: &Project) -> String {
    let text = p.read("spec/discuss.spec");
    let start = text
        .find("  context_pack ")
        .unwrap_or_else(|| panic!("the twin's research holds no context_pack:\n{text}"));
    let end = start + text[start..].find("}\n").expect("the block's end") + 2;
    text[start..end].to_string()
}

/// The twin's research as if it had never been recorded: no `context_pack` in its text and none in the ledger.
fn strip(p: &Project) {
    let record = twins_record(p);
    p.edit("spec/discuss.spec", &record, "");
    unledger(p, RESEARCH, "context_pack");
}

/// The pack `specforge context` prints of `route` for `subject`.
fn pack(p: &Project, route: &str, subject: &str) -> serde_json::Value {
    let run = p.run(&["context", route, subject]);
    assert_eq!(run.code, 0, "{}", run.all());
    serde_json::from_str(&run.stdout).expect("a pack")
}

#[test]
fn record_research_reproduces_the_twins_pack() {
    let p = twin();
    let (ledger, text) = (p.read("specforge-harness"), p.read("spec/discuss.spec"));
    strip(&p);
    let run = gsd(&p, "record-research", &[RESEARCH]);
    assert_eq!(run.code, 0, "{}", run.all());
    let read = p.read("specforge-harness");
    let recorded: Vec<&str> = read
        .lines()
        .filter(|l| l.starts_with(&format!("{RESEARCH} context_pack ")))
        .collect();
    assert_eq!(
        p.read("spec/discuss.spec"),
        text,
        "the twin's text again; recorded:\n{}\nledger: {recorded:?}",
        twins_record(&p)
    );
    let mut was: Vec<&str> = ledger.lines().collect();
    let mut now: Vec<&str> = read.lines().collect();
    was.sort();
    now.sort();
    assert_eq!(
        was, now,
        "the ledger is the twin's again; recorded: {recorded:?}"
    );
}

#[test]
fn the_recorded_digest_is_what_specforge_context_prints() {
    let p = project("discuss");
    plant(
        &p,
        "spec/discuss.spec",
        "validation 02-VALIDATION \"V\" {\n  phase v1.0/02\n}\n",
    );
    let printed = pack(&p, "gsd/phase-context", "v1.0/02");
    for (command, entity, kind) in [
        ("record-research", "v1.0/02/02-RESEARCH", "research"),
        ("record-spec", "v1.0/02/02-SPEC", "phase_spec"),
        ("record-validation", "v1.0/02/02-VALIDATION", "validation"),
    ] {
        let run = gsd(&p, command, &[entity, "--format", "json"]);
        assert_eq!(run.code, 0, "{kind}: {}", run.all());
        assert_eq!(json(&run)["output"]["route"], "gsd/phase-context", "{kind}");
        assert_eq!(json(&run)["output"]["recorded"], entity, "{kind}");
        let record = &p.fields_of(entity)["context_pack"];
        assert_eq!(record["route"], "gsd/phase-context", "{kind}: {record}");
        assert_eq!(record["digest"], printed["digest"], "{kind}: {record}");
        assert_eq!(record["max_tokens"], 50000, "{kind}: {record}");
    }
    assert!(
        !p.codes().contains(&"gsd/W406".to_string()),
        "{:?}",
        p.codes()
    );
}

#[test]
fn record_spec_leaves_its_e202_verdict_fresh() {
    let p = project("discuss");
    let spec = "v1.0/02/02-SPEC";
    let run = p.record(spec, "E202", "passed", &["--values", SCORES]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.inputs(spec, "E202")["verdict"]["state"], "fresh");
    let run = gsd(&p, "record-spec", &[spec]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.inputs(spec, "E202")["verdict"]["state"], "fresh");
}

#[test]
fn record_research_leaves_the_phases_plan_checks_fresh() {
    let p = twin();
    strip(&p);
    let plan = "v1.0/02/02-01";
    let run = p.record(plan, "W311", "passed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.inputs(plan, "W311")["verdict"]["state"], "fresh");
    let run = gsd(&p, "record-research", &[RESEARCH]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.inputs(plan, "W311")["verdict"]["state"], "fresh");
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
fn a_phase_with_no_context_records_its_pack() {
    let p = project("verify");
    let validation = "v1.0/01/01-VALIDATION";
    let run = gsd(&p, "record-validation", &[validation]);
    assert_eq!(run.code, 0, "{}", run.all());
    let record = &p.fields_of(validation)["context_pack"];
    assert_eq!(record["route"], "gsd/phase-context", "{record}");
    assert_eq!(
        record["digest"],
        pack(&p, "gsd/phase-context", "v1.0/01")["digest"]
    );
    assert!(
        !p.codes().contains(&"gsd/W406".to_string()),
        "{:?}",
        p.codes()
    );
}
