//! Check-phase extension passes: a pass declared with `phase: "check"`
//! runs inside every compile (one-shot and session), in its declared
//! order, and what it reports joins the compile's diagnostics; the build
//! cache is handed to it as `previous`.

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use specforge_common::{Diagnostic, Severity};
use specforge_project::{CompiledProject, ProjectSession, SourceChange};
use specforge_test::prelude::*;
use specforge_wasm::{WasmCallResult, WasmRuntime, WasmTrapInfo};
use tempfile::TempDir;

const EXT: &str = "@test/passes";

/// An extension, in process, that declares the `gadget` kind and five
/// passes, deliberately out of order:
/// - `second` (check, after `first`) and `first` (check) report nothing;
/// - `audit` (check) reports E951 for every gadget whose id starts with
///   `bad`, naming the entity and giving no span, and W952 with an explicit
///   span-less, entity-less message;
/// - `boom` (check) traps;
/// - `report` (no phase) reports A999 for every gadget.
///
/// Every pass call is recorded with its input, oldest first.
pub(crate) struct PassesExtension {
    calls: Mutex<Vec<(String, Value)>>,
}

impl PassesExtension {
    pub(crate) fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
        }
    }

    fn passes() -> Value {
        json!([
            { "name": "second", "after": "first", "phase": "check" },
            { "name": "audit", "after": "resolve", "phase": "check" },
            { "name": "report", "after": "resolve" },
            { "name": "boom", "phase": "check" },
            { "name": "first", "phase": "check" }
        ])
    }

    /// The pass exports called so far, oldest first.
    fn called(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|(export, _)| export.clone())
            .collect()
    }

    /// The input the last call to `export` received.
    fn last_input(&self, export: &str) -> Value {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(e, _)| e == export)
            .map(|(_, input)| input.clone())
            .unwrap_or_else(|| panic!("{export} was never called"))
    }

    fn clear(&self) {
        self.calls.lock().unwrap().clear();
    }
}

fn gadgets(input: &Value) -> Vec<String> {
    input["entities"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["kind"] == "gadget")
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect()
}

impl WasmRuntime for PassesExtension {
    fn load_module(&self, _: &Path) -> Result<(), String> {
        Ok(())
    }

    fn call_export(&self, extension: &str, export: &str, input: &[u8]) -> WasmCallResult {
        let ok = |value: Value| WasmCallResult::Ok(value.to_string().into_bytes());
        let trap = |kind: &str| {
            WasmCallResult::Trap(WasmTrapInfo {
                kind: kind.to_string(),
                message: format!("{kind}: {export}"),
                export_name: export.to_string(),
            })
        };
        if extension != EXT {
            return trap("extension_not_found");
        }
        match export {
            "__handshake" => ok(json!({
                "protocol_version": "1.0.0",
                "name": EXT,
                "version": "1.0.0",
                "contribution_flags": { "entities": true },
                "peer_dependencies": [],
                "sandbox_policy": null
            })),
            "__describe" => {
                let request: Value = serde_json::from_slice(input).unwrap();
                let category = request["category"].as_str().unwrap();
                let items = match category {
                    // `docs` names files (a file reference field): the
                    // checks report a missing one (E016).
                    "entities" => json!([{
                        "name": "gadget",
                        "keyword": "gadget",
                        "fields": [{ "name": "docs", "field_type": "string_list", "file_reference": true }]
                    }]),
                    "passes" => Self::passes(),
                    _ => json!([]),
                };
                ok(json!({ "category": category, "items": items }))
            }
            pass if pass.starts_with("__pass_") => {
                let input: Value = serde_json::from_slice(input).unwrap();
                self.calls
                    .lock()
                    .unwrap()
                    .push((pass.to_string(), input.clone()));
                match pass {
                    "__pass_audit" => {
                        let mut diagnostics: Vec<Value> = gadgets(&input)
                            .into_iter()
                            .filter(|id| id.starts_with("bad"))
                            .map(|id| {
                                json!({
                                    "code": "E951", "severity": "Error",
                                    "message": format!("gadget '{id}' fails the audit"),
                                    "entity": id
                                })
                            })
                            .collect();
                        diagnostics.push(json!({
                            "code": "W952", "severity": "Warning",
                            "message": "the audit ran"
                        }));
                        ok(json!({ "diagnostics": diagnostics, "summary": { "audited": true } }))
                    }
                    "__pass_report" => ok(Value::Array(
                        gadgets(&input)
                            .into_iter()
                            .map(|id| {
                                json!({
                                    "code": "A999", "severity": "Info",
                                    "message": format!("gadget '{id}' reported")
                                })
                            })
                            .collect(),
                    )),
                    "__pass_boom" => trap("unreachable"),
                    _ => ok(json!([])),
                }
            }
            _ => trap("export_not_found"),
        }
    }
}

pub(crate) fn project(spec: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let config = json!({ "name": "p", "version": "0.1.0", "extensions": [EXT] });
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    fs::write(dir.path().join("a.spec"), spec).unwrap();
    dir
}

pub(crate) const SPEC: &str =
    "gadget good \"Good\" {\n  status active\n}\n\ngadget bad_one \"Bad\" {\n  status retired\n}\n";

fn with_code<'a>(diagnostics: &'a [Diagnostic], code: &str) -> Vec<&'a Diagnostic> {
    diagnostics.iter().filter(|d| d.code == code).collect()
}

#[specforge_test(
    behavior = "run_check_phase_passes",
    verify = "a pass declared for the check phase runs on every compile"
)]
fn a_check_pass_runs_on_every_compile() {
    let dir = project(SPEC);
    let ext = PassesExtension::new();

    let compiled = CompiledProject::compile(dir.path(), Some(&ext));

    let diagnostics = compiled.diagnostics();
    let audit = with_code(&diagnostics, "E951");
    assert_eq!(audit.len(), 1, "{diagnostics:?}");
    assert_eq!(audit[0].severity, Severity::Error);
    assert_eq!(audit[0].message, "gadget 'bad_one' fails the audit");
    let ran = with_code(&diagnostics, "W952");
    assert_eq!(ran.len(), 1, "{diagnostics:?}");
    assert_eq!(ran[0].severity, Severity::Warning);
    assert_eq!(
        gadgets(&ext.last_input("__pass_audit")),
        ["bad_one", "good"],
        "the pass sees the compiled entities"
    );

    // A second compile runs it again.
    ext.clear();
    let again = CompiledProject::compile(dir.path(), Some(&ext)).diagnostics();
    assert_eq!(with_code(&again, "E951").len(), 1, "{again:?}");
    assert!(ext.called().contains(&"__pass_audit".to_string()));
}

#[specforge_test(
    behavior = "run_check_phase_passes",
    verify = "a session reports a check pass's diagnostics after an update"
)]
fn a_session_reports_check_pass_diagnostics_after_an_update() {
    let dir = project("gadget good \"Good\" {\n}\n");
    let ext = Arc::new(PassesExtension::new());
    let mut session = ProjectSession::open_with_runtime(
        dir.path(),
        Some(Arc::clone(&ext) as specforge_project::SharedRuntime),
    );
    assert!(with_code(&session.diagnostics(), "E951").is_empty());

    let update = session.update(SourceChange::Buffer {
        path: "a.spec",
        text: Some("gadget good \"Good\" {\n}\n\ngadget bad_two \"Bad\" {\n}\n"),
    });

    let audit = with_code(&update.diagnostics, "E951");
    assert_eq!(audit.len(), 1, "{:?}", update.diagnostics);
    assert_eq!(audit[0].message, "gadget 'bad_two' fails the audit");
    // The session reports what a fresh compile of the same sources does.
    fs::write(
        dir.path().join("a.spec"),
        "gadget good \"Good\" {\n}\n\ngadget bad_two \"Bad\" {\n}\n",
    )
    .unwrap();
    let fresh = CompiledProject::compile(dir.path(), Some(ext.as_ref())).diagnostics();
    assert_eq!(as_set(&update.diagnostics), as_set(&fresh));
}

/// Diagnostics as their sorted full JSON: order-independent.
fn as_set(diagnostics: &[Diagnostic]) -> Vec<String> {
    let mut set: Vec<String> = diagnostics
        .iter()
        .map(|d| serde_json::to_string(d).unwrap())
        .collect();
    set.sort();
    set
}

#[specforge_test(
    behavior = "run_check_phase_passes",
    verify = "a pass without the check phase runs only under analyze"
)]
fn a_pass_without_the_check_phase_runs_only_under_analyze() {
    let dir = project(SPEC);
    let ext = PassesExtension::new();

    let compiled = CompiledProject::compile(dir.path(), Some(&ext));
    assert!(
        !ext.called().contains(&"__pass_report".to_string()),
        "{:?}",
        ext.called()
    );
    assert!(with_code(&compiled.diagnostics(), "A999").is_empty());

    ext.clear();
    let registries = &compiled.env.registries;
    let ctx = specforge_project::passes::AnalysisContext {
        graph: &compiled.graph,
        kind_registry: &registries.kinds,
        field_registry: &registries.fields,
        rules: &registries.rules,
        project_root: Some(dir.path()),
        test_results: None,
        proved_claims: None,
    };
    let reports =
        specforge_project::passes::run_extension_passes(&registries.manifests, &ctx, &ext, "all");
    let names: Vec<&str> = reports.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["@test/passes:report"], "analyze skips check passes");
    assert_eq!(reports[0].findings.len(), 2);
    assert_eq!(ext.called(), ["__pass_report"]);
}

#[specforge_test(
    behavior = "run_check_phase_passes",
    verify = "check passes run in their declared after/before order"
)]
fn check_passes_run_in_their_declared_order() {
    let dir = project(SPEC);
    let ext = PassesExtension::new();

    CompiledProject::compile(dir.path(), Some(&ext));

    // `second` is declared first but runs after `first`; the others keep
    // their declaration order.
    assert_eq!(
        ext.called(),
        [
            "__pass_audit",
            "__pass_boom",
            "__pass_first",
            "__pass_second"
        ]
    );
}

#[specforge_test(
    behavior = "run_check_phase_passes",
    verify = "a trapping check pass is a diagnostic, not a crash"
)]
fn a_trapping_check_pass_is_a_diagnostic() {
    let dir = project(SPEC);
    let ext = PassesExtension::new();

    let diagnostics = CompiledProject::compile(dir.path(), Some(&ext)).diagnostics();

    let failures: Vec<&Diagnostic> = with_code(&diagnostics, "E028")
        .into_iter()
        .filter(|d| d.message.contains("'@test/passes:boom'"))
        .collect();
    assert_eq!(failures.len(), 1, "{diagnostics:?}");
    assert_eq!(failures[0].severity, Severity::Error);
    assert!(
        failures[0].message.contains("unreachable"),
        "{}",
        failures[0].message
    );
    // The passes after it still ran.
    assert!(ext.called().contains(&"__pass_second".to_string()));
}

#[specforge_test(
    behavior = "run_check_phase_passes",
    verify = "a pass diagnostic naming an entity gets that entity's span"
)]
fn a_pass_diagnostic_naming_an_entity_gets_its_span() {
    let dir = project(SPEC);
    let ext = PassesExtension::new();

    let compiled = CompiledProject::compile(dir.path(), Some(&ext));

    let diagnostics = compiled.diagnostics();
    let audit = with_code(&diagnostics, "E951");
    let entity_span = compiled.graph.node("bad_one").unwrap().source_span.clone();
    assert_eq!(audit[0].span.as_ref(), Some(&entity_span));
    assert_eq!(entity_span.file.as_str(), "a.spec");
    // A diagnostic that names no entity keeps no span.
    assert_eq!(with_code(&diagnostics, "W952")[0].span, None);
}

// ── the build cache, read ──────────────────────────────────────────────────

pub(crate) fn write_cache(dir: &TempDir, text: &str) {
    fs::write(dir.path().join(specforge_project::BUILD_CACHE_FILE), text).unwrap();
}

#[specforge_test(
    behavior = "read_build_cache",
    verify = "check passes receive the cached statuses as previous"
)]
fn check_passes_receive_the_cached_statuses() {
    let dir = project(SPEC);
    write_cache(
        &dir,
        r#"{"format": 1, "statuses": {
            "good": {"kind": "gadget", "status": "proposed"},
            "gone": {"kind": "gadget", "status": "done"}
        }}"#,
    );
    let ext = PassesExtension::new();

    let diagnostics = CompiledProject::compile(dir.path(), Some(&ext)).diagnostics();

    let expected = json!({
        "statuses": {
            "gone": {"kind": "gadget", "status": "done"},
            "good": {"kind": "gadget", "status": "proposed"}
        }
    });
    for pass in ["__pass_audit", "__pass_first", "__pass_second"] {
        assert_eq!(ext.last_input(pass)["previous"], expected, "{pass}");
    }
    assert!(
        with_code(&diagnostics, "W144").is_empty(),
        "{diagnostics:?}"
    );

    // A session reads it too, and sees a cache written after it opened.
    let ext = Arc::new(PassesExtension::new());
    let mut session = ProjectSession::open_with_runtime(
        dir.path(),
        Some(Arc::clone(&ext) as specforge_project::SharedRuntime),
    );
    assert_eq!(ext.last_input("__pass_audit")["previous"], expected);
    write_cache(
        &dir,
        r#"{"format": 1, "statuses": {"good": {"kind": "gadget", "status": "active"}}}"#,
    );
    session.update(SourceChange::Disk(&["a.spec".to_string()]));
    assert_eq!(
        ext.last_input("__pass_audit")["previous"]["statuses"]["good"]["status"],
        "active"
    );
}

#[specforge_test(
    behavior = "read_build_cache",
    verify = "without a cache file previous is absent"
)]
fn without_a_cache_previous_is_absent() {
    let dir = project(SPEC);
    let ext = PassesExtension::new();

    let diagnostics = CompiledProject::compile(dir.path(), Some(&ext)).diagnostics();

    let input = ext.last_input("__pass_audit");
    assert!(input.get("previous").is_none(), "{input}");
    assert!(
        with_code(&diagnostics, "W144").is_empty(),
        "{diagnostics:?}"
    );
}

#[specforge_test(
    behavior = "read_build_cache",
    verify = "an invalid cache file is W144 and previous is absent"
)]
fn an_invalid_cache_is_w144() {
    for (text, problem) in [
        ("{ not json", "does not parse"),
        (r#"{"statuses": {}}"#, "does not parse"),
        (r#"{"format": 2, "statuses": {}}"#, "declares format 2"),
    ] {
        let dir = project(SPEC);
        write_cache(&dir, text);
        let ext = PassesExtension::new();

        let diagnostics = CompiledProject::compile(dir.path(), Some(&ext)).diagnostics();

        let warnings = with_code(&diagnostics, "W144");
        assert_eq!(warnings.len(), 1, "{text}: {diagnostics:?}");
        assert_eq!(warnings[0].severity, Severity::Warning);
        assert!(
            warnings[0].message.contains(problem),
            "{}",
            warnings[0].message
        );
        assert!(ext.last_input("__pass_audit").get("previous").is_none());
        // The passes still ran.
        assert_eq!(with_code(&diagnostics, "E951").len(), 1);
    }

    // With no check pass loaded, the file is not read.
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.spec"), SPEC).unwrap();
    write_cache(&dir, "{ not json");
    let diagnostics = CompiledProject::compile(dir.path(), None).diagnostics();
    assert!(
        with_code(&diagnostics, "W144").is_empty(),
        "{diagnostics:?}"
    );
}
