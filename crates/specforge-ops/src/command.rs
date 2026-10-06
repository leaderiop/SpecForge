//! Extension commands: the CLI commands extensions declare in their
//! surfaces, each a `cmd__<id>` Wasm export.
//!
//! The CLI routes `specforge <ext_short> <command>` to one, and MCP
//! auto-promotes each to the tool `specforge.<ext_short>.<id>`. Both run it
//! here: the export receives the protocol's `CommandInput` (the args, the
//! project root, the compiled graph in the graph export's shape, the format
//! the caller asked for and the host's date) and answers with a
//! `CommandOutput` (`specforge_protocol_types`, through
//! `specforge_wasm::ExtensionCalls`). The host knows no command: which
//! exist, their args and what they print are the extension's (ADR 0008).
//! The host owns `--format` (ADR 0011).

use serde_json::{Map, Value};
use specforge_graph::Graph;
use specforge_protocol_types::{CommandDescriptor, CommandInput, CommandOutput, RawGraph};
use specforge_registry::RegistryBuild;
use specforge_wasm::runtime::WasmRuntime;
use specforge_wasm::{CallError, ExtensionCalls};
use std::path::Path;

/// The output a command is asked for: `human` (the CLI default) or `json`
/// (always, over MCP). The host's, not the command's: no command declares
/// an arg named `format` (ADR 0011).
pub use specforge_protocol_types::CommandFormat;

/// One enabled command an extension contributes.
#[derive(Debug, Clone, Copy)]
pub struct ExtensionCommand<'a> {
    /// The contributing extension's name (`@specforge/product`).
    pub extension: &'a str,
    pub contribution: &'a CommandDescriptor,
}

impl ExtensionCommand<'_> {
    /// The command's name on the command line: its id, `_` spelled `-`
    /// (`milestone_completion` is `milestone-completion`).
    pub fn cli_name(&self) -> String {
        self.contribution.id.replace('_', "-")
    }
}

/// The commands a project's extensions contribute, in load order.
pub fn extension_commands(build: &RegistryBuild) -> Vec<ExtensionCommand<'_>> {
    build
        .declarations()
        .iter()
        .flat_map(|declaration| {
            declaration
                .surfaces
                .commands
                .iter()
                .map(move |contribution| ExtensionCommand {
                    extension: declaration.name(),
                    contribution,
                })
        })
        .collect()
}

/// The options the host gives every extension command on the command line
/// (`--path`, `--format`, `--help`), which no declared arg may take.
pub const HOST_OPTIONS: &[&str] = &["path", "format", "help"];

/// Why the host refuses `contribution`, if it does: an arg takes an option
/// the host reserves ([`HOST_OPTIONS`]), or two args share a name (`_` and
/// `-` spelled alike). A refused command is on neither surface: the CLI
/// refuses to run it (exit 2) and MCP does not promote it to a tool
/// (ADR 0011).
pub fn refusal(contribution: &CommandDescriptor) -> Option<String> {
    let mut seen: Vec<String> = Vec::new();
    for arg in &contribution.args {
        let name = arg.name.replace('_', "-");
        if HOST_OPTIONS.contains(&name.as_str()) {
            return Some(format!("its arg '{}' takes the host's --{name}", arg.name));
        }
        if seen.contains(&name) {
            return Some(format!("it declares the arg '{name}' twice"));
        }
        seen.push(name);
    }
    None
}

/// What the host passes a command beside its args: the format the caller
/// asked for, and the host's date when it was called (UTC, `YYYY-MM-DD`),
/// computed by the caller so a test can pin it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandContext {
    pub format: CommandFormat,
    pub today: String,
}

/// What a `cmd__` export receives: the args, the project root, the format
/// and the date, and `graph` as `specforge_emitter::json::emit_json` renders
/// it, spliced as rendered rather than parsed back into a value.
pub fn command_input(
    graph: &Graph,
    args: &Map<String, Value>,
    cwd: &Path,
    context: &CommandContext,
) -> CommandInput<RawGraph> {
    CommandInput {
        args: args.clone(),
        cwd: cwd.display().to_string(),
        format: context.format,
        today: context.today.clone(),
        graph: RawGraph::new(specforge_emitter::json::emit_json(graph))
            .expect("the graph export is one JSON value"),
    }
}

/// Run `export` of `extension` with `args` over `graph`, in `context`. Err:
/// the export did not answer a `CommandOutput` (it trapped, the guest does
/// not route it, or its answer is not one): E028.
pub fn run_command(
    runtime: &dyn WasmRuntime,
    extension: &str,
    export: &str,
    graph: &Graph,
    args: &Map<String, Value>,
    cwd: &Path,
    context: &CommandContext,
) -> Result<CommandOutput, CallError> {
    let input = command_input(graph, args, cwd, context);
    ExtensionCalls::new(runtime).run_command(extension, export, &input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use specforge_common::{SourceSpan, Sym};
    use specforge_extension_sdk::prelude::*;
    use specforge_graph::Node;
    use specforge_parser::{EntityId, EntityKind, FieldMap};
    use specforge_test_macros::test as specforge_test;
    use specforge_wasm::runtime::WasmCallResult;
    use specforge_wasm::testing::InProcessRuntime;
    use specforge_wasm::{CallFailure, Operation};

    const EXT: &str = "@acme/x";

    /// `@acme/x`, whose guest answers `cmd__ok` with a command output after
    /// reading its input as the SDK's `CommandInput`, and panics in
    /// `cmd__boom`; it routes no other export.
    fn fake() -> InProcessRuntime {
        fn guest(export: &str, input: &[u8]) -> Option<Result<Vec<u8>, String>> {
            match export {
                "cmd__ok" => Some(
                    serde_json::from_slice::<specforge_extension_sdk::CommandInput>(input)
                        .map_err(|e| e.to_string())
                        .map(|_| {
                            CommandOutput {
                                exit_code: 3,
                                stdout: "out\n".into(),
                                stderr: "err\n".into(),
                            }
                            .to_bytes()
                        }),
                ),
                "cmd__boom" => panic!("the command panicked"),
                _ => None,
            }
        }
        InProcessRuntime::new().with_handler(
            || ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0")),
            guest,
        )
    }

    /// The input the last call received, as JSON.
    fn last_input(runtime: &InProcessRuntime) -> Value {
        runtime.calls().last().expect("a call").input.clone()
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

    fn command(id: &str) -> CommandDescriptor {
        CommandDescriptor {
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
        let runtime = fake();
        let args = json!({"status": "done", "limit": 2, "all": true});
        let out = run_command(
            &runtime,
            EXT,
            "cmd__ok",
            &graph(),
            args.as_object().unwrap(),
            Path::new("/p"),
            &CommandContext::default(),
        )
        .unwrap();
        // What the guest decodes: the SDK's CommandInput.
        let input: specforge_extension_sdk::CommandInput =
            serde_json::from_value(last_input(&runtime)).unwrap();
        assert_eq!(Value::Object(input.args), args);
        assert_eq!(input.cwd, "/p");
        let f1 = input.graph.node("f1").unwrap();
        assert_eq!(f1.title.as_deref(), Some("One"));
        assert!(input.graph.edges().is_empty());
        assert_eq!(
            (out.exit_code, out.stdout.as_str(), out.stderr.as_str()),
            (3, "out\n", "err\n")
        );
    }

    #[specforge_test(
        behavior = "dispatch_surface_command",
        verify = "the CommandInput carries the format the caller asked for and the host's date"
    )]
    fn the_input_carries_the_format_and_the_date() {
        let runtime = fake();
        for format in CommandFormat::ALL {
            let context = CommandContext {
                format,
                today: "2026-10-03".into(),
            };
            run_command(
                &runtime,
                EXT,
                "cmd__ok",
                &graph(),
                &Map::new(),
                Path::new("/p"),
                &context,
            )
            .unwrap();
            let input = last_input(&runtime);
            assert_eq!(input["format"], format.as_str());
            assert_eq!(input["today"], "2026-10-03");
            assert_eq!(input["args"], json!({}), "the format is not an arg");
        }
        assert_eq!(CommandFormat::parse("json"), Some(CommandFormat::Json));
        assert_eq!(CommandFormat::parse("table"), None);
    }

    #[specforge_test(
        behavior = "dispatch_surface_command",
        verify = "Wasm trap caught and reported as ExtensionError"
    )]
    fn a_trapping_command_is_an_extension_error() {
        let err = run_command(
            &fake(),
            EXT,
            "cmd__boom",
            &graph(),
            &Map::new(),
            Path::new("/p"),
            &CommandContext::default(),
        )
        .unwrap_err();
        let diagnostic = err.diagnostic();
        assert_eq!(diagnostic.code, "E028");
        assert_eq!(
            diagnostic.message,
            "command cmd__boom() of '@acme/x' trapped: call_failed: unreachable: the command panicked"
        );
    }

    #[specforge_test(
        behavior = "dispatch_surface_command",
        verify = "a command whose output is not a CommandOutput is an ExtensionError, not exit 0 with the raw bytes"
    )]
    fn a_command_whose_output_is_not_a_command_output_is_an_extension_error() {
        for raw in [
            &b"not json at all"[..],
            br#"{"exit_code":"3","stdout":"x"}"#,
            br#"{}"#,
            br#"[1,2]"#,
        ] {
            let runtime = fake().answer_raw(EXT, "cmd__ok", WasmCallResult::Ok(raw.to_vec()));
            let err = run_command(
                &runtime,
                EXT,
                "cmd__ok",
                &graph(),
                &Map::new(),
                Path::new("/p"),
                &CommandContext::default(),
            )
            .unwrap_err();
            let shown = String::from_utf8_lossy(raw);
            assert_eq!(err.operation, Operation::Command, "{shown}");
            assert!(
                matches!(
                    err.failure,
                    CallFailure::Malformed {
                        expected: "CommandOutput",
                        ..
                    }
                ),
                "{shown}: {err}"
            );
            let diagnostic = err.diagnostic();
            assert_eq!(diagnostic.code, "E028");
            assert!(
                diagnostic.message.starts_with(
                    "command cmd__ok() of '@acme/x' answered output that is not a CommandOutput: "
                ),
                "{}",
                diagnostic.message
            );
        }
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

        let json = CommandContext {
            format: CommandFormat::Json,
            today: "2026-10-03".into(),
        };
        let out = run_command(
            &runtime,
            features.extension,
            &features.contribution.export,
            &env.build_graph(),
            &Map::new(),
            dir.path(),
            &json,
        )
        .unwrap();
        assert_eq!(out.exit_code, 0, "{}", out.stderr);
        let listed: Value = serde_json::from_str(&out.stdout).unwrap();
        assert_eq!(listed["features"][0]["id"], "f1", "{listed}");
        assert_eq!(
            runtime.loaded_names(),
            loaded,
            "running it loaded no module"
        );
    }

    #[specforge_test(
        behavior = "dispatch_surface_command",
        verify = "a declared export the guest does not route is an ExtensionError when dispatched"
    )]
    fn an_export_the_guest_does_not_route_is_an_extension_error() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("specforge.json"),
            json!({"name": "p", "version": "0.1.0", "extensions": ["@specforge/product"]})
                .to_string(),
        )
        .unwrap();
        let runtime = specforge_component::project_runtime(dir.path());
        let err = run_command(
            &runtime,
            "@specforge/product",
            "cmd__product_no_such_command",
            &graph(),
            &Map::new(),
            dir.path(),
            &CommandContext::default(),
        )
        .unwrap_err();
        assert_eq!(err.diagnostic().code, "E028");
        assert_eq!(
            err.to_string(),
            "command cmd__product_no_such_command() of '@specforge/product' trapped: guest_error: \
             unknown export 'cmd__product_no_such_command'"
        );
    }

    #[test]
    fn a_command_is_named_by_its_extension_short_name_and_dashed_id() {
        let product = specforge_protocol_types::ExtensionDeclaration {
            handshake: specforge_protocol_types::HandshakeResponse {
                name: "@specforge/product".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(product.short(), "product");
        let c = command("milestone_completion");
        let routed = ExtensionCommand {
            extension: "@specforge/product",
            contribution: &c,
        };
        assert_eq!(routed.cli_name(), "milestone-completion");
    }
}
