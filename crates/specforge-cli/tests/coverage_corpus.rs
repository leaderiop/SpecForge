//! Coverage characterization corpus (architecture plan 02, step S1).
//!
//! Pins what `analyze coverage`, `stats` and the MCP coverage views report
//! today on one small project (`fixtures/coverage/fx1`) and on the shipped
//! `examples/todo-app`. The surfaces disagree; each disagreement is marked
//! with the step that removes it, and that step turns the assertion red on
//! purpose and rewrites it.
//!
//! fx1 holds:
//! - `login`: one obligation, proven by a passing test that names it;
//! - `logout`: no obligation, one passing test;
//! - `reset_password`: one obligation, named by its test only up to case
//!   (the slug matches, the text does not);
//! - `Status`: a union type;
//! - `Payload`: a struct field named `verify` *and* a verify statement the
//!   passing test names;
//! - `signin`: a feature (not testable) with one passing, unnamed test;
//! - `no_lost_login`: a formal property with one obligation and no test.
//!
//! These tests prove no spec obligation (they pin current behavior, bugs
//! included), so they carry no `specforge_test` link.

use assert_cmd::Command;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tempfile::TempDir;

fn specforge() -> Command {
    Command::new(env!("CARGO_BIN_EXE_specforge"))
}

fn fixture_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/coverage")
        .join(name)
}

fn copy_tree(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let dest = to.join(entry.file_name());
        if entry.path().is_dir() {
            std::fs::create_dir_all(&dest).unwrap();
            copy_tree(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), &dest).unwrap();
        }
    }
}

/// A scratch copy of a corpus project, so no surface writes into the repo.
fn project(name: &str) -> TempDir {
    let tmp = TempDir::new().unwrap();
    copy_tree(&fixture_dir(name), tmp.path());
    tmp
}

/// The `@specforge/testing:coverage` pass of `analyze coverage --json`.
fn analyze_coverage(root: &Path) -> Value {
    let out = specforge()
        .args([
            "analyze",
            "--path",
            root.to_str().unwrap(),
            "coverage",
            "--json",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("analyze --json is not JSON ({e}): {stdout}"));
    doc["passes"]
        .as_array()
        .and_then(|passes| {
            passes
                .iter()
                .find(|p| p["pass"] == "@specforge/testing:coverage")
        })
        .cloned()
        .unwrap_or_else(|| panic!("no coverage pass: {doc}"))
}

/// Each finding of a pass as `(code, message)`, sorted.
fn findings(pass: &Value) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = pass["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["code"].as_str().unwrap().to_string(),
                f["message"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    out.sort();
    out
}

fn stats(root: &Path) -> Value {
    let out = specforge()
        .args(["stats", "--format", "json", root.to_str().unwrap()])
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stats is not JSON ({e}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

/// Call MCP tools in one `specforge mcp` session; one result per call, in
/// order: the tool's JSON content, or the JSON-RPC error object.
fn mcp_calls(root: &Path, calls: &[Value]) -> Vec<Value> {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_specforge"))
        .arg("mcp")
        .arg(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdin = child.stdin.as_mut().unwrap();
    let init = json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
        "protocolVersion": "2025-03-26", "capabilities": {},
        "clientInfo": {"name": "corpus", "version": "0"}}});
    writeln!(stdin, "{init}").unwrap();
    for (i, call) in calls.iter().enumerate() {
        let req = json!({"jsonrpc": "2.0", "id": i + 1, "method": "tools/call", "params": call});
        writeln!(stdin, "{req}").unwrap();
    }
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    let responses: Vec<Value> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    (1..=calls.len())
        .map(|id| {
            let resp = responses
                .iter()
                .find(|r| r["id"] == id)
                .unwrap_or_else(|| panic!("no response {id}: {responses:?}"));
            if !resp["error"].is_null() {
                return json!({"error": resp["error"]});
            }
            let text = resp["result"]["content"][0]["text"].as_str().unwrap();
            serde_json::from_str(text).unwrap_or_else(|_| json!({"text": text}))
        })
        .collect()
}

/// `specforge.coverage` rows as `id -> (status, obligations, proven)`.
fn coverage_rows(content: &Value) -> BTreeMap<String, (String, u64, u64)> {
    content
        .as_array()
        .unwrap_or_else(|| panic!("coverage is not an array: {content}"))
        .iter()
        .map(|row| {
            (
                row["entity_id"].as_str().unwrap().to_string(),
                (
                    row["status"].as_str().unwrap().to_string(),
                    row["obligations"].as_u64().unwrap(),
                    row["proven"].as_u64().unwrap(),
                ),
            )
        })
        .collect()
}

#[test]
fn fx1_analyze_coverage_today() {
    let tmp = project("fx1");
    let pass = analyze_coverage(tmp.path());
    let summary = &pass["summary"];

    // Testable: login, logout, reset_password, Status and Payload, as in
    // stats. The formal property accepts verify statements but its kind is
    // not testable (S4).
    assert_eq!(summary["testable_total"], 5, "{summary}");
    assert_eq!(summary["testable_verified"], 3, "{summary}");
    assert_eq!(summary["obligations"], 4, "{summary}");
    assert_eq!(
        summary["test_results"]["obligations_proven"], 2,
        "{summary}"
    );
    // login, Payload, plus logout (zero obligations, passing test: D2-a,
    // S10) and the feature signin (not testable: S9).
    assert_eq!(
        summary["discharge_funnel"]["entities_proven"], 4,
        "{summary}"
    );

    let expected: Vec<(String, String)> = [
        ("A001", "behavior 'logout' declares no verify obligations"),
        // A union type can never hold obligations (D2-b, S10).
        ("A001", "type 'Status' declares no verify obligations"),
        (
            "A015",
            "behavior 'reset_password' has 1 obligation(s) no passing test proves: \"Reset link expires after one hour\"",
        ),
        (
            "A015",
            "property 'no_lost_login' has 1 obligation(s) no passing test proves: \"no login is lost\"",
        ),
        (
            "A016",
            "tests name obligation(s) behavior 'reset_password' does not declare: \"reset link expires after one hour\"",
        ),
    ]
    .iter()
    .map(|(c, m)| (c.to_string(), m.to_string()))
    .collect();
    assert_eq!(findings(&pass), expected);
}

#[test]
fn fx1_stats_today() {
    let tmp = project("fx1");
    let stats = stats(tmp.path());
    // Testable kinds: behavior and type (5 entities); the property is not.
    assert_eq!(stats["testable_count"], 5, "{stats}");
    // login, reset_password, Payload (its statement sits behind a struct
    // field named `verify`, S2) and the property, which counts toward
    // "verified" though its kind is not testable.
    assert_eq!(stats["verified_count"], 4, "{stats}");
    assert_eq!(stats["coverage_pct"], 60.0, "{stats}");
}

#[test]
fn fx1_mcp_coverage_today() {
    let tmp = project("fx1");
    let results = mcp_calls(
        tmp.path(),
        &[
            json!({"name": "specforge.coverage", "arguments": {}}),
            json!({"name": "specforge.inspect", "arguments": {"entity_id": "Payload"}}),
            json!({"name": "specforge.stats", "arguments": {}}),
        ],
    );

    let rows = coverage_rows(&results[0]);
    let row =
        |status: &str, obligations: u64, proven: u64| (status.to_string(), obligations, proven);
    let expected: BTreeMap<String, (String, u64, u64)> = [
        // Payload's statement sits behind its `verify` struct field (S2).
        ("Payload", row("covered", 1, 1)),
        ("Status", row("uncovered", 0, 0)),
        ("login", row("covered", 1, 1)),
        ("logout", row("uncovered", 0, 0)),
        ("reset_password", row("uncovered", 1, 0)),
    ]
    .into_iter()
    .map(|(id, r)| (id.to_string(), r))
    .collect();
    assert_eq!(rows, expected, "{}", results[0]);

    let inspect = &results[1];
    // `testable` reports "declares a verify list" (D2-d, S10).
    assert_eq!(inspect["testable"], true, "{inspect}");
    assert_eq!(
        inspect["verify_declarations"],
        json!(["unit Payload schema is valid"]),
        "{inspect}"
    );
    assert_eq!(inspect["coverage_status"], "covered", "{inspect}");

    assert_eq!(results[2]["coverage_pct"], 60.0, "{}", results[2]);
}

#[test]
fn todo_app_analyze_and_stats_today() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/todo-app");
    let pass = analyze_coverage(&example);
    // The same 18 testable entities stats counts; the formal property
    // no_lost_completion is not testable (S4).
    assert_eq!(pass["summary"]["testable_total"], 18, "{}", pass["summary"]);
    let a001: Vec<String> = findings(&pass)
        .into_iter()
        .filter(|(code, _)| code == "A001")
        .map(|(_, message)| message)
        .collect();
    assert_eq!(
        a001,
        [
            "failure_mode 'lost_task' declares no verify obligations",
            "type 'TaskStatus' declares no verify obligations",
        ]
    );

    let stats = stats(&example);
    assert_eq!(stats["testable_count"], 18, "{stats}");
    assert_eq!(stats["verified_count"], 16, "{stats}");
}
