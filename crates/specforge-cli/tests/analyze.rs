// `specforge analyze` — end-to-end tests against the real binary.

use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[allow(deprecated)]
fn specforge_cmd() -> Command {
    Command::cargo_bin("specforge").unwrap()
}

/// Scaffold a minimal project with the formal extension installed and the
/// given main.spec content.
fn project(spec: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"extensions": ["@specforge/formal"]}"#,
    )
    .unwrap();
    fs::write(dir.path().join("main.spec"), spec).unwrap();
    dir
}

fn json_body(dir: &TempDir, extra_args: &[&str]) -> (serde_json::Value, i32) {
    let output = specforge_cmd()
        .args(["analyze", "--path", dir.path().to_str().unwrap(), "--json"])
        .args(extra_args)
        .output()
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    (doc, output.status.code().unwrap_or(-1))
}

#[test]
fn analyze_clean_project_exits_zero() {
    let dir = project(
        "invariant i1 \"Has obligations\" {\n  guarantee \"always holds\"\n  risk low\n  verify property \"i1 holds under all inputs\"\n}\n",
    );
    let (doc, code) = json_body(&dir, &[]);
    assert_eq!(code, 0, "clean project must exit 0: {doc}");
    assert_eq!(doc["ok"], true);

    let coverage = &doc["passes"][0];
    assert_eq!(coverage["pass"], "coverage");
    assert_eq!(coverage["summary"]["invariants"][0]["risk"], "low");
    assert_eq!(coverage["summary"]["invariants"][0]["total"], 1);
    assert_eq!(coverage["summary"]["invariants"][0]["unverified"], 0);
    assert_eq!(coverage["summary"]["obligations"], 1);
}

#[test]
fn analyze_high_risk_unverified_invariant_fails() {
    let dir =
        project("invariant bad \"Never verified\" {\n  guarantee \"nothing\"\n  risk high\n}\n");
    let (doc, code) = json_body(&dir, &[]);
    assert_eq!(code, 1, "high-risk unverified invariant must exit 1: {doc}");
    assert_eq!(doc["ok"], false);
    let findings: Vec<&serde_json::Value> = doc["passes"][0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .collect();
    assert!(
        findings.iter().any(|f| f["code"] == "A002"),
        "expected A002 finding: {findings:?}"
    );
}

#[test]
fn analyze_low_risk_warning_promoted_by_strict() {
    let dir = project("invariant soft \"Softly unverified\" {\n  guarantee \"x\"\n  risk low\n}\n");
    // Without --strict: A002 is a warning, exit 0.
    let (doc, code) = json_body(&dir, &[]);
    assert_eq!(code, 0, "warning alone must not fail: {doc}");
    assert_eq!(doc["ok"], true);

    // With --strict: the warning promotes to an error.
    let (_, code) = json_body(&dir, &["--strict"]);
    assert_eq!(code, 1, "--strict must promote the warning: exit {code}");
}

#[test]
fn analyze_unknown_pass_exits_two() {
    let dir = project("");
    let output = specforge_cmd()
        .args([
            "analyze",
            "nonsense",
            "--path",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown analysis pass"), "{stderr}");
}

#[test]
fn analyze_single_pass_selection() {
    let dir = project(
        "invariant only \"Only coverage\" {\n  guarantee \"g\"\n  risk low\n  verify unit \"u1\"\n}\n",
    );
    let output = specforge_cmd()
        .args([
            "analyze",
            "coverage",
            "--path",
            dir.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let passes = doc["passes"].as_array().unwrap();
    assert_eq!(passes.len(), 1, "only the requested pass runs");
    assert_eq!(passes[0]["pass"], "coverage");
}

#[test]
fn analyze_enforcement_maps_invariant_references() {
    // invariant referenced by a behavior via `invariants [...]` -> enforced;
    // an unreferenced invariant is an orphan guarantee (A011 warning).
    let spec = r#"
invariant held "Held" {
  guarantee "g"
  risk low
  verify property "holds"
}

invariant orphan "Orphan" {
  guarantee "nothing points here"
  risk low
  verify property "holds too"
}

behavior keeper "Keeper" {
  title "Keeper"
  invariants [held]
  verify unit "keeps held"
}
"#;
    let dir = project(spec.trim_start());
    let (doc, code) = json_body(&dir, &[]);
    assert_eq!(code, 0, "warnings must not fail: {doc}");

    let coverage = &doc["passes"][0];
    assert_eq!(coverage["summary"]["invariant_enforced"], 1);
    assert_eq!(coverage["summary"]["invariant_orphans"], 1);

    let a011: Vec<&serde_json::Value> = coverage["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["code"] == "A011")
        .collect();
    assert_eq!(a011.len(), 1, "exactly one orphan: {a011:?}");
    assert!(
        a011[0]["message"].as_str().unwrap().contains("orphan"),
        "A011 message: {a011:?}"
    );

    // --strict promotes the orphan warning to a failure.
    let (_, code) = json_body(&dir, &["--strict"]);
    assert_eq!(code, 1, "--strict must fail on the orphan warning");
}
