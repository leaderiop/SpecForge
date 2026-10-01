//! The cross-validation of the extensions' fields against the kinds and
//! edge types they register, as the real load does it: W021 from the
//! manifest consistency check once every configured extension is in.

use std::fs;
use std::path::Path;

use specforge_common::{Diagnostic, Severity};
use specforge_project::Environment;
use specforge_test::prelude::*;
use specforge_wasm::{WasmCallResult, WasmRuntime, WasmTrapInfo};
use tempfile::TempDir;

/// In-process extensions that contribute entity kinds (with their fields)
/// and edge types. Each is `{ "name", "peers": [..], "entities": [..],
/// "edges": [..] }`, the descriptor JSON the protocol carries.
struct KindExtensions(Vec<serde_json::Value>);

impl KindExtensions {
    fn find(&self, name: &str) -> Option<&serde_json::Value> {
        self.0.iter().find(|e| e["name"] == name)
    }
}

impl WasmRuntime for KindExtensions {
    fn load_module(&self, _: &Path) -> Result<(), String> {
        Ok(())
    }

    fn call_export(&self, extension: &str, export: &str, input: &[u8]) -> WasmCallResult {
        let ok = |value: serde_json::Value| WasmCallResult::Ok(value.to_string().into_bytes());
        let Some(ext) = self.find(extension) else {
            return WasmCallResult::Trap(WasmTrapInfo {
                kind: "extension_not_found".to_string(),
                message: format!("Extension '{extension}' not loaded"),
                export_name: export.to_string(),
            });
        };
        match export {
            "__handshake" => {
                let peers: Vec<serde_json::Value> = ext["peers"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|peer| serde_json::json!({ "name": peer, "version": "^1.0.0" }))
                    .collect();
                ok(serde_json::json!({
                    "protocol_version": "1.0.0",
                    "name": extension,
                    "version": "1.0.0",
                    "contribution_flags": { "entities": true },
                    "peer_dependencies": peers,
                    "sandbox_policy": null
                }))
            }
            "__describe" => {
                let request: serde_json::Value = serde_json::from_slice(input).unwrap();
                let category = request["category"].as_str().unwrap();
                let items = match category {
                    "entities" => ext["entities"].clone(),
                    "edges" => ext["edges"].clone(),
                    _ => serde_json::Value::Null,
                };
                let items = if items.is_null() {
                    serde_json::json!([])
                } else {
                    items
                };
                ok(serde_json::json!({ "category": category, "items": items }))
            }
            other => WasmCallResult::Trap(WasmTrapInfo {
                kind: "guest_error".to_string(),
                message: format!("unknown export '{other}'"),
                export_name: other.to_string(),
            }),
        }
    }
}

/// Load a project configuring `extensions` (in order) from `runtime`.
fn load(runtime: &KindExtensions, extensions: &[&str]) -> Environment {
    let dir = TempDir::new().unwrap();
    let config = serde_json::json!({
        "name": "p", "version": "0.1.0", "extensions": extensions
    });
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    Environment::load(dir.path(), Some(runtime))
}

fn w021(env: &Environment) -> Vec<Diagnostic> {
    env.diagnostics()
        .filter(|d| d.code == "W021")
        .cloned()
        .collect()
}

/// `@test/tasks`: a `task` whose `owner` is a `person` over `owned_by`,
/// with `person` declared by its peer `@test/people`.
fn tasks_and_people() -> KindExtensions {
    KindExtensions(vec![
        serde_json::json!({
            "name": "@test/people",
            "entities": [{ "name": "person" }]
        }),
        serde_json::json!({
            "name": "@test/tasks",
            "peers": ["@test/people"],
            "entities": [{
                "name": "task",
                "fields": [{
                    "name": "owner", "field_type": "reference",
                    "target_kind": "person", "edge": "owned_by"
                }]
            }],
            "edges": [{ "label": "owned_by", "source_kind": "task", "target_kind": "person" }]
        }),
    ])
}

/// A field's `target_kind` naming a kind its peer registers is accepted.
#[specforge_test(
    behavior = "validate_registered_entity_fields",
    verify = "target_kind reference resolves to registered kind"
)]
fn a_target_kind_another_extension_registers_resolves() {
    let env = load(&tasks_and_people(), &["@test/people", "@test/tasks"]);

    assert!(env.registries.kinds.contains("person"));
    let owner = env.registries.fields.get("task", "owner").unwrap();
    assert_eq!(owner.target_kind.as_deref(), Some("person"));
    assert!(w021(&env).is_empty(), "{:?}", w021(&env));
}

/// A field's edge label naming an edge type the extension declares is
/// accepted.
#[specforge_test(
    behavior = "validate_registered_entity_fields",
    verify = "edge label resolves to registered edge type"
)]
fn an_edge_label_the_extension_declares_resolves() {
    let env = load(&tasks_and_people(), &["@test/people", "@test/tasks"]);

    let owned_by = env.registries.edges.get("owned_by").unwrap();
    assert_eq!(owned_by.target_kind.as_deref(), Some("person"));
    assert!(w021(&env).is_empty(), "{:?}", w021(&env));
}

/// A `target_kind` no loaded extension declares is a W021 warning naming
/// the field, the kind and the reference; the extension still registers.
#[specforge_test(
    behavior = "validate_registered_entity_fields",
    verify = "unresolved target_kind produces warning"
)]
fn an_unresolved_target_kind_is_w021() {
    let runtime = KindExtensions(vec![serde_json::json!({
        "name": "@test/tasks",
        "entities": [{
            "name": "task",
            "fields": [{ "name": "owner", "field_type": "reference", "target_kind": "robot" }]
        }]
    })]);

    let env = load(&runtime, &["@test/tasks"]);

    let warnings = w021(&env);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].severity, Severity::Warning);
    for part in ["'owner'", "'task'", "target_kind 'robot'"] {
        assert!(
            warnings[0].message.contains(part),
            "{}",
            warnings[0].message
        );
    }
    assert!(env.registries.kinds.contains("task"));
}

/// An edge label no edge type of the extension declares is a W021 warning
/// naming the field and the label.
#[specforge_test(
    behavior = "validate_registered_entity_fields",
    verify = "unresolved edge label produces warning"
)]
fn an_unresolved_edge_label_is_w021() {
    let runtime = KindExtensions(vec![serde_json::json!({
        "name": "@test/tasks",
        "entities": [
            {
                "name": "task",
                "fields": [{
                    "name": "owner", "field_type": "reference",
                    "target_kind": "person", "edge": "assigned_to"
                }]
            },
            { "name": "person" }
        ]
    })]);

    let env = load(&runtime, &["@test/tasks"]);

    let warnings = w021(&env);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].severity, Severity::Warning);
    for part in ["'owner'", "edge label 'assigned_to'"] {
        assert!(
            warnings[0].message.contains(part),
            "{}",
            warnings[0].message
        );
    }
}

/// A domain the host knows nothing of (cooking) cross-validates by
/// structure alone: its references resolve, so nothing is reported.
#[specforge_test(
    behavior = "validate_registered_entity_fields",
    verify = "cross-validation uses no domain-specific logic"
)]
fn a_domain_the_host_does_not_know_cross_validates_cleanly() {
    let runtime = KindExtensions(vec![serde_json::json!({
        "name": "@custom/cooking",
        "entities": [
            {
                "name": "recipe",
                "fields": [{
                    "name": "ingredients", "field_type": "reference_list",
                    "edge": "uses", "target_kind": "ingredient"
                }]
            },
            { "name": "ingredient" }
        ],
        "edges": [{ "label": "uses", "source_kind": "recipe", "target_kind": "ingredient" }]
    })]);

    let env = load(&runtime, &["@custom/cooking"]);

    assert!(env.registries.kinds.contains("recipe"));
    let diagnostics: Vec<&Diagnostic> = env.diagnostics().collect();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

/// Once every extension is in, resolvable references pass and unresolvable
/// ones are warnings only: the extension still loads its kinds, so the
/// user's compile is not failed by an extension's authoring error.
#[specforge_test(
    behavior = "validate_registered_entity_fields",
    verify = "Validate Registered Entity Fields: field cross-validation holds for the declared obligations"
)]
fn field_cross_validation_holds_on_load() {
    let runtime = KindExtensions(vec![
        serde_json::json!({
            "name": "@test/people",
            "entities": [{ "name": "person" }]
        }),
        serde_json::json!({
            "name": "@test/tasks",
            "peers": ["@test/people"],
            "entities": [{
                "name": "task",
                "fields": [
                    {
                        "name": "owner", "field_type": "reference",
                        "target_kind": "person", "edge": "owned_by"
                    },
                    { "name": "robot", "field_type": "reference", "target_kind": "robot" },
                    {
                        "name": "reviewer", "field_type": "reference",
                        "target_kind": "person", "edge": "reviewed_by"
                    }
                ]
            }],
            "edges": [{ "label": "owned_by", "source_kind": "task", "target_kind": "person" }]
        }),
    ]);

    // The dependency loads after the extension that needs it: the check
    // waits for both.
    let env = load(&runtime, &["@test/tasks", "@test/people"]);

    let warnings = w021(&env);
    let mut messages: Vec<&str> = warnings.iter().map(|d| d.message.as_str()).collect();
    messages.sort();
    assert_eq!(messages.len(), 2, "{messages:?}");
    assert!(
        messages[0].contains("edge label 'reviewed_by'"),
        "{messages:?}"
    );
    assert!(messages[1].contains("target_kind 'robot'"), "{messages:?}");
    assert!(
        !messages.iter().any(|m| m.contains("'owner'")),
        "{messages:?}"
    );
    assert!(warnings.iter().all(|d| d.severity == Severity::Warning));
    assert!(
        !env.diagnostics().any(|d| d.severity == Severity::Error),
        "{:?}",
        env.diagnostics().collect::<Vec<_>>()
    );
    assert!(env.registries.kinds.contains("task") && env.registries.kinds.contains("person"));
}

/// The extension whose `task.owner` targets `person` loads before the peer
/// that declares `person`: a validation run while only the first extension
/// was registered would report the target (W021) and the `person` entity
/// (E024). A real compile reports neither and resolves `owner` to `p1`, so
/// every kind was registered before the first check.
#[specforge_test(
    behavior = "populate_kind_registry_from_extensions",
    verify = "population completes before validation"
)]
fn population_completes_before_any_validation() {
    let runtime = KindExtensions(vec![
        serde_json::json!({
            "name": "@test/tasks",
            "peers": ["@test/people"],
            "entities": [{
                "name": "task",
                "fields": [{
                    "name": "owner", "field_type": "reference",
                    "target_kind": "person", "edge": "owned_by"
                }]
            }],
            "edges": [{ "label": "owned_by", "source_kind": "task", "target_kind": "person" }]
        }),
        serde_json::json!({
            "name": "@test/people",
            "entities": [{ "name": "person" }]
        }),
    ]);
    let dir = TempDir::new().unwrap();
    let config = serde_json::json!({
        "name": "p", "version": "0.1.0", "extensions": ["@test/tasks", "@test/people"]
    });
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    fs::write(
        dir.path().join("main.spec"),
        "task t1 \"T\" {\n  owner p1\n}\n\nperson p1 \"P\" {\n}\n",
    )
    .unwrap();

    let project = specforge_project::CompiledProject::compile(dir.path(), Some(&runtime));

    let loaded: Vec<&str> = project
        .env
        .registries
        .manifests
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert_eq!(loaded, ["@test/tasks", "@test/people"], "load order");
    let diagnostics = project.diagnostics();
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.code == "W021" || d.code == "E024"),
        "{diagnostics:?}"
    );
    assert!(
        project
            .graph
            .edges_from("t1")
            .iter()
            .any(|e| e.label == "owner" && e.target == "p1"),
        "{:?}",
        project.graph.edges()
    );
}
