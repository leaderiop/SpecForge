//! CLI↔MCP parity harness (architecture plan 03, step O0).
//!
//! Each scenario runs one user-level operation twice on identical fixture
//! projects: once through the `specforge` binary, once through an in-process
//! MCP server. It then compares what the operation *did*, not the JSON it
//! answered with (the spec gives each surface its own result types):
//!
//! - `Outcome`: whether the operation succeeded;
//! - `Files`:   every file under the project after the operation, except
//!   `specforge.json`;
//! - `Config`:  `specforge.json` after the operation, parsed;
//! - `Check`:   what a fresh `specforge check` reports afterwards.
//!
//! Today's differences are listed in [`EXPECTED_DIVERGENCES`]. A scenario
//! fails when an aspect differs without a row, and also when a row's aspect
//! no longer differs: fixing a divergence means deleting its row. The
//! surfaces' payloads, and everything above, are pinned per surface as
//! insta snapshots (`tests/snapshots/tests__parity__*.snap`).

use serde_json::{Value, json};
use specforge_mcp::McpServer;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Aspect {
    Outcome,
    Files,
    Config,
    Check,
}

const ASPECTS: [Aspect; 4] = [
    Aspect::Outcome,
    Aspect::Files,
    Aspect::Config,
    Aspect::Check,
];

/// Where the CLI and MCP behave differently today, and why. Later steps of
/// plan 03 delete rows; only spec-sanctioned differences should remain.
const EXPECTED_DIVERGENCES: &[(&str, Aspect, &str)] = &[
    // export: both surfaces export through `specforge_ops::export` (O2), but
    // only the CLI keeps `.specforge/schema-cache.json` for its W053 check.
    // The MCP export tool is a read-only query with nowhere to show W053: if
    // it rewrote the cache, the next CLI export would miss the warning.
    (
        "export",
        Aspect::Files,
        "only the CLI writes the schema cache",
    ),
];

// ── fixtures ────────────────────────────────────────────────────────────────

/// The registry every fixture points at: nothing listens on port 9, so a
/// surface that goes to the network fails at once instead of timing out.
const CONFIG: &str = r#"{
  "name": "demo",
  "version": "0.1.0",
  "spec_root": "spec",
  "extensions": ["@specforge/software"],
  "registries": [
    { "alias": "offline", "url": "http://127.0.0.1:9/v1", "default_registry": true }
  ]
}
"#;

const MAIN_SPEC: &str =
    "behavior alpha \"Alpha\" {\n  category \"core\"\n  contract \"The system MUST work\"\n}\n";

/// A third-party extension (`@sdk/greet` 0.1.0, contributing `greeting`).
fn greet_blob() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/greet-extension/greet.wasm")
        .canonicalize()
        .expect("the greet fixture is vendored")
}

const GREET: &str = "@sdk/greet";

fn project(root: &Path) {
    std::fs::create_dir_all(root.join("spec")).unwrap();
    std::fs::write(root.join("specforge.json"), CONFIG).unwrap();
    std::fs::write(root.join("spec/main.spec"), MAIN_SPEC).unwrap();
}

fn empty(_root: &Path) {}

fn project_with_product_enabled(root: &Path) {
    project(root);
    let config = CONFIG.replace(
        r#"["@specforge/software"]"#,
        r#"["@specforge/software", "@specforge/product"]"#,
    );
    std::fs::write(root.join("specforge.json"), config).unwrap();
}

fn project_with_greet_installed(root: &Path) {
    project(root);
    let out = cli()
        .args(["add", greet_blob().to_str().unwrap(), "--path"])
        .arg(root)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success(), "setup install failed: {out:?}");
}

fn project_unformatted(root: &Path) {
    project(root);
    std::fs::write(
        root.join("spec/messy.spec"),
        "behavior messy \"Messy\" {\ncategory \"core\"\ncontract \"The system MUST work\"\n}\n",
    )
    .unwrap();
}

fn project_at_old_format(root: &Path) {
    project(root);
    std::fs::write(
        root.join("spec/old.spec"),
        "// specforge-format: 0.1\nbehavior old_one \"Old\" {\n  category \"core\"\n  contract \"The system MUST work\"\n}\n",
    )
    .unwrap();
}

// ── scenarios ───────────────────────────────────────────────────────────────

struct Scenario {
    name: &'static str,
    setup: fn(&Path),
    /// CLI arguments (after `specforge`); run with the project as cwd.
    cli: fn(&Path) -> Vec<String>,
    /// MCP tool name and arguments.
    mcp: fn(&Path) -> (&'static str, Value),
    /// Whether the MCP server is started on the project. `init` targets a
    /// directory that isn't a project yet.
    mcp_rooted: bool,
}

fn s(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|a| a.to_string()).collect()
}

const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "init",
        setup: empty,
        cli: |_| args(&["init", "--name", "demo", "--format", "json"]),
        mcp: |root| ("specforge.init", json!({"path": s(root), "name": "demo"})),
        mcp_rooted: false,
    },
    Scenario {
        name: "add_builtin",
        setup: project,
        cli: |root| {
            args(&[
                "add",
                "@specforge/product",
                "--path",
                &s(root),
                "--format",
                "json",
            ])
        },
        mcp: |_| {
            (
                "specforge.add_extension",
                json!({"specifier": "@specforge/product"}),
            )
        },
        mcp_rooted: true,
    },
    Scenario {
        name: "add_local",
        setup: project,
        cli: |root| {
            args(&[
                "add",
                &s(&greet_blob()),
                "--path",
                &s(root),
                "--format",
                "json",
            ])
        },
        mcp: |_| {
            (
                "specforge.add_extension",
                json!({"specifier": s(&greet_blob())}),
            )
        },
        mcp_rooted: true,
    },
    Scenario {
        name: "remove_builtin",
        setup: project_with_product_enabled,
        cli: |root| {
            args(&[
                "remove",
                "@specforge/product",
                "--path",
                &s(root),
                "--format",
                "json",
            ])
        },
        mcp: |_| {
            (
                "specforge.remove_extension",
                json!({"name": "@specforge/product"}),
            )
        },
        mcp_rooted: true,
    },
    Scenario {
        name: "remove_installed",
        setup: project_with_greet_installed,
        cli: |root| args(&["remove", GREET, "--path", &s(root), "--format", "json"]),
        mcp: |_| ("specforge.remove_extension", json!({"name": GREET})),
        mcp_rooted: true,
    },
    Scenario {
        name: "extensions",
        setup: project_with_greet_installed,
        cli: |root| args(&["extensions", "--path", &s(root), "--format", "json"]),
        mcp: |_| ("specforge.extensions", json!({})),
        mcp_rooted: true,
    },
    Scenario {
        name: "providers",
        setup: project,
        cli: |root| args(&["providers", "--path", &s(root), "--format", "json"]),
        mcp: |_| ("specforge.providers", json!({})),
        mcp_rooted: true,
    },
    Scenario {
        name: "doctor",
        setup: project,
        cli: |root| args(&["doctor", "--path", &s(root), "--format", "json"]),
        mcp: |_| ("specforge.doctor", json!({})),
        mcp_rooted: true,
    },
    Scenario {
        name: "format",
        setup: project_unformatted,
        cli: |root| args(&["format", "--path", &s(root)]),
        mcp: |root| ("specforge.format", json!({"path": s(root)})),
        mcp_rooted: true,
    },
    Scenario {
        name: "migrate",
        setup: project_at_old_format,
        cli: |root| args(&["migrate", "--path", &s(root), "--format", "json"]),
        mcp: |root| ("specforge.migrate", json!({"path": s(root)})),
        mcp_rooted: true,
    },
    Scenario {
        name: "export",
        setup: project,
        cli: |root| args(&["export", &s(root), "--format", "graph"]),
        mcp: |_| ("specforge.export", json!({"format": "graph"})),
        mcp_rooted: true,
    },
    Scenario {
        name: "analyze",
        setup: project,
        cli: |root| args(&["analyze", "--path", &s(root), "--json"]),
        mcp: |root| ("specforge.analyze", json!({"path": s(root)})),
        mcp_rooted: true,
    },
];

// ── running a surface ───────────────────────────────────────────────────────

/// What one surface did to a project.
struct Observed {
    ok: bool,
    payload: Value,
    files: BTreeMap<String, String>,
    config: Value,
    check: Value,
}

fn cli() -> Command {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("specforge"));
    // No user credentials or caches leak into the snapshots.
    cmd.env("HOME", std::env::temp_dir().join("specforge-parity-home"));
    cmd
}

fn run_cli(scenario: &Scenario, root: &Path) -> (bool, Value) {
    let out = cli()
        .args((scenario.cli)(root))
        .current_dir(root)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let payload = serde_json::from_str(&stdout).unwrap_or(Value::String(stdout));
    (out.status.success(), payload)
}

fn run_mcp(scenario: &Scenario, root: &Path) -> (bool, Value) {
    let mut server = if scenario.mcp_rooted {
        McpServer::with_project_root(root.to_path_buf())
    } else {
        McpServer::new()
    };
    let init = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}});
    server.handle_message(&init.to_string());
    let (tool, arguments) = (scenario.mcp)(root);
    let req = json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": tool, "arguments": arguments}
    });
    let resp: Value = serde_json::from_str(&server.handle_message(&req.to_string()).unwrap())
        .expect("the server answers JSON");
    if let Some(error) = resp.get("error") {
        return (false, json!({"error": error}));
    }
    let result = &resp["result"];
    let text = result["content"][0]["text"].as_str().unwrap_or_default();
    let payload = serde_json::from_str(text).unwrap_or(Value::String(text.to_string()));
    (result["isError"] != true, payload)
}

/// Every file under `root` but `specforge.json`, by relative path. Binary
/// files are named by size.
fn files_under(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(root, &path, out);
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .to_string();
            if rel == "specforge.json" {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            let content = match String::from_utf8(bytes) {
                Ok(text) => text,
                Err(e) => format!("<binary, {} bytes>", e.as_bytes().len()),
            };
            out.insert(rel, content);
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn read_config(root: &Path) -> Value {
    std::fs::read_to_string(root.join("specforge.json"))
        .map(|text| serde_json::from_str(&text).unwrap_or(Value::String(text)))
        .unwrap_or(Value::Null)
}

/// The codes a fresh `specforge check` reports, sorted, and its exit status.
fn check(root: &Path) -> Value {
    let out = cli()
        .args(["check", "--format", "json"])
        .arg(root)
        .current_dir(root)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut codes: Vec<String> = serde_json::from_str::<Value>(&stdout)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .map(|d| {
            format!(
                "{} {}",
                d["severity"].as_str().unwrap_or("?"),
                d["code"].as_str().unwrap_or("?")
            )
        })
        .collect();
    codes.sort();
    json!({"ok": out.status.success(), "diagnostics": codes})
}

fn observe(
    scenario: &Scenario,
    root: &Path,
    run: fn(&Scenario, &Path) -> (bool, Value),
) -> Observed {
    (scenario.setup)(root);
    let (ok, payload) = run(scenario, root);
    let files = files_under(root);
    let config = read_config(root);
    let check = check(root);
    Observed {
        ok,
        payload,
        files,
        config,
        check,
    }
}

/// `value` with the parts that churn for reasons outside this harness
/// redacted: the embedded Graph Protocol schema (any extension change moves
/// it), and whether z3 is installed on this machine.
fn redacted(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let v = match k.as_str() {
                        "schema" if v.is_object() => json!("[SCHEMA]"),
                        "z3_available" => json!("[Z3]"),
                        _ => redacted(v),
                    };
                    (k.clone(), v)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redacted).collect()),
        other => other.clone(),
    }
}

/// `text` with every 64-digit hex run (a SHA-256: blob hashes move whenever
/// an extension is rebuilt) replaced by `[SHA256]`.
fn without_hashes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_hexdigit() {
            run.push(c);
            continue;
        }
        if run.len() == 64 {
            out.push_str("[SHA256]");
        } else {
            out.push_str(&run);
        }
        run.clear();
        out.push(c);
    }
    out.pop();
    out
}

/// `value` as pretty JSON with the machine-specific parts replaced.
fn normalized(value: &Value, root: &Path) -> String {
    let mut text = without_hashes(&serde_json::to_string_pretty(&redacted(value)).unwrap());
    let blob = s(&greet_blob());
    text = text.replace(&blob, "[BLOB]");
    let canonical = s(&root.canonicalize().unwrap());
    text = text.replace(&canonical, "[ROOT]");
    text = text.replace(&s(root), "[ROOT]");
    text
}

fn snapshot_of(o: &Observed, root: &Path) -> String {
    let files: BTreeMap<&String, Value> = o
        .files
        .iter()
        .map(|(path, content)| {
            // Binary files by kind only: blob sizes move on every rebuild.
            let shown = if content.starts_with("<binary,") {
                "<binary>".to_string()
            } else if path.ends_with("schema-cache.json") {
                "[SCHEMA]".to_string()
            } else {
                content.clone()
            };
            (path, Value::String(shown))
        })
        .collect();
    let doc = json!({
        "ok": o.ok,
        "payload": o.payload,
        "files": files,
        "config": o.config,
        "check": o.check,
    });
    normalized(&doc, root)
}

fn differs(aspect: Aspect, cli: &Observed, mcp: &Observed) -> bool {
    match aspect {
        Aspect::Outcome => cli.ok != mcp.ok,
        Aspect::Files => cli.files != mcp.files,
        Aspect::Config => cli.config != mcp.config,
        Aspect::Check => cli.check != mcp.check,
    }
}

fn parity(name: &str) {
    let scenario = SCENARIOS
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no scenario {name}"));
    let cli_dir = tempfile::tempdir().unwrap();
    let mcp_dir = tempfile::tempdir().unwrap();
    let cli_obs = observe(scenario, cli_dir.path(), run_cli);
    let mcp_obs = observe(scenario, mcp_dir.path(), run_mcp);

    let cli_snap = snapshot_of(&cli_obs, cli_dir.path());
    let mcp_snap = snapshot_of(&mcp_obs, mcp_dir.path());
    insta::assert_snapshot!(format!("{name}__cli"), cli_snap);
    insta::assert_snapshot!(format!("{name}__mcp"), mcp_snap);

    let mut unexpected = Vec::new();
    for aspect in ASPECTS {
        let expected = EXPECTED_DIVERGENCES
            .iter()
            .any(|(s, a, _)| *s == name && *a == aspect);
        let actual = differs(aspect, &cli_obs, &mcp_obs);
        if actual && !expected {
            unexpected.push(format!(
                "{aspect:?} differs but has no EXPECTED_DIVERGENCES row"
            ));
        }
        if !actual && expected {
            unexpected.push(format!(
                "{aspect:?} no longer differs: delete its EXPECTED_DIVERGENCES row"
            ));
        }
    }
    assert!(
        unexpected.is_empty(),
        "{name}:\n  {}\n--- cli ---\n{cli_snap}\n--- mcp ---\n{mcp_snap}",
        unexpected.join("\n  ")
    );
}

#[test]
fn every_expected_divergence_names_a_scenario() {
    for (name, aspect, _) in EXPECTED_DIVERGENCES {
        assert!(
            SCENARIOS.iter().any(|s| s.name == *name),
            "EXPECTED_DIVERGENCES row ({name}, {aspect:?}) names no scenario"
        );
    }
}

#[test]
fn parity_init() {
    parity("init");
}

#[test]
fn parity_add_builtin() {
    parity("add_builtin");
}

#[test]
fn parity_add_local() {
    parity("add_local");
}

#[test]
fn parity_remove_builtin() {
    parity("remove_builtin");
}

#[test]
fn parity_remove_installed() {
    parity("remove_installed");
}

#[test]
fn parity_extensions() {
    parity("extensions");
}

#[test]
fn parity_providers() {
    parity("providers");
}

#[test]
fn parity_doctor() {
    parity("doctor");
}

#[test]
fn parity_format() {
    parity("format");
}

#[test]
fn parity_migrate() {
    parity("migrate");
}

#[test]
fn parity_export() {
    parity("export");
}

#[test]
fn parity_analyze() {
    parity("analyze");
}

// ── export: one function, one schema policy (O2, ADR 0004 D3-a) ─────────────

/// An initialized MCP server on `root`.
fn mcp_on(root: &Path) -> McpServer {
    let mut server = McpServer::with_project_root(root.to_path_buf());
    let init = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}});
    server.handle_message(&init.to_string());
    server
}

/// The JSON document an MCP request answered with: a tool's text content,
/// or a resource's.
fn mcp_document(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc": "2.0", "id": 2, "method": method, "params": params});
    let resp: Value =
        serde_json::from_str(&server.handle_message(&req.to_string()).unwrap()).unwrap();
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .or_else(|| resp["result"]["contents"][0]["text"].as_str())
        .unwrap_or_else(|| panic!("no document in {resp}"));
    serde_json::from_str(text).unwrap()
}

fn mcp_export(root: &Path, arguments: Value) -> Value {
    mcp_document(
        &mut mcp_on(root),
        "tools/call",
        json!({"name": "specforge.export", "arguments": arguments}),
    )
}

fn cli_export(root: &Path, flags: &[&str]) -> Value {
    let out = cli()
        .arg("export")
        .arg(root)
        .args(flags)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    serde_json::from_slice(&out.stdout).unwrap()
}

#[specforge_test_macros::test(
    behavior = "provide_mcp_export_tool",
    verify = "the graph export is the document specforge export --format graph writes, Graph Protocol 2.0 with the schema embedded"
)]
fn mcp_graph_export_is_the_cli_export() {
    let dir = tempfile::tempdir().unwrap();
    project(dir.path());

    let mcp = mcp_export(dir.path(), json!({"format": "graph"}));
    let cli = cli_export(dir.path(), &["--format", "graph"]);

    assert_eq!(mcp["format_version"], "2.0", "{mcp}");
    assert!(mcp["schema"].is_object(), "{mcp}");
    assert_eq!(mcp, cli);
}

#[specforge_test_macros::test(
    behavior = "provide_mcp_export_tool",
    verify = "with_schema embeds the schema in a context, brief or budgeted export, and no_schema leaves it out of a graph export"
)]
fn mcp_export_schema_flags_are_the_cli_flags() {
    let dir = tempfile::tempdir().unwrap();
    project(dir.path());

    for format in ["context", "brief"] {
        let plain = mcp_export(dir.path(), json!({"format": format}));
        assert!(plain.get("schema").is_none(), "{format}: {plain}");
        let with = mcp_export(dir.path(), json!({"format": format, "with_schema": true}));
        assert!(with["schema"].is_object(), "{format}: {with}");
        assert_eq!(
            with,
            cli_export(dir.path(), &["--format", format, "--with-schema"])
        );
    }

    let budgeted = mcp_export(
        dir.path(),
        json!({"format": "graph", "max_tokens": 100000, "with_schema": true}),
    );
    assert!(budgeted["schema"].is_object(), "{budgeted}");

    let without = mcp_export(dir.path(), json!({"format": "graph", "no_schema": true}));
    assert_eq!(without["format_version"], "1.0", "{without}");
    assert!(without.get("schema").is_none(), "{without}");
    assert_eq!(
        without,
        cli_export(dir.path(), &["--format", "graph", "--no-schema"])
    );
}

#[specforge_test_macros::test(
    behavior = "serve_graph_resource",
    verify = "specforge://graph under max_tokens stays within the budget, as the budgeted export does"
)]
fn graph_resource_under_a_budget_is_the_budgeted_export() {
    let dir = tempfile::tempdir().unwrap();
    project(dir.path());
    let budget = 300;

    let resource = mcp_document(
        &mut mcp_on(dir.path()),
        "resources/read",
        json!({"uri": format!("specforge://graph?max_tokens={budget}")}),
    );

    // The schema alone is thousands of tokens: a budgeted graph leaves it out.
    assert!(resource.get("schema").is_none(), "{resource}");
    let estimate = specforge_emitter::estimate_tokens(&resource.to_string());
    assert!(
        estimate <= budget,
        "{estimate} tokens > {budget}: {resource}"
    );
    assert_eq!(
        resource,
        cli_export(dir.path(), &["--format", "graph", "--max-tokens", "300"])
    );
}

// ── listings: one list per surface pair (O5) ────────────────────────────────

/// The CLI's `<command> --format json` and the MCP `tool`'s answer on the
/// same project.
fn both_listings(root: &Path, command: &str, tool: &str) -> (Value, Value) {
    let out = cli()
        .args([command, "--path"])
        .arg(root)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let cli: Value = serde_json::from_slice(&out.stdout).unwrap();
    let mcp = mcp_document(
        &mut mcp_on(root),
        "tools/call",
        json!({"name": tool, "arguments": {}}),
    );
    (cli, mcp)
}

#[specforge_test_macros::test(
    behavior = "list_installed_extensions",
    verify = "the CLI and the MCP extensions tool list the same entries"
)]
fn extensions_listings_match() {
    let dir = tempfile::tempdir().unwrap();
    project_with_greet_installed(dir.path());
    std::fs::write(
        dir.path().join("spec/hello.spec"),
        "greeting hello \"Hello\" {\n  style warm\n}\n",
    )
    .unwrap();

    let (cli, mcp) = both_listings(dir.path(), "extensions", "specforge.extensions");

    assert_eq!(cli["extensions"], mcp["extensions"]);
    let greet = &cli["extensions"][0];
    assert_eq!(greet["name"], GREET, "{cli}");
    assert_eq!(greet["status"], "loaded", "{cli}");
    assert_eq!(greet["entity_kinds"], json!(["greeting"]), "{cli}");
    assert_eq!(greet["entity_count"], 1, "{cli}");
}

#[specforge_test_macros::test(
    behavior = "list_configured_providers",
    verify = "the CLI and the MCP providers tool list the same entries"
)]
fn providers_listings_match() {
    let dir = tempfile::tempdir().unwrap();
    project(dir.path());
    let mut config: Value = serde_json::from_str(CONFIG).unwrap();
    config["providers"] = json!([
        {"scheme": "gh", "alias": "work", "extension": "@acme/github"},
        {"scheme": "file", "alias": "local", "extension": "@specforge/software"},
    ]);
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();

    let (cli, mcp) = both_listings(dir.path(), "providers", "specforge.providers");

    assert_eq!(cli, mcp);
    assert_eq!(
        cli["providers"],
        json!([
            {"scheme": "gh", "alias": "work", "extension": "@acme/github", "status": "extension_not_loaded"},
            {"scheme": "file", "alias": "local", "extension": "@specforge/software", "status": "not_a_provider"},
        ])
    );
}

// ── init: one scaffold (O7) ─────────────────────────────────────────────────

#[specforge_test_macros::test(
    behavior = "provide_mcp_init_tool",
    verify = "specforge.init writes the files and config specforge init writes for the same inputs"
)]
fn mcp_init_writes_what_cli_init_writes() {
    let cli_dir = tempfile::tempdir().unwrap();
    let mcp_dir = tempfile::tempdir().unwrap();
    let (cli_root, mcp_root) = (cli_dir.path().join("demo"), mcp_dir.path().join("demo"));
    std::fs::create_dir(&cli_root).unwrap();

    let out = cli()
        .args([
            "init",
            "--extensions",
            "@specforge/software,@specforge/product",
        ])
        .current_dir(&cli_root)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let mut server = McpServer::new();
    let init = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}});
    server.handle_message(&init.to_string());
    let reply = mcp_document(
        &mut server,
        "tools/call",
        json!({"name": "specforge.init", "arguments": {
            "path": s(&mcp_root),
            "extensions": ["@specforge/software", "@specforge/product"],
        }}),
    );
    assert_eq!(reply["starter_file"], "spec/hello.spec", "{reply}");

    let files = files_under(&cli_root);
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        [".gitignore", "spec/hello.spec"]
    );
    assert_eq!(files, files_under(&mcp_root));
    assert_eq!(read_config(&cli_root), read_config(&mcp_root));
    assert_eq!(check(&mcp_root)["ok"], true);
}
