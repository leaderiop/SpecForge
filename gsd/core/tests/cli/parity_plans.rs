//! The taskboard's SpecForge twin of its plans, and the gate-cutover twin, against what gsd-tools 1.18.0 says
//! (parity suite): `phase-plan-index`, `verify plan-structure` and the plan-time `check` gates. The goldens come from
//! `record.sh` (taskboard) and from gsd-core's own `tests/fixtures/gate-cutover/` (the other project).

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

use crate::harness::{Project, project, taskboard};

fn parity(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../parity")
        .join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("read {}: {e}", full.display()))
}

fn json(text: &str, what: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{what} is JSON: {e}"))
}

fn golden(name: &str) -> Value {
    json(&parity(&format!("goldens/taskboard/{name}.json")), name)
}

/// What a gate-cutover golden printed on stdout.
pub(super) fn printed(name: &str) -> Value {
    let file = json(
        &parity(&format!("upstream/v1.18.0/gate-cutover/{name}.json")),
        name,
    );
    json(file["stdout"].as_str().expect("a stdout"), name)
}

/// The nodes `specforge query <name> --depth 1` prints.
fn neighbours(p: &Project, name: &str) -> Vec<Value> {
    let run = p.run(&["query", name, "--depth", "1", "--format", "json"]);
    assert_eq!(run.code, 0, "query {name}: {}", run.all());
    json(&run.stdout, name)["nodes"]
        .as_array()
        .expect("nodes")
        .clone()
}

/// `(code, entity, message)` of every `gsd/` diagnostic of `check`.
pub(super) fn diagnostics(p: &Project) -> Vec<(String, String, String)> {
    let run = p.run(&["check", "--format", "json"]);
    json(&run.stdout, "check")
        .as_array()
        .expect("a list")
        .iter()
        .filter(|d| d["code"].as_str().is_some_and(|c| c.starts_with("gsd/")))
        .map(|d| {
            let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
            (
                text(&d["code"]),
                text(&d["data"]["entity"]),
                text(&d["message"]),
            )
        })
        .collect()
}

fn with_code<'a>(
    found: &'a [(String, String, String)],
    code: &'a str,
) -> impl Iterator<Item = &'a (String, String, String)> {
    found.iter().filter(move |(c, ..)| c == code)
}

/// The text between the first pair of backticks: the verify command a message names.
fn command_of(message: &str) -> String {
    message
        .split('`')
        .nth(1)
        .unwrap_or_else(|| panic!("no command in {message}"))
        .to_string()
}

fn names(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("a list")
        .iter()
        .map(|v| v.as_str().expect("a string").to_string())
        .collect()
}

#[test]
fn taskboard_waves_match() {
    let p = taskboard();
    // The twin checks clean (no E201, W204, W406 or E083): only the judged rules report, on every plan.
    p.check_exact(&[]);
    for (index, phase) in [
        ("phase-plan-index-1", "v1.0/01"),
        ("phase-plan-index-2", "v1.0/02"),
    ] {
        let gsd = golden(index);
        let mut waves: std::collections::BTreeMap<String, Vec<String>> = Default::default();
        for plan in gsd["plans"].as_array().expect("plans") {
            let id = plan["id"].as_str().expect("an id");
            let nodes = neighbours(&p, &format!("{phase}/{id}"));
            let twin = nodes
                .iter()
                .find(|n| n["id"] == format!("{phase}/{id}"))
                .unwrap_or_else(|| panic!("no plan {id}"));
            let fields = &twin["fields"];
            assert_eq!(fields["wave"], plan["wave"], "{id} wave");
            assert_eq!(fields["autonomous"], plan["autonomous"], "{id} autonomous");
            let depends: Vec<String> = fields.get("depends_on").map(names).unwrap_or_default();
            assert_eq!(
                json(&serde_json::to_string(&depends).expect("json"), "depends"),
                plan["depends_on"],
                "{id} depends_on"
            );
            assert_eq!(
                fields["files_modified"], plan["files_modified"],
                "{id} files_modified"
            );
            assert_eq!(
                fields
                    .get("files_deleted")
                    .cloned()
                    .unwrap_or(Value::Array(vec![])),
                plan["files_deleted"],
                "{id} files_deleted"
            );
            assert_eq!(fields["objective"], plan["objective"], "{id} objective");
            let steps = nodes
                .iter()
                .filter(|n| n["kind"] == "task" || n["kind"] == "checkpoint")
                .count();
            assert_eq!(steps, plan["task_count"], "{id} tasks and checkpoints");
            waves
                .entry(twin["fields"]["wave"].to_string())
                .or_default()
                .push(id.to_string());
        }
        let expected: std::collections::BTreeMap<String, Vec<String>> = gsd["waves"]
            .as_object()
            .expect("waves")
            .iter()
            .map(|(wave, ids)| (wave.clone(), names(ids)))
            .collect();
        assert_eq!(waves, expected, "{index} waves");
    }
}

#[test]
fn taskboard_plan_verdicts_match() {
    let p = taskboard();
    for plan in ["01-01", "01-02", "02-01"] {
        let verdict = golden(&format!("verify-plan-structure-{plan}"));
        assert_eq!(verdict["valid"], true, "{plan}");
        assert_eq!(verdict["errors"], serde_json::json!([]), "{plan}");
    }
    // These goldens list no command (every taskboard task has a bare `<verify>`): they prove only 0 blockers. The
    // E320 and E507 proof is the gate-cutover test below.
    for name in ["verify-command-paths", "verify-failure-directions"] {
        for phase in ["1", "2"] {
            let verdict = golden(&format!("{name}-{phase}"));
            assert_eq!(verdict["counts"]["blocker"], 0, "{name}-{phase}");
        }
    }
    for phase in ["1", "2"] {
        let gap = golden(&format!("gap-analysis-plan-post-{phase}"));
        assert_eq!(
            gap["counts"]["uncovered"], 0,
            "gap-analysis-plan-post-{phase}"
        );
    }
    let coverage = golden("decision-coverage-plan-2");
    assert_eq!(
        (coverage["covered"].as_u64(), coverage["total"].as_u64()),
        (Some(3), Some(3))
    );

    let found = diagnostics(&p);
    let silent: BTreeSet<&str> = [
        "gsd/E303", "gsd/E307", "gsd/W308", "gsd/W309", "gsd/E320", "gsd/E507", "gsd/W324",
        "gsd/W317", "gsd/E310",
    ]
    .into();
    let stray: Vec<_> = found
        .iter()
        .filter(|(c, ..)| silent.contains(c.as_str()))
        .collect();
    assert!(stray.is_empty(), "{stray:#?}");
}

/// `p` without the first `decision <name> ` block of `spec/discuss.spec`.
pub(super) fn without_decision(p: &Project, name: &str) {
    let text = p.read("spec/discuss.spec");
    let start = text
        .find(&format!("decision {name} "))
        .unwrap_or_else(|| panic!("no decision {name}"));
    let end = start + text[start..].find("\n}\n").expect("a block end") + 3;
    let end = end + text[end..].len() - text[end..].trim_start_matches('\n').len();
    p.write(
        "spec/discuss.spec",
        &format!("{}{}", &text[..start], &text[end..]),
    );
}

pub(super) fn e310_ids(p: &Project) -> Vec<String> {
    diagnostics(p)
        .iter()
        .filter(|(c, ..)| c == "gsd/E310")
        .map(|(_, entity, _)| entity.rsplit('/').next().unwrap_or_default().to_string())
        .collect()
}

#[test]
fn gate_cutover_plan_verdicts_reproduce() {
    let p = project("gate_cutover");
    let mut codes = p.codes();
    codes.sort();
    assert_eq!(
        codes,
        [
            "gsd/E301", "gsd/E310", "gsd/E320", "gsd/E507", "gsd/W317", "gsd/W317"
        ]
    );

    // Decision coverage: the uncovered ids are the E310 entities.
    let uncovered: Vec<String> = printed("decision-coverage-plan-uncovered")["uncovered"]
        .as_array()
        .expect("uncovered")
        .iter()
        .map(|d| d["id"].as_str().expect("an id").to_string())
        .collect();
    assert_eq!(uncovered, ["D-03"]);
    assert_eq!(e310_ids(&p), uncovered);

    let found = diagnostics(&p);
    // Verify command paths: the blocker rows are the E320s; the clean phase has none.
    let paths = printed("verify-command-paths-phase-findings");
    let blockers: BTreeSet<(String, String)> = paths["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .filter(|c| c["severity"] == "blocker")
        .map(|c| {
            assert_eq!(c["reason"], "missing_dir");
            (
                c["command"].as_str().expect("a command").to_string(),
                c["rawTarget"]
                    .as_str()
                    .expect("a target")
                    .trim_start_matches("./")
                    .to_string(),
            )
        })
        .collect();
    let twin: BTreeSet<(String, String)> = with_code(&found, "gsd/E320")
        .map(|(_, _, m)| {
            assert!(m.contains("is not a directory"), "{m}");
            (
                command_of(m),
                m.split("but '")
                    .nth(1)
                    .and_then(|r| r.split('\'').next())
                    .unwrap_or_default()
                    .to_string(),
            )
        })
        .collect();
    assert_eq!(twin, blockers, "E320 against the blocker rows");
    assert_eq!(
        printed("verify-command-paths-phase-clean")["counts"]["blocker"],
        0
    );

    // Failing directions: the `missing` rows are the E507s; the clean phase has none.
    let missing: BTreeSet<String> = printed("verify-failure-directions-phase-findings")["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .filter(|c| c["status"] == "missing")
        .map(|c| c["command"].as_str().expect("a command").to_string())
        .collect();
    let twin: BTreeSet<String> = with_code(&found, "gsd/E507")
        .map(|(_, _, m)| command_of(m))
        .collect();
    assert_eq!(twin, missing, "E507 against the missing rows");
    assert_eq!(
        printed("verify-failure-directions-phase-clean")["counts"]["blocker"],
        0
    );
    assert!(
        found
            .iter()
            .all(|(c, entity, _)| !(c == "gsd/E320" || c == "gsd/E507")
                || !entity.starts_with("v1.0/03/")),
        "the clean phase has no finding"
    );

    // The gap golden's summary is the W317 message's prefix, byte for byte.
    let summary = printed("gap-analysis-plan-post-enabled")["summary"]
        .as_str()
        .expect("a summary")
        .to_string();
    assert!(
        with_code(&found, "gsd/W317")
            .any(|(_, entity, m)| entity == "v1.0/04" && m.starts_with(&summary)),
        "{summary} in {found:#?}"
    );
}

#[test]
fn gate_cutover_variants_reproduce_the_other_verdicts() {
    // All covered: CONTEXT-allcovered.md drops D-03.
    let p = project("gate_cutover");
    without_decision(&p, "D-03");
    let all = printed("decision-coverage-plan-all-covered");
    assert_eq!(
        (all["passed"].clone(), all["uncovered"].clone()),
        (true.into(), serde_json::json!([]))
    );
    assert!(e310_ids(&p).is_empty());

    // No trackable decisions at all.
    let p = project("gate_cutover");
    for name in ["D-03", "D-02", "D-01"] {
        without_decision(&p, name);
    }
    let none = printed("decision-coverage-plan-no-trackable");
    assert_eq!(
        (none["skipped"].clone(), none["total"].clone()),
        (true.into(), 0.into())
    );
    assert!(e310_ids(&p).is_empty());

    // The gate off.
    let p = project("gate_cutover");
    p.settings(serde_json::json!({ "workflow.context_coverage_gate": false }));
    let off = printed("decision-coverage-plan-disabled");
    assert_eq!(
        (off["skipped"].clone(), off["total"].clone()),
        (true.into(), 0.into())
    );
    assert!(e310_ids(&p).is_empty());

    // The gap gate off.
    let p = project("gate_cutover");
    p.settings(serde_json::json!({ "workflow.post_planning_gaps": false }));
    assert_eq!(printed("gap-analysis-plan-post-disabled")["enabled"], false);
    assert!(with_code(&diagnostics(&p), "gsd/W317").next().is_none());
}
