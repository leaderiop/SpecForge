//! The read views on both surfaces (architecture plan 2026-10 02).
//!
//! Stats, trace, the coverage views, the schema, the model and the outline
//! are each one operation over the project view (`specforge_ops::view`),
//! shared by the CLI and MCP. The `*_today` tests pin what each surface
//! answers on two small projects, `fixtures/coverage/fx1` and
//! `fixtures/read_views/rv1`, as insta snapshots
//! (`tests/snapshots/tests__read_views__*.snap`). They prove no spec
//! obligation (they pin current behavior, bugs included), so they carry no
//! `specforge_test` link; a change that alters what they pin re-blesses the
//! snapshot in the same commit, where the diff shows it.
//!
//! rv1 holds a verified behavior `alpha`, an unverified `beta`, a union
//! `Color = Red | Green`, and, under `spec/sub/`, a behavior `gamma`, so a
//! compile of the sub-path has an entity.

use assert_cmd::Command;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::coverage_corpus::{copy_tree, mcp_calls, mcp_responses, project};
use crate::parity::normalized;

/// A scratch copy of `fixtures/read_views/rv1`.
fn rv1() -> TempDir {
    let tmp = TempDir::new().unwrap();
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/read_views/rv1"),
        tmp.path(),
    );
    tmp
}

fn s(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// What a CLI command printed, and its exit code.
struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn cli(args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_specforge"))
        .args(args)
        .output()
        .unwrap();
    Run {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// A command's stdout as JSON.
fn cli_json(args: &[&str]) -> Value {
    let run = cli(args);
    serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("{args:?} is not JSON ({e}): {}{}", run.stdout, run.stderr))
}

/// `text` with the machine-specific parts of `root` replaced.
fn normalized_text(text: &str, root: &Path) -> String {
    let canonical = root.canonicalize().unwrap();
    text.replace(s(&canonical), "[ROOT]")
        .replace(s(root), "[ROOT]")
}

/// The text content of each MCP tool call's result, as the server wrote it.
fn mcp_texts(root: &Path, calls: &[Value]) -> Vec<String> {
    mcp_responses(root, calls)
        .into_iter()
        .map(|resp| {
            resp["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_else(|| panic!("no text content: {resp}"))
                .to_string()
        })
        .collect()
}

/// A schema-shaped document reduced to what does not move whenever an
/// extension adds a field: its version, the kinds and edge types by name,
/// how many validation rules it lists. Other documents are kept whole.
fn schema_digest(value: &Value) -> Value {
    let names = |items: &Value, key: &str| -> Value {
        items
            .as_array()
            .map(|items| items.iter().map(|i| i[key].clone()).collect())
            .unwrap_or(Value::Null)
    };
    if value.get("entity_kinds").is_some() {
        let mut digest = json!({
            "schema_version": value["schema_version"],
            "extensions": value["extensions"],
            "entity_kinds": names(&value["entity_kinds"], "name"),
            "edge_types": names(&value["edge_types"], "label"),
        });
        if value.get("edge_types").is_none() {
            digest.as_object_mut().unwrap().remove("edge_types");
        }
        if let Some(rules) = value.get("validation_rules") {
            digest["validation_rules"] = json!(rules.as_array().map(Vec::len));
        }
        return digest;
    }
    if value.get("fields").is_some() && value.get("name").is_some() {
        return json!({
            "name": value["name"],
            "testable": value["testable"],
            "fields": names(&value["fields"], "name"),
        });
    }
    if value.get("$schema").is_some() {
        let mut properties: Vec<&String> = value["properties"]
            .as_object()
            .map(|p| p.keys().collect())
            .unwrap_or_default();
        properties.sort();
        return json!({
            "$id": value["$id"],
            "title": value["title"],
            "required": value["required"],
            "properties": properties,
        });
    }
    value.clone()
}

/// What `specforge stats` prints for `root`, and what `specforge.stats`
/// answers, pinned under `stats_<name>_*`.
fn pin_stats(name: &str, root: &Path) {
    let json = cli_json(&["stats", "--format", "json", s(root)]);
    insta::assert_snapshot!(format!("stats_{name}_cli"), normalized(&json, root));
    let human = cli(&["stats", s(root)]);
    insta::assert_snapshot!(
        format!("stats_{name}_cli_human"),
        normalized_text(&human.stdout, root)
    );
    let mcp = &mcp_calls(root, &[json!({"name": "specforge.stats", "arguments": {}})])[0];
    insta::assert_snapshot!(format!("stats_{name}_mcp"), normalized(mcp, root));
}

#[test]
fn stats_today() {
    pin_stats("fx1", project("fx1").path());
    pin_stats("rv1", rv1().path());
}

/// `specforge stats` and `specforge.stats` on `root` carry the same numbers
/// in their two shapes (`ProjectStatistics`, `McpStatsResult`).
fn assert_stats_agree(root: &Path) {
    let cli = cli_json(&["stats", "--format", "json", s(root)]);
    let mcp = &mcp_calls(root, &[json!({"name": "specforge.stats", "arguments": {}})])[0];
    let counts: serde_json::Map<String, Value> = mcp["entity_counts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["kind"].as_str().unwrap().to_string(), c["count"].clone()))
        .collect();
    assert_eq!(cli["entities_by_kind"], Value::Object(counts), "{root:?}");
    let total: u64 = mcp["entity_counts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["count"].as_u64().unwrap())
        .sum();
    assert_eq!(cli["total_entities"], json!(total));
    assert_eq!(cli["total_edges"], mcp["edge_count"]);
    assert_eq!(cli["orphan_count"], mcp["orphan_count"]);
    for key in ["declared_pct", "proof_pct", "coverage_pct"] {
        assert_eq!(cli[key], mcp[key], "{key} on {root:?}");
    }
    let summary = &mcp["diagnostic_summary"];
    assert_eq!(cli["error_count"], summary["errors"]);
    assert_eq!(cli["warning_count"], summary["warnings"]);
    assert_eq!(cli["info_count"], summary["infos"]);
}

#[specforge_test_macros::test(
    behavior = "read_views_over_the_project_view",
    verify = "specforge stats and specforge.stats report the same numbers"
)]
fn cli_and_mcp_stats_report_the_same_numbers() {
    assert_stats_agree(project("fx1").path());
    assert_stats_agree(rv1().path());
    let example = TempDir::new().unwrap();
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/todo-app"),
        example.path(),
    );
    assert_stats_agree(example.path());
}

#[test]
fn trace_today() {
    let tmp = project("fx1");
    let root = tmp.path();
    let snap = |name: &str, value: &Value| {
        insta::assert_snapshot!(format!("trace_fx1_{name}"), normalized(value, root));
    };

    snap(
        "login_cli",
        &cli_json(&["trace", "login", "--path", s(root), "--format", "json"]),
    );
    snap(
        "every_cli",
        &cli_json(&["trace", "--path", s(root), "--format", "json"]),
    );
    let human = cli(&["trace", "login", "--path", s(root), "--format", "human"]);
    insta::assert_snapshot!("trace_fx1_login_cli_human", human.stdout);
    let ghost = cli(&["trace", "ghost", "--path", s(root)]);
    insta::assert_snapshot!(
        "trace_fx1_ghost_cli",
        format!("exit: {:?}\nstderr:\n{}", ghost.code, ghost.stderr)
    );

    let mcp = mcp_calls(
        root,
        &[
            json!({"name": "specforge.trace", "arguments": {"entity_id": "login"}}),
            json!({"name": "specforge.trace", "arguments": {"plan": {"entries": [
                {"entity_id": "login"}, {"entity_id": "ghost"}]}}}),
            json!({"name": "specforge.trace", "arguments": {"entity_id": "ghost"}}),
        ],
    );
    snap("login_mcp", &mcp[0]);
    snap("plan_mcp", &mcp[1]);
    snap("ghost_mcp", &mcp[2]);
}

/// For every entity of `root`, `specforge.trace` answers the document
/// `specforge trace <id> --format json` prints.
fn assert_traces_agree(root: &Path) {
    let every = cli_json(&["trace", "--path", s(root), "--format", "json"]);
    let ids: Vec<String> = every["traces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["entity_id"].as_str().unwrap().to_string())
        .collect();
    assert!(!ids.is_empty(), "{root:?}");
    let calls: Vec<Value> = ids
        .iter()
        .map(|id| json!({"name": "specforge.trace", "arguments": {"entity_id": id}}))
        .collect();
    for (id, mcp) in ids.iter().zip(mcp_calls(root, &calls)) {
        let cli = cli_json(&["trace", id, "--path", s(root), "--format", "json"]);
        assert_eq!(mcp, cli, "{id} on {root:?}");
    }
}

#[specforge_test_macros::test(
    behavior = "read_views_over_the_project_view",
    verify = "specforge trace and specforge.trace return the same chain for an entity"
)]
fn cli_and_mcp_trace_are_the_same_chain() {
    assert_traces_agree(project("fx1").path());
    assert_traces_agree(rv1().path());
}

#[test]
fn coverage_views_today() {
    let tmp = project("fx1");
    let root = tmp.path();
    let prompt = |name: &str, arguments: Value| {
        json!({"method": "prompts/get", "params": {
            "name": format!("specforge://prompts/{name}"), "arguments": arguments}})
    };
    let calls = [
        (
            "coverage_all",
            json!({"name": "specforge.coverage", "arguments": {}}),
        ),
        (
            "coverage_status_union",
            json!({"name": "specforge.coverage", "arguments": {"entity_id": "Status"}}),
        ),
        (
            "coverage_kind_behavior",
            json!({"name": "specforge.coverage", "arguments": {"kind": "behavior"}}),
        ),
        (
            "coverage_uncovered",
            json!({"name": "specforge.coverage", "arguments": {"status_filter": "uncovered"}}),
        ),
        (
            "inspect_payload",
            json!({"name": "specforge.inspect", "arguments": {"entity_id": "Payload"}}),
        ),
        (
            "query_login",
            json!({"name": "specforge.query",
                   "arguments": {"entity_id": "login", "include_coverage": true}}),
        ),
        ("prompt_review", prompt("review", json!({}))),
        (
            "prompt_trace_login",
            prompt("trace", json!({"entity_id": "login"})),
        ),
        (
            "prompt_trace_plan",
            prompt(
                "trace",
                json!({"plan": {"entries": [{"entity_id": "login"}, {"entity_id": "ghost"}]}}),
            ),
        ),
    ];
    let requests: Vec<Value> = calls.iter().map(|(_, call)| call.clone()).collect();
    for ((name, _), result) in calls.iter().zip(mcp_calls(root, &requests)) {
        insta::assert_snapshot!(format!("coverage_fx1_{name}"), normalized(&result, root));
    }
}

#[test]
fn schema_today() {
    let tmp = rv1();
    let root = tmp.path();
    let snap = |name: &str, value: &Value| {
        insta::assert_snapshot!(
            format!("schema_rv1_{name}"),
            normalized(&schema_digest(value), root)
        );
    };

    snap("cli", &cli_json(&["schema", s(root)]));
    snap(
        "cli_kind_behavior",
        &cli_json(&["schema", s(root), "--kind", "behavior"]),
    );
    snap(
        "cli_publish_context",
        &cli_json(&["schema", s(root), "--publish", "--format", "context"]),
    );
    let nosuch = cli(&["schema", s(root), "--kind", "nosuch"]);
    insta::assert_snapshot!(
        "schema_rv1_cli_kind_nosuch",
        format!("exit: {:?}\nstderr:\n{}", nosuch.code, nosuch.stderr)
    );

    let mcp = mcp_calls(
        root,
        &[
            json!({"name": "specforge.schema", "arguments": {}}),
            json!({"name": "specforge.schema", "arguments": {"kind": "behavior"}}),
            json!({"name": "specforge.schema", "arguments": {"include_edges": false}}),
            json!({"name": "specforge.schema", "arguments": {"include_validation_rules": true}}),
            json!({"name": "specforge.schema", "arguments": {"kind": "nosuch"}}),
        ],
    );
    snap("mcp", &mcp[0]);
    snap("mcp_kind_behavior", &mcp[1]);
    snap("mcp_no_edges", &mcp[2]);
    snap("mcp_validation_rules", &mcp[3]);
    snap("mcp_kind_nosuch", &mcp[4]);
    let resource = &mcp_responses(
        root,
        &[json!({"method": "resources/read", "params": {"uri": "specforge://schema"}})],
    )[0];
    let text = resource["result"]["contents"][0]["text"].as_str().unwrap();
    snap("resource", &serde_json::from_str(text).unwrap());
}

const MODEL_FORMATS: [&str; 5] = ["markdown", "mermaid", "dot", "json", "dbml"];

#[test]
fn model_and_outline_today() {
    let tmp = rv1();
    let root = tmp.path();
    let calls: Vec<Value> = MODEL_FORMATS
        .iter()
        .map(|format| json!({"name": "specforge.model", "arguments": {"format": format}}))
        .chain(std::iter::once(
            json!({"name": "specforge.outline_extensions", "arguments": {"format": "json"}}),
        ))
        .collect();
    let mcp = mcp_texts(root, &calls);
    for (format, text) in MODEL_FORMATS.iter().zip(&mcp) {
        let run = cli(&["model", s(root), "--format", format]);
        assert_eq!(run.code, Some(0), "{}", run.stderr);
        assert!(!run.stdout.is_empty(), "model --format {format}");
        assert_eq!(&run.stdout, text, "model --format {format}");
    }
    let outline = cli(&["outline", s(root), "--format", "json"]);
    assert_eq!(outline.code, Some(0), "{}", outline.stderr);
    assert_eq!(&outline.stdout, &mcp[5]);
}

/// `rv1` with a `specforge-report.json` that does not parse.
fn rv1_with_a_malformed_report() -> TempDir {
    let tmp = rv1();
    std::fs::write(tmp.path().join("specforge-report.json"), "{not json").unwrap();
    tmp
}

#[specforge_test_macros::test(
    behavior = "read_views_over_the_project_view",
    verify = "the recorded test report is read at the view's root, never an ancestor's"
)]
fn analyze_and_stats_read_the_report_at_the_compiled_root() {
    let tmp = rv1_with_a_malformed_report();
    let sub: PathBuf = tmp.path().join("spec");

    // A sub-path compiles without the project's config, and reads the
    // report at the root it compiled: there is none.
    let stats = cli(&["stats", s(&sub), "--format", "json"]);
    assert_eq!(stats.code, Some(0), "{}", stats.stderr);
    let analyze = cli(&["analyze", "--path", s(&sub), "--json"]);
    assert_eq!(analyze.code, Some(0), "{}", analyze.stderr);

    // At the root both refuse the report.
    for args in [
        vec!["stats", s(tmp.path()), "--format", "json"],
        vec!["analyze", "--path", s(tmp.path()), "--json"],
    ] {
        let run = cli(&args);
        assert_eq!(run.code, Some(2), "{args:?}: {}", run.stdout);
        assert!(
            run.stderr.contains("invalid test results"),
            "{}",
            run.stderr
        );
    }
}

/// The kinds `<dir>/.specforge/schema-cache.json` holds.
fn cached_kinds(dir: &Path) -> usize {
    let raw = std::fs::read_to_string(dir.join(".specforge/schema-cache.json")).unwrap();
    let cache: Value = serde_json::from_str(&raw).unwrap();
    cache["schema"]["entity_kinds"].as_array().unwrap().len()
}

#[test]
fn schema_cache_root_today() {
    let tmp = rv1();
    let root = tmp.path();
    let first = cli(&["export", s(root), "--format", "graph"]);
    assert_eq!(first.code, Some(0), "{}", first.stderr);
    assert_eq!(cached_kinds(root), 5);

    // Exporting the sub-path compiles it without the project's config (no
    // extension), yet versions against and records in the project's cache.
    let sub = cli(&["export", s(&root.join("spec")), "--format", "graph"]);
    assert_eq!(sub.code, Some(0), "{}", sub.stderr);
    assert_eq!(sub.stderr.matches("W053").count(), 20, "{}", sub.stderr);
    assert_eq!(cached_kinds(root), 0);
    assert!(!root.join("spec/.specforge").exists());
}
