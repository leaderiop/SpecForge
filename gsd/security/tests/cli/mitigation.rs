//! `gsd.security/W302`: a closed threat owes a judged verdict that its declared disposition holds, written by
//! `@gsd/security`'s own `record_security_verdict` under its qualified rule, and stale once a file it reads changes.

use serde_json::{Value, json};

use super::harness::{Project, Run, judged, vault};

const W302: &str = "gsd.security/W302";
/// A closed threat of plan 01-01, with no `files`.
const THREAT: &str = "v1.0/01/01-01/T-01-01";
/// The one open threat of the vault.
const OPEN: &str = "v1.0/01/01-02/T-01-02";
/// The closed threats of the vault, each owed a verdict.
const CLOSED: [&str; 5] = [
    "v1.0/01/01-01/T-01-01",
    "v1.0/01/01-01/T-01-02",
    "v1.0/01/01-01/T-01-SC",
    "v1.0/01/01-02/T-01-03",
    "v1.0/01/01-02/T-01-SC",
];

fn project() -> Project {
    vault().with_extension("@gsd/security", "GSD_SECURITY_WASM")
}

fn json_of(run: &Run) -> Value {
    assert_eq!(run.code, 0, "{}", run.all());
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("JSON ({e}): {}", run.all()))
}

/// `(entity, message)` of every `gsd.security/W302` diagnostic of `specforge check`, sorted.
fn w302(p: &Project) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = p
        .report_all()
        .into_iter()
        .filter(|(code, _)| code == W302)
        .map(|(_, message)| {
            let id = message.split('\'').nth(1).expect("an entity").to_string();
            (id, message)
        })
        .collect();
    found.sort();
    found
}

/// The verdict's state and whether it passes, as `query --inputs` answers them.
fn state(p: &Project, entity: &str) -> (String, bool) {
    let verdict = &p.inputs(entity, W302)["verdict"];
    (
        verdict["state"].as_str().unwrap_or_default().to_string(),
        verdict["passed"].as_bool().unwrap_or(false),
    )
}

/// The dotted qualified rule end to end, through the CLI alone: `query --inputs` reads it, the security command
/// writes it, the spec and the ledger hold it, and the host reads it back fresh.
#[test]
fn the_qualified_rule_is_read_recorded_and_read_back_w302() {
    let p = project();
    let inputs = json_of(&p.run(&["query", THREAT, "--inputs", W302, "--format", "json"]));
    let names: Vec<&str> = inputs["inputs"]
        .as_array()
        .expect("inputs")
        .iter()
        .filter_map(|i| i["name"].as_str())
        .collect();
    assert_eq!(
        names,
        ["threat", "plan", "files", "accepted", "rows"],
        "{inputs:#}"
    );
    let run = p.run(&[
        "gsd",
        "record-security-verdict",
        THREAT,
        "W302",
        "holds",
        "test-model",
        &inputs["basis"].to_string(),
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(
        p.read("spec/plans.spec")
            .contains("verdict gsd.security/W302 holds {"),
        "{}",
        p.read("spec/plans.spec")
    );
    assert!(
        p.read("specforge-harness")
            .contains(&format!("{THREAT} verdict:{W302} ")),
        "{}",
        p.read("specforge-harness")
    );
    let again = json_of(&p.run(&["query", THREAT, "--inputs", W302, "--format", "json"]));
    assert_eq!(again["verdict"]["state"], "fresh", "{again:#}");
    assert_eq!(again["verdict"]["passed"], true, "{again:#}");
}

/// Every closed threat of the vault, none of which names a file, owes W302 a verdict and has none; the open one owes
/// none. The threat's own `complete_when` under its plan's raises no W112.
#[test]
fn a_closed_threat_without_a_verdict_reports_w302() {
    let p = project();
    let found = w302(&p);
    let ids: Vec<&str> = found.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, CLOSED, "{found:#?}");
    for (id, message) in &found {
        assert_eq!(
            message,
            &format!("threat '{id}' has no fresh mitigation verdict: no verdict")
        );
    }
    let warnings: Vec<String> = p
        .checked()
        .into_iter()
        .filter(|(code, _)| code == W302)
        .map(|(_, message)| message)
        .collect();
    assert_eq!(warnings.len(), CLOSED.len());
    let check = p.run(&["check", "--format", "json"]);
    let all: Value = serde_json::from_str(&check.stdout).expect("check prints JSON");
    for d in all.as_array().expect("a list") {
        assert!(d["code"] != "W112", "{d}");
        if d["code"] == W302 {
            assert_eq!(d["severity"], "Warning", "{d}");
        }
    }
    // a closed threat without files is owed a verdict over its other inputs
    assert_eq!(state(&p, THREAT), ("missing".to_string(), false));
}

/// The open threat owes no verdict, and a fresh `holds` answers a closed one.
#[test]
fn an_open_threat_and_a_fresh_holds_are_silent_w302() {
    let p = project();
    assert!(w302(&p).iter().all(|(id, _)| id != OPEN));
    let run = p.record(THREAT, W302, "holds", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(state(&p, THREAT), ("fresh".to_string(), true));
    assert!(
        w302(&p).iter().all(|(id, _)| id != THREAT),
        "{:#?}",
        w302(&p)
    );
    assert_eq!(judged(&p.report_all()), p.expected_judged(&[]));
}

/// A threat that names its files reads them: an edit makes the verdict stale, naming `files`, until it is judged
/// again.
#[test]
fn an_edit_to_a_threats_file_stales_w302() {
    let p = project();
    p.edit(
        "spec/plans.spec",
        "  mitigation  \"AES-256-GCM under a key that never reaches the disk\"\n",
        "  mitigation  \"AES-256-GCM under a key that never reaches the disk\"\n  files       [\"src/vault.js\"]\n",
    );
    assert_eq!(p.record(THREAT, W302, "holds", &[]).code, 0);
    assert_eq!(state(&p, THREAT), ("fresh".to_string(), true));
    p.write(
        "src/vault.js",
        &format!("{}\n// edited\n", p.read("src/vault.js")),
    );
    let inputs = p.inputs(THREAT, W302);
    assert_eq!(inputs["verdict"]["state"], "stale", "{inputs:#}");
    assert!(
        inputs["verdict"]["changed"]
            .as_array()
            .is_some_and(|c| c.contains(&json!("files"))),
        "{inputs:#}"
    );
    let found = w302(&p);
    let mine = found
        .iter()
        .find(|(id, _)| id == THREAT)
        .expect("W302 on the threat");
    assert!(mine.1.contains("is stale: files changed"), "{found:#?}");
    assert_eq!(p.record(THREAT, W302, "holds", &[]).code, 0);
    assert_eq!(state(&p, THREAT), ("fresh".to_string(), true));
    assert!(w302(&p).iter().all(|(id, _)| id != THREAT));
}

/// `broken` is recorded and reported as the rule's finding.
#[test]
fn a_broken_mitigation_is_reported() {
    let p = project();
    assert_eq!(p.record(THREAT, W302, "broken", &[]).code, 0);
    assert_eq!(state(&p, THREAT), ("fresh".to_string(), false));
    let found = w302(&p);
    let mine = found
        .iter()
        .find(|(id, _)| id == THREAT)
        .expect("W302 on the threat");
    assert!(mine.1.ends_with("is 'broken' (0 findings)"), "{found:#?}");
}

/// Each command writes its own extension's rules only, and only their outcomes.
#[test]
fn a_verdict_is_written_by_its_own_extensions_command() {
    let p = project();
    let basis = p.inputs(THREAT, W302)["basis"].to_string();
    let core = p.run(&[
        "gsd",
        "record-verdict",
        THREAT,
        "W302",
        "holds",
        "test-model",
        &basis,
    ]);
    assert_ne!(core.code, 0, "{}", core.all());
    let truth = "v1.0/01/01-01/TR1";
    let security = p.run(&[
        "gsd",
        "record-security-verdict",
        truth,
        "E501",
        "verified",
        "test-model",
        "{}",
    ]);
    assert_ne!(security.code, 0, "{}", security.all());
    let outcome = p.record_over(THREAT, W302, "verified", &basis, &[]);
    assert_ne!(outcome.code, 0, "{}", outcome.all());
    assert!(
        outcome.all().contains("is no outcome of gsd.security/W302"),
        "{}",
        outcome.all()
    );
    assert!(!p.read("spec/plans.spec").contains("verdict gsd"));
    assert_eq!(state(&p, THREAT).0, "missing");
}

/// A verdict whose outcome is edited by hand is no longer the one the ledger vouches for: `unrecorded`, never fresh.
#[test]
fn a_hand_edited_outcome_is_unrecorded() {
    let p = project();
    assert_eq!(p.record(THREAT, W302, "broken", &[]).code, 0);
    p.edit(
        "spec/plans.spec",
        "verdict gsd.security/W302 broken {",
        "verdict gsd.security/W302 holds {",
    );
    assert_eq!(state(&p, THREAT), ("unrecorded".to_string(), false));
    assert!(w302(&p).iter().any(|(id, _)| id == THREAT));
}

/// With `workflow.security_enforcement` off, W302 is never evaluated.
#[test]
fn enforcement_off_gates_w302() {
    let p = project();
    p.settings_of(
        "@gsd/security",
        json!({"workflow.security_enforcement": false}),
    );
    assert_eq!(w302(&p), Vec::<(String, String)>::new());
    assert_eq!(judged(&p.report_all()), p.expected_judged(&["W302"]));
}
