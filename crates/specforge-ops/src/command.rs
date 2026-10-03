//! Extension commands: the CLI commands extensions contribute in their
//! manifest's surfaces, each a `cmd__<id>` Wasm export.
//!
//! The CLI routes `specforge <ext_short> <command>` to one, and MCP
//! auto-promotes each to the tool `specforge.<ext_short>.<id>`. Both run it
//! here: the export receives the SDK's `CommandInput` (the args, the project
//! root and the compiled graph in the graph export's shape) and answers
//! with a `CommandOutput`. The host knows no command: which exist, their
//! args and what they print are the extension's (ADR 0008).

use serde_json::{Map, Value};
use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_registry::{CommandContribution, ManifestV2, RegistryBuild, SurfaceType};
use specforge_wasm::CommandOutput;
use specforge_wasm::runtime::WasmRuntime;
use std::path::Path;

/// One enabled command an extension contributes.
#[derive(Debug, Clone, Copy)]
pub struct ExtensionCommand<'a> {
    /// The contributing extension's name (`@specforge/product`).
    pub extension: &'a str,
    pub contribution: &'a CommandContribution,
}

impl ExtensionCommand<'_> {
    /// The command's name on the command line: its id, `_` spelled `-`
    /// (`milestone_completion` is `milestone-completion`).
    pub fn cli_name(&self) -> String {
        self.contribution.id.replace('_', "-")
    }
}

/// The commands a project's extensions contribute, in manifest order, but
/// those its configuration disables: a disabled command is not routed.
pub fn extension_commands(build: &RegistryBuild) -> Vec<ExtensionCommand<'_>> {
    build
        .manifest_surfaces
        .iter()
        .flat_map(|(extension, surfaces)| {
            surfaces
                .commands
                .iter()
                .map(move |contribution| ExtensionCommand {
                    extension,
                    contribution,
                })
        })
        .filter(|command| {
            build
                .surfaces
                .iter()
                .find(|e| {
                    e.surface_type == SurfaceType::Command
                        && e.contribution_name == command.contribution.id
                        && e.extension_name == command.extension
                })
                .is_none_or(|e| e.enabled)
        })
        .collect()
}

/// An extension's short name, which names its commands on the CLI and its
/// tools over MCP: its manifest `ext_short`, else the last segment of its
/// name (`@specforge/product` is `product`).
pub fn ext_short(manifests: &[ManifestV2], extension: &str) -> String {
    manifests
        .iter()
        .find(|m| m.name == extension)
        .and_then(|m| m.ext_short.clone())
        .unwrap_or_else(|| {
            extension
                .rsplit('/')
                .next()
                .unwrap_or(extension)
                .trim_start_matches('@')
                .to_string()
        })
}

/// What a `cmd__` export receives: `{"args", "cwd", "graph"}`, the graph as
/// `specforge_emitter::json::emit_json` writes it.
pub fn command_input(graph: &Graph, args: &Map<String, Value>, cwd: &Path) -> Vec<u8> {
    let head = serde_json::json!({
        "args": args,
        "cwd": cwd.display().to_string(),
    })
    .to_string();
    // Splice the graph export in rather than parse it back into a Value.
    let graph = specforge_emitter::json::emit_json(graph);
    let mut input = String::with_capacity(head.len() + graph.len() + 10);
    input.push_str(&head[..head.len() - 1]);
    input.push_str(",\"graph\":");
    input.push_str(&graph);
    input.push('}');
    input.into_bytes()
}

/// Run `export` of `extension` with `args` over `graph`. Err: the export
/// trapped (E028).
pub fn run_command(
    runtime: &dyn WasmRuntime,
    extension: &str,
    export: &str,
    graph: &Graph,
    args: &Map<String, Value>,
    cwd: &Path,
) -> Result<CommandOutput, Diagnostic> {
    let input = command_input(graph, args, cwd);
    specforge_wasm::dispatch_surface_command(extension, export, &input, runtime)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use specforge_common::{SourceSpan, Sym};
    use specforge_graph::Node;
    use specforge_parser::{EntityId, EntityKind, FieldMap};
    use specforge_registry::{SurfaceContributions, SurfaceRegistryEntry};
    use specforge_test_macros::test as specforge_test;
    use specforge_wasm::runtime::{WasmCallResult, WasmTrapInfo};
    use std::sync::Mutex;

    /// Answers `cmd__ok` with a command output and records its input; any
    /// other export traps.
    #[derive(Default)]
    struct Fake {
        inputs: Mutex<Vec<Value>>,
    }

    impl WasmRuntime for Fake {
        fn load_module(&self, _: &Path) -> Result<(), String> {
            Ok(())
        }

        fn call_export(&self, _: &str, export: &str, input: &[u8]) -> WasmCallResult {
            if export != "cmd__ok" {
                return WasmCallResult::Trap(WasmTrapInfo {
                    kind: "unreachable".into(),
                    message: "the command panicked".into(),
                    export_name: export.into(),
                });
            }
            self.inputs
                .lock()
                .unwrap()
                .push(serde_json::from_slice(input).unwrap());
            WasmCallResult::Ok(
                json!({"exit_code": 3, "stdout": "out\n", "stderr": "err\n"})
                    .to_string()
                    .into_bytes(),
            )
        }
    }

    fn graph() -> Graph {
        let mut graph = Graph::new();
        graph.add_node(Node {
            id: EntityId {
                raw: Sym::new("f1"),
            },
            kind: EntityKind {
                raw: Sym::new("feature"),
            },
            title: Some("One".into()),
            fields: FieldMap::new(),
            source_span: SourceSpan {
                file: Sym::new("main.spec"),
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 2,
            },
            methods: Vec::new(),
        });
        graph
    }

    fn command(id: &str) -> CommandContribution {
        CommandContribution {
            id: id.into(),
            title: id.into(),
            description: String::new(),
            category: None,
            export: format!("cmd__{id}"),
            args: Vec::new(),
            sandbox: None,
        }
    }

    #[specforge_test(
        behavior = "dispatch_surface_command",
        verify = "args serialized as JSON to cmd__ export"
    )]
    fn args_reach_the_export_as_json_with_the_graph() {
        let runtime = Fake::default();
        let args = json!({"status": "done", "limit": 2, "all": true});
        let out = run_command(
            &runtime,
            "@acme/x",
            "cmd__ok",
            &graph(),
            args.as_object().unwrap(),
            Path::new("/p"),
        )
        .unwrap();
        let input = runtime.inputs.lock().unwrap().remove(0);
        assert_eq!(input["args"], args);
        assert_eq!(input["cwd"], "/p");
        assert_eq!(input["graph"]["nodes"][0]["id"], "f1");
        assert_eq!(input["graph"]["nodes"][0]["title"], "One");
        assert_eq!(input["graph"]["edges"], json!([]));
        assert_eq!(out.exit_code, 3);
        assert_eq!(out.stdout, b"out\n");
        assert_eq!(out.stderr, b"err\n");
    }

    #[specforge_test(
        behavior = "dispatch_surface_command",
        verify = "Wasm trap caught and reported as ExtensionError"
    )]
    fn a_trapping_command_is_an_extension_error() {
        let err = run_command(
            &Fake::default(),
            "@acme/x",
            "cmd__boom",
            &graph(),
            &Map::new(),
            Path::new("/p"),
        )
        .unwrap_err();
        assert_eq!(err.code, "E028");
        assert!(
            err.message.contains("cmd__boom") && err.message.contains("the command panicked"),
            "{}",
            err.message
        );
    }

    #[specforge_test(
        behavior = "dispatch_surface_command",
        verify = "the command runs in the runtime that read the project's declarations, which loaded only the extensions the project enables"
    )]
    fn a_command_runs_in_the_runtime_that_read_the_declarations() {
        use specforge_wasm::runtime::WasmRuntime as _;
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("specforge.json"),
            json!({"name": "p", "version": "0.1.0", "extensions": ["@specforge/product"]})
                .to_string(),
        )
        .unwrap();
        std::fs::write(
            dir.path().join("main.spec"),
            "feature f1 \"One\" {\n  status done\n  problem \"p\"\n}\n",
        )
        .unwrap();

        // What the CLI does: one runtime, the project's environment read
        // through it, then the routed command run in it.
        let runtime = specforge_component::project_runtime(dir.path());
        let loaded = runtime.loaded_names();
        assert_eq!(
            loaded,
            ["@specforge/product"],
            "only what the project enables"
        );
        let env = specforge_project::Environment::load(dir.path(), Some(&runtime));
        let commands = extension_commands(&env.registries);
        let features = commands
            .iter()
            .find(|c| c.contribution.id == "features")
            .expect("product declares `features`");
        assert!(runtime.load_failure(features.extension).is_none());

        let mut args = Map::new();
        args.insert("format".into(), json!("json"));
        let out = run_command(
            &runtime,
            features.extension,
            &features.contribution.export,
            &env.build_graph(),
            &args,
            dir.path(),
        )
        .unwrap();
        assert_eq!(out.exit_code, 0, "{}", String::from_utf8_lossy(&out.stderr));
        let listed: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(listed["entities"][0]["id"], "f1", "{listed}");
        assert_eq!(
            runtime.loaded_names(),
            loaded,
            "running it loaded no module"
        );
    }

    #[specforge_test(
        behavior = "toggle_surface_contributions",
        verify = "disabled command excluded from CLI routing"
    )]
    fn a_disabled_command_is_not_routed() {
        let build = RegistryBuild {
            manifest_surfaces: vec![(
                "@acme/x".into(),
                SurfaceContributions {
                    commands: vec![command("on"), command("off")],
                    mcp_tools: Vec::new(),
                    mcp_resources: Vec::new(),
                },
            )],
            surfaces: ["on", "off"]
                .into_iter()
                .map(|id| SurfaceRegistryEntry {
                    surface_type: SurfaceType::Command,
                    contribution_name: id.into(),
                    extension_name: "@acme/x".into(),
                    export_name: format!("cmd__{id}"),
                    enabled: id == "on",
                })
                .collect(),
            ..Default::default()
        };
        let routed: Vec<&str> = extension_commands(&build)
            .iter()
            .map(|c| c.contribution.id.as_str())
            .collect();
        assert_eq!(routed, ["on"]);
    }

    #[test]
    fn a_command_is_named_by_its_extension_short_name_and_dashed_id() {
        assert_eq!(ext_short(&[], "@specforge/product"), "product");
        let c = command("milestone_completion");
        let routed = ExtensionCommand {
            extension: "@specforge/product",
            contribution: &c,
        };
        assert_eq!(routed.cli_name(), "milestone-completion");
    }
}
