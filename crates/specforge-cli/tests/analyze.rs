// `specforge analyze` — end-to-end tests against the real binary.

use std::fs;

use assert_cmd::Command;
use specforge_test_macros::test as specforge_test;
use tempfile::TempDir;

#[allow(deprecated)]
fn specforge_cmd() -> Command {
    Command::cargo_bin("specforge").unwrap()
}

/// Scaffold a minimal project with the formal and testing extensions
/// installed and the given main.spec content.
fn project(spec: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"extensions": ["@specforge/formal", "@specforge/testing"]}"#,
    )
    .unwrap();
    fs::write(dir.path().join("main.spec"), spec).unwrap();
    dir
}

/// The coverage pass report (owned by @specforge/testing, ADR 0002).
fn coverage(doc: &serde_json::Value) -> &serde_json::Value {
    doc["passes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pass"] == "@specforge/testing:coverage")
        .unwrap_or_else(|| panic!("no coverage pass in {doc}"))
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

    let coverage = coverage(&doc);
    assert_eq!(coverage["summary"]["invariants"][0]["risk"], "low");
    assert_eq!(coverage["summary"]["invariants"][0]["total"], 1);
    assert_eq!(coverage["summary"]["invariants"][0]["unverified"], 0);
    assert_eq!(coverage["summary"]["obligations"], 1);
}

#[specforge_test(
    behavior = "te_coverage_pass",
    verify = "a high-risk invariant without obligations is an A002 error"
)]
fn analyze_high_risk_unverified_invariant_fails() {
    let dir =
        project("invariant bad \"Never verified\" {\n  guarantee \"nothing\"\n  risk high\n}\n");
    let (doc, code) = json_body(&dir, &[]);
    assert_eq!(code, 1, "high-risk unverified invariant must exit 1: {doc}");
    assert_eq!(doc["ok"], false);
    let findings: Vec<&serde_json::Value> = coverage(&doc)["findings"]
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
    assert_eq!(passes[0]["pass"], "@specforge/testing:coverage");
}

#[specforge_test(
    behavior = "te_coverage_pass",
    verify = "invariant references count as enforcement"
)]
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

    let coverage = coverage(&doc);
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

#[specforge_test(
    behavior = "te_coverage_pass",
    verify = "recorded test results prove entities and failing tests are A014"
)]
fn analyze_discharge_intent_and_proof() {
    let dir = project(concat!(
        "invariant held \"Held\" {\n",
        "  guarantee \"g\"\n",
        "  risk low\n",
        "  verify property \"holds\"\n",
        "}\n",
        "\n",
        "invariant untested \"Untested\" {\n",
        "  guarantee \"g\"\n",
        "  risk low\n",
        "  verify property \"holds\"\n",
        "}\n",
    ));

    // Intent: both declare obligations; nothing is proven without results.
    let (doc, code) = json_body(&dir, &[]);
    assert_eq!(code, 0);
    let funnel = &coverage(&doc)["summary"]["discharge_funnel"];
    assert_eq!(funnel["entities_with_obligations"], 2);
    assert_eq!(funnel["entities_proven"], 0);

    // Proof: a failing test in the recorded results is an error.
    let analyze_with = |report: serde_json::Value| {
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
        let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        (doc, output.status.code())
    };
    let (doc, code) = analyze_with(serde_json::json!({
        "runner": "manual",
        "results": {"held": {"tests": [{"name": "holds", "status": "fail"}]}}
    }));
    assert_eq!(code, Some(1), "failing proof must exit 1: {doc}");
    assert_eq!(doc["ok"], false);
    let a014: Vec<&serde_json::Value> = coverage(&doc)["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["code"] == "A014")
        .collect();
    assert_eq!(a014.len(), 1, "A014 reported: {a014:?}");

    // Proven: all recorded tests pass.
    let (doc, code) = analyze_with(serde_json::json!({
        "runner": "manual",
        "results": {"held": {"tests": [{"name": "holds", "status": "pass"}]}}
    }));
    assert_eq!(code, Some(0));
    let summary = &coverage(&doc)["summary"];
    assert_eq!(summary["discharge_funnel"]["entities_proven"], 1);
    assert_eq!(summary["test_results"]["runner"], "manual");
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
    assert_eq!(a, 1, "requires-without-ensures must surface W096");
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

fn layering_findings(doc: &serde_json::Value) -> Vec<(String, String)> {
    doc["passes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pass"] == "@specforge/formal:layering_verify")
        .expect("layering_verify pass must be dispatched")["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["code"].as_str().unwrap().to_string(),
                f["message"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// Through the real binary and embedded formal blob: the pass must read the
/// edges `refinement` entities actually produce (C10-02 was unreachable when
/// it matched labels no field emits).
#[test]
fn analyze_refinement_that_drops_an_ensures_condition_is_e031() {
    let dir = project(concat!(
        "behavior provision \"Provision\" {\n",
        "  title \"Provision\"\n",
        "  ensures {\n",
        "    config_created \"config exists\"\n",
        "    file_created \"file exists\"\n",
        "  }\n",
        "  verify contract \"abstract contract\"\n",
        "}\n",
        "behavior provision_full \"Provision (full)\" {\n",
        "  title \"Provision (full)\"\n",
        "  ensures {\n",
        "    config_created \"config exists\"\n",
        "    file_created \"file exists\"\n",
        "    audit_logged \"audit entry written\"\n",
        "  }\n",
        "  verify unit \"full\"\n",
        "}\n",
        "behavior provision_lazy \"Provision (lazy)\" {\n",
        "  title \"Provision (lazy)\"\n",
        "  ensures {\n",
        "    config_created \"config exists\"\n",
        "  }\n",
        "  verify unit \"lazy\"\n",
        "}\n",
        "refinement full_refines \"Full\" {\n",
        "  abstract_entity provision\n",
        "  concrete_entity provision_full\n",
        "}\n",
        "refinement lazy_refines \"Lazy\" {\n",
        "  abstract_entity provision\n",
        "  concrete_entity provision_lazy\n",
        "}\n",
    ));

    let (doc, code) = json_body(&dir, &[]);
    let findings = layering_findings(&doc);
    let e031: Vec<&String> = findings
        .iter()
        .filter(|(c, _)| c == "E031")
        .map(|(_, m)| m)
        .collect();
    assert_eq!(
        e031.len(),
        1,
        "only the weakening refinement fires: {findings:?}"
    );
    assert!(
        e031[0].contains("lazy_refines") && e031[0].contains("file_created"),
        "names the refinement and the dropped condition: {}",
        e031[0]
    );
    assert_ne!(code, 0, "an E031 error must fail analyze");
}

/// The field form of specification layering (`abstract true` + `refines`),
/// declared by @specforge/formal on @specforge/software behaviors.
#[test]
fn refines_and_abstract_fields_drive_layering_end_to_end() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"extensions": ["@specforge/formal", "@specforge/software", "@specforge/testing"]}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("main.spec"),
        concat!(
            "behavior provision \"Provision\" {\n",
            "  title \"Provision\"\n",
            "  abstract true\n",
            "  ensures {\n",
            "    config_created \"config exists\"\n",
            "    file_created \"file exists\"\n",
            "  }\n",
            "}\n",
            "behavior provision_lazy \"Provision (lazy)\" {\n",
            "  title \"Provision (lazy)\"\n",
            "  refines provision\n",
            "  ensures {\n",
            "    config_created \"config exists\"\n",
            "  }\n",
            "  verify unit \"lazy\"\n",
            "}\n",
            "behavior plain \"Plain\" {\n  title \"Plain\"\n  verify unit \"plain\"\n}\n",
            "behavior derived \"Derived\" {\n",
            "  title \"Derived\"\n",
            "  refines plain\n",
            "  verify unit \"derived\"\n",
            "}\n",
            "behavior orphan_spec \"Orphan spec\" {\n",
            "  title \"Orphan spec\"\n",
            "  abstract true\n",
            "}\n",
        ),
    )
    .unwrap();

    let check = specforge_cmd()
        .args(["check", dir.path().to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let diagnostics: Vec<serde_json::Value> = serde_json::from_slice(&check.stdout).unwrap();
    let flagged: Vec<String> = diagnostics
        .iter()
        .filter(|d| d["code"] == "W020" || d["code"] == "W004")
        .map(|d| d["message"].as_str().unwrap().to_string())
        .collect();
    assert!(
        flagged.is_empty(),
        "abstract/refines are declared fields and abstract behaviors need no verify: {flagged:?}"
    );

    let (doc, code) = json_body(&dir, &[]);
    let findings = layering_findings(&doc);
    let codes: Vec<&str> = findings.iter().map(|(c, _)| c.as_str()).collect();
    assert_eq!(codes, vec!["E031", "W030", "W110"], "{findings:?}");
    assert!(
        findings[0]
            .1
            .contains("'provision_lazy' refines 'provision'")
    );
    assert!(
        findings[1].1.contains("'orphan_spec'"),
        "only the unrefined abstract"
    );
    assert!(findings[2].1.contains("'derived' refines 'plain'"));
    assert_ne!(code, 0, "E031 fails analyze");
}

#[test]
fn analyze_refinement_cycle_is_e041() {
    let dir = project(concat!(
        "behavior alpha \"Alpha\" {\n  title \"Alpha\"\n  verify unit \"a\"\n}\n",
        "behavior beta \"Beta\" {\n  title \"Beta\"\n  verify unit \"b\"\n}\n",
        "refinement alpha_refines_beta \"A\" {\n",
        "  abstract_entity beta\n",
        "  concrete_entity alpha\n",
        "}\n",
        "refinement beta_refines_alpha \"B\" {\n",
        "  abstract_entity alpha\n",
        "  concrete_entity beta\n",
        "}\n",
    ));

    let (doc, _) = json_body(&dir, &[]);
    let codes: Vec<String> = layering_findings(&doc)
        .into_iter()
        .map(|(c, _)| c)
        .collect();
    assert_eq!(codes, vec!["E041"], "a two-refinement loop is one cycle");
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
    assert_eq!(prove["summary"]["unsatisfiable"], true);
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
    assert_eq!(prove["summary"]["satisfiable"], true);
    assert_eq!(prove["summary"]["unsatisfiable"], false);
}

#[specforge_test(
    behavior = "te_coverage_pass",
    verify = "a proved formal claim discharges verify property obligations"
)]
fn analyze_proved_claims_discharge_verify_property_obligations() {
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
        r#"{"extensions": ["@specforge/governance", "@specforge/testing"]}"#,
    )
    .unwrap();
    fs::create_dir(dir.path().join("spec")).unwrap();
    fs::write(
        dir.path().join("spec/main.spec"),
        r#"
constraint budget "Latency Budget" {
    description "Budget"
    metric expr {
        latency < 100ms
    }
}

invariant responsive "System Stays Responsive" {
    description "Follows from the declared budget"
    expression expr {
        latency < 250ms
    }
    verify property "latency bound entails responsiveness"
}
"#,
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

    let prove = doc["passes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pass"] == "prove")
        .expect("prove pass must be dispatched");
    assert_eq!(prove["summary"]["claims"], 1);
    assert_eq!(prove["summary"]["claims_proved"], 1);

    assert_eq!(
        coverage(&doc)["summary"]["discharge_funnel"]["formally_discharged"],
        1,
        "the proved claim must discharge the verify property obligation"
    );
}
