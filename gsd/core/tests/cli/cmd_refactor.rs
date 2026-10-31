//! `propose_refactor` and `dispose_refactor`: the complexity trigger's proposals (the scorer's own vectors are in
//! `src/commands/complexity/tests.rs`).

use super::commands_support::*;
use super::harness::{Project, project};

pub(super) const PHASE: &str = "v1.0/01";

/// A function of `branches` ifs: it scores `branches + 1`.
pub(super) fn busy(name: &str, branches: usize) -> String {
    let body: String = (0..branches)
        .map(|i| format!("  if (x === {i}) {{ return {i}; }}\n"))
        .collect();
    format!("export function {name}(x) {{\n{body}  return -1;\n}}\n")
}

pub(super) fn with_source(source: &str) -> Project {
    let p = project("execute");
    p.write("src/tasks.js", source);
    p
}

#[test]
fn propose_refactor_reads_the_plans_files_and_creates_a_proposal() {
    let p = with_source(&format!("{}\n{}", busy("add", 16), busy("list", 3)));
    let run = gsd(&p, "propose-refactor", &[PHASE]);
    assert_eq!(run.code, 0, "{}", run.all());
    let found = p.fields_of("v1.0/01/RP1");
    assert_eq!(found["target_file"], "src/tasks.js");
    assert_eq!(found["target_function"], "add");
    assert_eq!(found["score"], 17);
    assert_eq!(
        found["baseline"], 17,
        "no earlier disposition: no jump on first sight"
    );
    assert_eq!(found["delta"], 0);
    assert_eq!(found["reasons"], serde_json::json!(["threshold"]));
    let candidates = found["candidates"].as_array().expect("candidates");
    assert_eq!(candidates.len(), 1, "{found}");
    assert_eq!(candidates[0]["function"], "add");
    assert_eq!(candidates[0]["score"], 17);
    assert_eq!(candidates[0]["line"], 1);
    assert_eq!(found["status"], "proposed");
    assert!(
        !p.codes().iter().any(|c| c == "E083" || c == "E006"),
        "{:?}",
        p.codes()
    );
}

#[test]
fn nothing_crossing_writes_nothing() {
    let p = with_source(&busy("add", 3));
    let before = bytes(&p);
    let run = gsd(&p, "propose-refactor", &[PHASE]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("nothing crosses"), "{}", run.all());
    unchanged(&p, &before);
}

#[test]
fn the_baseline_is_the_last_disposed_score() {
    // add scores 10 over a threshold of 5: proposed, then accepted.
    let p = with_source(&busy("add", 9));
    p.settings(serde_json::json!({ "refactor.complexity_threshold": 5 }));
    assert_eq!(gsd(&p, "propose-refactor", &[PHASE]).code, 0);
    assert_eq!(p.fields_of("v1.0/01/RP1")["score"], 10);
    let run = gsd_as_person(&p, "dispose-refactor", &["v1.0/01/RP1", "accepted", "ada"]);
    assert_eq!(run.code, 0, "{}", run.all());
    // Later add scores 16: under the new threshold of 30, but 6 up from the disposed 10, over the jump of 5.
    p.write("src/tasks.js", &busy("add", 15));
    p.settings(serde_json::json!({ "refactor.complexity_threshold": 30 }));
    let run = gsd(&p, "propose-refactor", &[PHASE]);
    assert_eq!(run.code, 0, "{}", run.all());
    let found = p.fields_of("v1.0/01/RP2");
    assert_eq!(found["score"], 16);
    assert_eq!(found["baseline"], 10);
    assert_eq!(found["delta"], 6);
    assert_eq!(found["reasons"], serde_json::json!(["jump"]));
    // A rise of exactly the jump delta is not a jump.
    p.write("src/tasks.js", &busy("add", 14));
    let before = bytes(&p);
    let run = gsd(&p, "propose-refactor", &[PHASE]);
    assert!(run.all().contains("nothing crosses"), "{}", run.all());
    unchanged(&p, &before);
}

#[test]
fn strict_records_a_deviation_window() {
    let p = with_source(&busy("add", 16));
    p.settings(serde_json::json!({ "refactor.trigger_strict": true }));
    let run = gsd(&p, "propose-refactor", &[PHASE]);
    assert_eq!(run.code, 0, "{}", run.all());
    let window = p.fields_of("BW1");
    assert_eq!(window["kind"], "deviation");
    assert_eq!(window["status"], "open");
    assert_eq!(window["refactor"], "v1.0/01/RP1");
    assert!(
        window["recorded_at"]
            .as_str()
            .is_some_and(|at| at.ends_with('Z')),
        "{window}"
    );
    assert_eq!(
        gsd_as_person(&p, "dispose-refactor", &["v1.0/01/RP1", "accepted", "ada"]).code,
        0
    );
    assert_eq!(
        p.fields_of("BW1")["status"],
        "fixed",
        "an accepted proposal closes its window"
    );
}

#[test]
fn a_second_run_does_not_duplicate_an_untriaged_proposal() {
    let p = with_source(&busy("add", 16));
    p.settings(serde_json::json!({ "refactor.trigger_strict": true }));
    assert_eq!(gsd(&p, "propose-refactor", &[PHASE]).code, 0);
    let before = bytes(&p);
    let run = gsd(&p, "propose-refactor", &[PHASE]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(
        run.all().contains("untriaged") && run.all().contains("RP1"),
        "{}",
        run.all()
    );
    unchanged(&p, &before);
    // Disposed, the function is no longer frozen.
    assert_eq!(
        gsd_as_person(
            &p,
            "dispose-refactor",
            &["v1.0/01/RP1", "declined", "ada", "--reason", "later"]
        )
        .code,
        0
    );
    assert_eq!(gsd(&p, "propose-refactor", &[PHASE]).code, 0);
    assert_eq!(p.fields_of("v1.0/01/RP2")["target_function"], "add");
}

#[test]
fn dispose_is_refused_twice() {
    let p = project("verify");
    let run = gsd_as_person(&p, "dispose-refactor", &["RP1", "declined", "ada"]);
    refused(&run, "invalid_input", &["needs --reason"]);
    let run = gsd_as_person(&p, "dispose-refactor", &["RP1", "accepted", "ada"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let found = p.fields_of("RP1");
    assert_eq!(found["disposition"]["outcome"], "accepted");
    assert_eq!(found["status"], "accepted");
    let before = bytes(&p);
    let again = gsd_as_person(
        &p,
        "dispose-refactor",
        &["RP1", "declined", "grace", "--reason", "changed my mind"],
    );
    refused(&again, "invalid_input", &["already disposed"]);
    unchanged(&p, &before);
}
