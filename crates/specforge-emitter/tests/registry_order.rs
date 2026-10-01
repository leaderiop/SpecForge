//! Characterization of the registry diagnostics' order (architecture plan
//! 05, step R0).
//!
//! `CompiledProject::compile` loads the extensions, populates the registries,
//! parses the rules and registers the surfaces in a fixed order, and its
//! diagnostics come out in that order: populate (E026, W018), then rule
//! parsing (W112), then the graph's own (here E002), and the surface
//! conflicts (E039) last. Moving that sequence behind one `build_registries`
//! (step R1) must keep this order byte for byte.

use specforge_common::Diagnostic;
use specforge_wasm::protocol::*;
use specforge_wasm::{WasmCallResult, WasmRuntime};
use std::collections::HashMap;
use std::path::Path;

/// A runtime whose extensions answer from fixed tables, keyed by extension
/// name, so two extensions can describe different (conflicting) things.
struct Extensions {
    /// (extension, describe category) -> items
    describe: HashMap<(String, String), serde_json::Value>,
}

impl WasmRuntime for Extensions {
    fn load_module(&self, _wasm_path: &Path) -> Result<(), String> {
        Ok(())
    }

    fn call_export(&self, extension: &str, export: &str, input: &[u8]) -> WasmCallResult {
        let reply = match export {
            "__handshake" => serde_json::to_value(HandshakeResponse {
                protocol_version: PROTOCOL_VERSION.to_string(),
                name: extension.to_string(),
                version: "1.0.0".to_string(),
                contribution_flags: ContributionFlags {
                    entities: true,
                    validators: true,
                    ..Default::default()
                },
                peer_dependencies: vec![],
                sandbox_policy: None,
                starter_template: None,
            })
            .unwrap(),
            "__describe" => {
                let request: DescribeRequest = serde_json::from_slice(input).unwrap();
                let items = self
                    .describe
                    .get(&(extension.to_string(), request.category.clone()))
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([]));
                serde_json::to_value(DescribeResponse {
                    category: request.category,
                    items,
                })
                .unwrap()
            }
            other => panic!("unexpected export {other}"),
        };
        WasmCallResult::Ok(serde_json::to_vec(&reply).unwrap())
    }
}

/// Two extensions that collide on everything the registry build checks:
/// the same entity kind (E026), the same edge label (W018) and the same CLI
/// command (E039); each also declares a rule whose check kind doesn't
/// exist (W112).
fn colliding_extensions() -> Extensions {
    let mut describe = HashMap::new();
    for ext in ["@test/alpha", "@test/beta"] {
        let mut add = |category: &str, items: serde_json::Value| {
            describe.insert((ext.to_string(), category.to_string()), items);
        };
        add(
            "entities",
            serde_json::json!([{"name": "gadget", "description": format!("gadget from {ext}")}]),
        );
        add(
            "edges",
            serde_json::json!([{"label": "GadgetUses", "source_kind": "gadget", "target_kind": "gadget"}]),
        );
        add(
            "validation_rules",
            serde_json::json!([{
                "code": "W900",
                "severity": "warning",
                "message_template": "never fires",
                "check": format!("no_such_check_{}", &ext[6..]),
                "target_kind": "gadget"
            }]),
        );
        add(
            "surfaces",
            serde_json::json!([{"commands": [{
                "id": "hello",
                "title": "Hello",
                "description": format!("hello from {ext}"),
                "export": "run_hello"
            }]}]),
        );
    }
    Extensions { describe }
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    let config = serde_json::json!({
        "name": "order",
        "version": "0.1.0",
        "extensions": ["@test/alpha", "@test/beta"]
    });
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    // A duplicate ID: a graph diagnostic, between the registry ones and E039.
    std::fs::write(
        dir.path().join("main.spec"),
        "gadget widget \"Widget\" {\n}\ngadget widget \"Again\" {\n}\n",
    )
    .unwrap();
    dir
}

fn render(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(specforge_emitter::format_diagnostic)
        .collect()
}

// A characterization, not linked: `diagnostic_determinism` is not proven
// while `check`'s W113 names an import cycle in HashMap order (see the
// parity harness in specforge-cli).
#[test]
fn registry_diagnostics_keep_their_order() {
    let dir = project();
    let runtime = colliding_extensions();

    let expected = [
        "<E026>: error[E026]: entity kind 'gadget' registered by '@test/beta' conflicts with '@test/alpha' (first registration wins)",
        "<W018>: warning[W018]: edge type 'GadgetUses' from '@test/beta' duplicates 'GadgetUses' from '@test/alpha' (first wins)",
        "<W112>: warning[W112]: extension '@test/alpha': unrecognized validation pattern kind 'no_such_check_alpha'",
        "<W112>: warning[W112]: extension '@test/beta': unrecognized validation pattern kind 'no_such_check_beta'",
        // Both extensions declare rule W900 (05·R4 wired W023, after the
        // rule-parse diagnostics).
        "<W023>: warning[W023]: validation rule code 'W900' from '@test/beta' duplicates code from '@test/alpha'",
        "main.spec:3:1: error[E002]: duplicate entity ID 'widget' (first declared at main.spec:1:1)\n  help: rename one of the entities to avoid the collision",
        "<E039>: error[E039]: duplicate surface command ID 'hello': extension '@test/beta' conflicts with '@test/alpha'",
    ];

    // Compiled again and again, with fresh hash maps each time: the order
    // must not depend on them.
    for _ in 0..20 {
        let ctx =
            specforge_project::CompiledProject::compile(dir.path(), Some(&runtime)).into_context();
        assert_eq!(render(&ctx.diagnostics), expected);
    }
}
