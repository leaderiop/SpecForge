//! Verdict freshness at the ship gate on the taskboard twin under gsd-strict (ADR 0379): of the judgements the ship
//! gate of phase 01 reads, the share that is fresh, with `gsd/E516` refusing exactly the others. The value is computed
//! from what `specforge` prints, never typed; each test asserts the judgements it is computed over.

use std::collections::BTreeSet;

use serde_json::json;

use super::harness::{Freshness, Project, Verdict, taskboard};

const PHASE: &str = "v1.0/01";
const E501: &str = "gsd/E501";
/// Phase 01's must-haves, each carrying a fresh verifier verdict in the committed twin.
const MUST_HAVES: [&str; 7] = [
    "v1.0/01/01-01/AR1",
    "v1.0/01/01-01/KL1",
    "v1.0/01/01-01/TR1",
    "v1.0/01/01-02/AR1",
    "v1.0/01/01-02/KL1",
    "v1.0/01/01-02/TR1",
    "v1.0/01/01-02/TR2",
];

fn verifier(entities: &[&str]) -> BTreeSet<Verdict> {
    entities
        .iter()
        .map(|e| (E501.to_string(), e.to_string()))
        .collect()
}

/// The taskboard twin as committed, under gsd-strict.
fn strict_twin() -> Project {
    let p = taskboard();
    p.settings(json!({ "profile": "gsd-strict" }));
    p
}

/// What holds of every measurement: the gate reads the phase's seven must-haves, `gsd/E516` refuses exactly the ones
/// that are not fresh (ship runs no judge, so none is excused), and its gate passes exactly when every one is fresh.
fn measured(p: &Project) -> Freshness {
    let f = p.ship_freshness(PHASE);
    assert_eq!(f.required, verifier(&MUST_HAVES), "{f:#?}");
    assert_eq!(f.refused_set(), f.not_fresh(), "{f:#?}");
    assert_eq!(f.gate_passes, f.fresh == f.required, "{f:#?}");
    f
}

/// Records the verifier's verdict on `entity` again, over its inputs now.
fn verify(p: &Project, entity: &str) {
    let citing = p.citing(entity);
    let extra: Vec<&str> = citing.iter().map(String::as_str).collect();
    let run = p.record(entity, "E501", "verified", &extra);
    assert_eq!(run.code, 0, "{entity}: {}", run.all());
}

#[test]
fn every_verdict_the_ship_gate_reads_is_fresh_under_strict() {
    let p = strict_twin();
    let f = measured(&p);
    // The judgements the gate reads are the verifier's on the phase, as re-verification lists them.
    let listed: BTreeSet<Verdict> = p.re_verify(PHASE, false)["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| {
            (
                i["rule"].as_str().expect("a rule").to_string(),
                i["entity"].as_str().expect("an entity").to_string(),
            )
        })
        .collect();
    assert_eq!(f.required, listed);
    let line = f.line("taskboard");
    eprintln!("{line}");
    // The line ADR 0379 quotes.
    assert_eq!(
        line,
        "verdict freshness at ship on the taskboard twin: 7 of 7 fresh (100%)"
    );
    assert_eq!(f.ratio(), 1.0, "{f:#?}");
    assert!(f.refused.is_empty() && f.gate_passes, "{f:#?}");
}

/// The control of a gate switched off: `specforge.verdict_gate: false` beside the profile beats the preset, and a
/// gate that refuses nothing cannot be measured, so the measure panics instead of printing 100%.
#[test]
#[should_panic(expected = "the verdict gate is off")]
fn the_measure_refuses_a_verdict_gate_switched_off_beside_strict() {
    let p = taskboard();
    p.settings(json!({ "profile": "gsd-strict", "specforge.verdict_gate": false }));
    p.ship_freshness(PHASE);
}

/// An edit to `src/tasks.js` stales the six verdicts that read it (all but 01-02's artifact, which names
/// `src/cli.js`): freshness drops and ship is refused under gsd/E516 for each; one recorded again is not enough, all
/// six are.
#[test]
fn a_stale_verdict_lowers_freshness_and_refuses_ship_until_it_is_recorded_again() {
    let p = strict_twin();
    p.write(
        "src/tasks.js",
        &format!("{}// edited\n", p.read("src/tasks.js")),
    );
    let stale = measured(&p);
    eprintln!("control, src/tasks.js edited: {}", stale.line("taskboard"));
    let mut reading = MUST_HAVES.to_vec();
    reading.retain(|e| *e != "v1.0/01/01-02/AR1");
    assert_eq!(stale.not_fresh(), verifier(&reading), "{stale:#?}");
    assert!(stale.ratio() < 1.0 && !stale.gate_passes, "{stale:#?}");
    assert!(
        stale
            .refused
            .values()
            .all(|says| says.starts_with("is stale")),
        "{stale:#?}"
    );
    verify(&p, reading[0]);
    let partial = measured(&p);
    eprintln!("control, one recorded again: {}", partial.line("taskboard"));
    assert_eq!(
        partial.not_fresh().len(),
        stale.not_fresh().len() - 1,
        "{partial:#?}"
    );
    assert!(
        partial.ratio() < 1.0 && !partial.gate_passes,
        "{partial:#?}"
    );
    for (_, entity) in partial.not_fresh() {
        verify(&p, &entity);
    }
    let again = measured(&p);
    assert_eq!(again.ratio(), 1.0, "{again:#?}");
    assert!(again.refused.is_empty() && again.gate_passes, "{again:#?}");
}

/// The negative control: the twin before its verifier ran owes the same seven judgements, none fresh, and ship is
/// refused once for each, as missing.
#[test]
fn stripped_verdicts_give_no_freshness_and_one_refusal_each() {
    let p = strict_twin();
    p.strip_verdicts();
    let f = measured(&p);
    eprintln!("control, verdicts stripped: {}", f.line("taskboard"));
    assert!(f.fresh.is_empty(), "{f:#?}");
    assert_eq!(f.ratio(), 0.0);
    assert_eq!(f.refused.len(), MUST_HAVES.len(), "{f:#?}");
    assert!(
        f.refused.values().all(|says| says == "is missing"),
        "{f:#?}"
    );
    assert!(!f.gate_passes);
}
