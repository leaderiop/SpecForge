//! Derived reference fields through the real load and compile: a field's
//! `derived_from`, declared by an extension, gives the compiled graph edges
//! from the type names entities write in their field types and method
//! signatures, and the extension's own rules see them.

use std::fs;
use std::path::Path;

use specforge_common::Diagnostic;
use specforge_project::CompiledProject;
use specforge_test::prelude::*;
use specforge_wasm::{WasmCallResult, WasmRuntime, WasmTrapInfo};
use tempfile::TempDir;

const EXTENSION: &str = "@test/shapes";

/// An extension, in process, with two kinds: `shape`, whose `parts` field
/// derives from its field types, and `iface`, whose `uses` field derives
/// from its method signatures; both point at `shape`. A rule reports a
/// shape nothing references.
struct ShapesExtension;

impl ShapesExtension {
    fn describe(category: &str) -> serde_json::Value {
        let items = match category {
            "entities" => serde_json::json!([
                {
                    "name": "Shape", "keyword": "shape", "open_fields": true,
                    "fields": [{
                        "name": "parts", "field_type": "reference_list",
                        "target_kind": "shape", "derived_from": "type_expressions"
                    }]
                },
                {
                    "name": "Iface", "keyword": "iface", "open_fields": true,
                    "fields": [{
                        "name": "uses", "field_type": "reference_list",
                        "target_kind": "shape", "derived_from": "method_signatures"
                    }]
                }
            ]),
            "validation_rules" => serde_json::json!([{
                "code": "W950", "severity": "warning", "check": "no_incoming_edges",
                "message_template": "shape '{id}' is not referenced",
                "target_kind": "shape"
            }]),
            _ => serde_json::json!([]),
        };
        serde_json::json!({ "category": category, "items": items })
    }
}

impl WasmRuntime for ShapesExtension {
    fn load_module(&self, _: &Path) -> Result<(), String> {
        Ok(())
    }

    fn call_export(&self, extension: &str, export: &str, input: &[u8]) -> WasmCallResult {
        let ok = |value: serde_json::Value| WasmCallResult::Ok(value.to_string().into_bytes());
        if extension != EXTENSION {
            return WasmCallResult::Trap(WasmTrapInfo {
                kind: "extension_not_found".to_string(),
                message: format!("Extension '{extension}' not loaded"),
                export_name: export.to_string(),
            });
        }
        match export {
            "__handshake" => ok(serde_json::json!({
                "protocol_version": "1.0.0",
                "name": EXTENSION,
                "version": "1.0.0",
                "contribution_flags": { "entities": true, "validators": true },
                "peer_dependencies": [],
                "sandbox_policy": null
            })),
            "__describe" => {
                let request: serde_json::Value = serde_json::from_slice(input).unwrap();
                ok(Self::describe(request["category"].as_str().unwrap()))
            }
            other => WasmCallResult::Trap(WasmTrapInfo {
                kind: "guest_error".to_string(),
                message: format!("unknown export '{other}'"),
                export_name: other.to_string(),
            }),
        }
    }
}

fn project(spec: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let config = serde_json::json!({
        "name": "p", "version": "0.1.0", "extensions": [EXTENSION]
    });
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    fs::write(dir.path().join("a.spec"), spec).unwrap();
    dir
}

/// The (target, label) of every edge out of `id`, sorted.
fn edges_from(compiled: &CompiledProject, id: &str) -> Vec<(String, String)> {
    let mut edges: Vec<(String, String)> = compiled
        .graph
        .edges_from(id)
        .iter()
        .map(|e| (e.target.to_string(), e.label.to_string()))
        .collect();
    edges.sort();
    edges
}

#[specforge_test(
    behavior = "link_derived_references",
    verify = "a field's derived_from reaches the graph from the extension's manifest"
)]
fn a_derived_from_declared_by_an_extension_links_the_compiled_graph() {
    let dir = project(
        r#"
shape Wheel {
  size number
}
shape Config {
  root string
}
shape Car {
  wheels Wheel[]
}
shape Spare {
  size number
}
iface Garage {
  method park(config: Config) -> Result<Car, Error>
}
"#,
    );

    let compiled = CompiledProject::compile(dir.path(), Some(&ShapesExtension));

    let edge = |target: &str, label: &str| (target.to_string(), label.to_string());
    assert_eq!(edges_from(&compiled, "Car"), vec![edge("Wheel", "parts")]);
    assert_eq!(
        edges_from(&compiled, "Garage"),
        vec![edge("Car", "uses"), edge("Config", "uses")]
    );
    // Only the shape nothing names is unreferenced.
    let diagnostics: Vec<Diagnostic> = compiled.diagnostics();
    let unreferenced: Vec<&str> = diagnostics
        .iter()
        .filter(|d| d.code == "W950")
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(
        unreferenced,
        ["shape 'Spare' is not referenced"],
        "{diagnostics:?}"
    );
}
