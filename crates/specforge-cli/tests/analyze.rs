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

#[test]
fn analyze_discharge_layers_intent_linkage_proof() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"extensions": ["@specforge/formal"]}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("main.spec"),
        concat!(
            "invariant held \"Held\" {\n",
            "  guarantee \"g\"\n",
            "  risk low\n",
            "  verify property \"holds\"\n",
            "  tests [\"tests/held.rs\"]\n",
            "}\n",
            "\n",
            "invariant unlinked \"Unlinked\" {\n",
            "  guarantee \"g\"\n",
            "  risk low\n",
            "  verify property \"holds\"\n",
            "}\n",
        ),
    )
    .unwrap();
    let tests_dir = dir.path().join("tests");
    fs::create_dir_all(&tests_dir).unwrap();
    fs::write(tests_dir.join("held.rs"), "// test").unwrap();

    // Layer 2: the existing link is accepted, the unlinked intent is info-only.
    let (doc, code) = json_body(&dir, &[]);
    assert_eq!(code, 0);
    let coverage = &doc["passes"][0];
    let funnel = &coverage["summary"]["discharge_funnel"];
    assert_eq!(funnel["entities_with_obligations"], 2);
    assert_eq!(funnel["entities_with_test_links"], 1);
    assert_eq!(funnel["broken_test_links"], 0);
    let codes: Vec<&str> = coverage["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["code"].as_str())
        .collect();
    assert!(
        codes.contains(&"A012"),
        "unlinked intent reported: {codes:?}"
    );

    // Layer 2 broken: point `tests` at a file that does not exist.
    fs::write(
        dir.path().join("main.spec"),
        concat!(
            "invariant held \"Held\" {\n",
            "  guarantee \"g\"\n",
            "  risk low\n",
            "  verify property \"holds\"\n",
            "  tests [\"tests/missing.rs\"]\n",
            "}\n",
        ),
    )
    .unwrap();
    let (doc, code) = json_body(&dir, &[]);
    assert_eq!(code, 0, "A013 is a warning, not an error");
    let a013: Vec<&serde_json::Value> = doc["passes"][0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["code"] == "A013")
        .collect();
    assert_eq!(a013.len(), 1, "broken linkage reported: {a013:?}");

    // Layer 3: a failing test in the report is an error and fails the run.
    let report = serde_json::json!({
        "specforge": "1.0",
        "runner": "manual",
        "results": {
            "held": {
                "file": "tests/missing.rs",
                "tests": [{"name": "holds", "status": "fail"}]
            }
        }
    });
    fs::write(dir.path().join("report.json"), report.to_string()).unwrap();
    let output = specforge_cmd()
        .args([
            "analyze",
            "coverage",
            "--path",
            dir.path().to_str().unwrap(),
            "--json",
            "--test-results",
            dir.path().join("report.json").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "failing proof must exit 1: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(doc["ok"], false);
    let a014: Vec<&serde_json::Value> = doc["passes"][0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["code"] == "A014")
        .collect();
    assert_eq!(a014.len(), 1, "A014 reported: {a014:?}");

    // Proven path: all tests passing -> entity counted as proven, exit 0.
    let report = serde_json::json!({
        "specforge": "1.0",
        "runner": "manual",
        "results": {
            "held": {
                "file": "tests/missing.rs",
                "tests": [{"name": "holds", "status": "pass"}]
            }
        }
    });
    fs::write(dir.path().join("report.json"), report.to_string()).unwrap();
    let output = specforge_cmd()
        .args([
            "analyze",
            "coverage",
            "--path",
            dir.path().to_str().unwrap(),
            "--json",
            "--test-results",
            dir.path().join("report.json").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let funnel = &doc["passes"][0]["summary"]["discharge_funnel"];
    assert_eq!(funnel["entities_proven"], 1);
}

#[test]
fn analyze_dispatches_extension_compiler_pass() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"extensions": ["@specforge/formal"]}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("main.spec"),
        concat!(
            "behavior obligated \"Obligated\" {\n",
            "  title \"Obligated\"\n",
            "  requires {\n",
            "    auth_ready \"auth is configured\"\n",
            "  }\n",
            "  verify unit \"runs\"\n",
            "}\n",
        ),
    )
    .unwrap();

    let output = specforge_cmd()
        .args(["analyze", "--path", dir.path().to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    let pass = doc["passes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pass"] == "@specforge/formal:condition_check")
        .expect("formal condition_check pass must be dispatched");
    let a = pass["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["code"] == "W096")
        .count();
    assert_eq!(a, 1, "requires-without-ensures must surface W075");
    assert_eq!(pass["summary"]["entities_analyzed"], 1);
    assert_eq!(pass["summary"]["extension"], "@specforge/formal");
}

#[test]
fn analyze_event_graph_flags_unconsumed_events() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"extensions": ["@specforge/formal", "@specforge/software"]}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("main.spec"),
        concat!(
            "event tick \"Tick\" {\n",
            "  title \"Tick\"\n",
            "  channel \"sys.tick\"\n",
            "}\n",
            "\n",
            "behavior ticker \"Ticker\" {\n",
            "  title \"Ticker\"\n",
            "  produces [tick]\n",
            "  verify unit \"emits tick\"\n",
            "}\n",
        ),
    )
    .unwrap();

    let output = specforge_cmd()
        .args(["analyze", "--path", dir.path().to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let report = doc["passes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pass"] == "@specforge/formal:event_graph_analyze")
        .expect("event_graph_analyze pass must be dispatched");
    let w029: Vec<&serde_json::Value> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["code"] == "W029")
        .collect();
    assert_eq!(w029.len(), 1, "unconsumed producer must surface W029");
}

#[test]
fn analyze_orders_extension_passes_by_constraints() {
    // The formal extension declares its passes shuffled (event_graph_analyze
    // first); the host must order them by the after-constraints:
    // condition_check -> layering_verify -> event_graph_analyze.
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"extensions": ["@specforge/formal"]}"#,
    )
    .unwrap();
    fs::write(dir.path().join("main.spec"), "").unwrap();

    let output = specforge_cmd()
        .args(["analyze", "--path", dir.path().to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let formal: Vec<&str> = doc["passes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["pass"].as_str())
        .filter(|n| n.starts_with("@specforge/formal:"))
        .map(|n| n.trim_start_matches("@specforge/formal:"))
        .collect();
    assert_eq!(
        formal,
        vec![
            "condition_check",
            "layering_verify",
            "event_graph_analyze",
            "coverage_tracking"
        ],
        "constraint order must beat declaration order: {formal:?}"
    );
}

#[test]
fn analyze_prove_flags_unsatisfiable_constraint() {
    if std::process::Command::new("z3")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("z3 not installed — skipping prove e2e");
        return;
    }

    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"extensions": ["@specforge/governance"]}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("main.spec"),
        concat!(
            "constraint impossible \"Impossible Bounds\" {\n",
            "  description \"Bounds that can never both hold\"\n",
            "  category performance\n",
            "  priority critical\n",
            "  metric \"\"\"\n",
            "    latency < 100ms\n",
            "    latency > 500ms\n",
            "  \"\"\"\n",
            "}\n",
        ),
    )
    .unwrap();

    let output = specforge_cmd()
        .args([
            "analyze",
            "--prove",
            "--path",
            dir.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "unsatisfiable must exit 1");
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(doc["ok"], false);
    let prove = doc["passes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pass"] == "prove")
        .expect("prove pass must be dispatched");
    assert!(
        prove["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "E046"),
        "E046 must surface: {prove}"
    );
    assert_eq!(prove["summary"]["unsatisfiable"], 1);
}

#[test]
fn analyze_prove_satisfiable_constraint_exits_zero() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"extensions": ["@specforge/governance"]}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("main.spec"),
        concat!(
            "constraint tight \"Tight But Possible\" {\n",
            "  description \"Bounds that can both hold\"\n",
            "  category performance\n",
            "  priority critical\n",
            "  metric \"\"\"\n",
            "    latency < 100ms\n",
            "    latency > 10ms\n",
            "  \"\"\"\n",
            "}\n",
        ),
    )
    .unwrap();

    let output = specforge_cmd()
        .args([
            "analyze",
            "--prove",
            "--path",
            dir.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(doc["ok"], true);
    let prove = doc["passes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pass"] == "prove")
        .expect("prove pass must be dispatched");
    assert_eq!(prove["summary"]["satisfiable"], 1);
    assert_eq!(prove["summary"]["unsatisfiable"], 0);
}
