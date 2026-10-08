//! Extension commands: the CLI commands extensions declare in their
//! surfaces, each a `cmd__<id>` Wasm export.
//!
//! [`ExtensionCommand`] derives one command, once, for every surface: its
//! name on the command line (`specforge <short> <cli name>`), its MCP tool
//! (`specforge.<short>.<id>`, its input schema), its args' command-line
//! shapes, the args both surfaces send its export (normalized by the one
//! arg rule the SDK also runs, `specforge_protocol_types::command_args`),
//! and why the host refuses it. [`ExtensionCommands`] is the project's
//! routing table. The CLI and MCP both read them and run the command here:
//! the export receives the protocol's `CommandInput` (the args, the project
//! root, the compiled graph in the graph export's shape, the format the
//! caller asked for and the host's date) and answers with a
//! `CommandOutput` (`specforge_protocol_types`, through
//! `specforge_wasm::ExtensionCalls`). The host knows no command: which
//! exist, their args and what they print are the extension's (ADR 0008).
//! The host owns `--format` (ADR 0011), and normalizes the args (ADR 0017).

use crate::view::ProjectView;
use serde_json::{Map, Value, json};
use specforge_graph::Graph;
use specforge_protocol_types::command_args::{self, ArgError, normalize_arg, option_name};
use specforge_protocol_types::{
    CommandArgDescriptor, CommandArgType, CommandDescriptor, CommandInput, CommandOutput,
    EntityEvidence, RawGraph,
};
use specforge_registry::RegistryBuild;
use specforge_wasm::runtime::WasmRuntime;
use specforge_wasm::{CallError, ExtensionCalls};
use std::path::Path;

/// The output a command is asked for: `human` (the CLI default) or `json`
/// (always, over MCP). The host's, not the command's: no command declares
/// an arg named `format` (ADR 0011).
pub use specforge_protocol_types::CommandFormat;

/// What a command's input says the recorded tests prove ([`evidence`]).
pub use specforge_protocol_types::CommandEvidence;

/// Why the host refuses a command: its arg declarations break the one arg
/// rule ([`command_args::refusal`]). A refused command is on neither
/// surface: the CLI refuses to run it (exit 2) and MCP serves no tool for
/// it (I017). Its `Display` is the reason both write.
pub use specforge_protocol_types::command_args::ArgRefusal as Refusal;

/// The options the host gives every extension command on the command line
/// (`--path`, `--format`, `--help`), which no declared arg may take: the
/// one list the SDK checks too.
pub use specforge_protocol_types::command_args::HOST_OPTIONS;

/// One command an extension declares, derived once for every surface.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionCommand {
    /// The declaring extension's name (`@specforge/product`).
    extension: String,
    /// The extension's short name (`product`): its declared `ext_short`,
    /// else its name's last segment.
    short: String,
    /// The command as declared.
    declaration: CommandDescriptor,
    /// Its args, in declaration order, as both surfaces present them.
    args: Vec<ExtensionArg>,
    refusal: Option<Refusal>,
}

/// One declared arg of a command, as both surfaces present it.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionArg {
    /// As declared: `sort_order`.
    pub name: String,
    pub value: ArgValue,
    pub shape: ArgShape,
    /// The declared default, typed (normalized once from `default_value`);
    /// `None` without one, or with one its type refuses (the command is
    /// then refused).
    pub default: Option<Value>,
    pub description: Option<String>,
}

/// What values an arg takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgValue {
    Text,
    Path,
    /// An integer, at least `minimum` when it has one (`0`: a count).
    Integer {
        minimum: Option<i64>,
    },
    /// `true` or `false`: `false` unless set.
    Flag,
    /// One of these values.
    OneOf(Vec<String>),
}

/// Where an arg is on the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgShape {
    /// A required arg that is not a flag: positional, its `index` among
    /// the positional args, in declaration order.
    Positional { index: usize },
    /// Any other arg that is not a flag: `--<long> <value>`.
    Option { long: String },
    /// Every flag, required or not: `--<long>`, set or not.
    Flag { long: String },
}

impl ExtensionArg {
    fn derive(declared: &CommandArgDescriptor, positional: &mut usize) -> Self {
        let value = match &declared.arg_type {
            CommandArgType::String => ArgValue::Text,
            CommandArgType::Path => ArgValue::Path,
            CommandArgType::Integer => ArgValue::Integer {
                minimum: declared.minimum,
            },
            CommandArgType::Bool => ArgValue::Flag,
            CommandArgType::Enum { values } => ArgValue::OneOf(values.clone()),
        };
        let long = option_name(&declared.name);
        let shape = match &value {
            ArgValue::Flag => ArgShape::Flag { long },
            _ if declared.required => {
                *positional += 1;
                ArgShape::Positional {
                    index: *positional - 1,
                }
            }
            _ => ArgShape::Option { long },
        };
        let default = match (&value, &declared.default_value) {
            (ArgValue::Flag, _) | (_, None) => None,
            (_, Some(default)) => normalize_arg(declared, &Value::String(default.clone())).ok(),
        };
        ExtensionArg {
            name: declared.name.clone(),
            value,
            shape,
            default,
            description: declared.description.clone(),
        }
    }

    /// The arg's property in its command tool's input schema: its JSON
    /// type, its values, its minimum, its default and its description.
    fn schema(&self) -> Value {
        let mut property = match &self.value {
            ArgValue::Text | ArgValue::Path => json!({"type": "string"}),
            ArgValue::Integer { minimum } => {
                let mut property = json!({"type": "integer"});
                if let Some(minimum) = minimum {
                    property["minimum"] = json!(minimum);
                }
                property
            }
            ArgValue::Flag => json!({"type": "boolean", "default": false}),
            ArgValue::OneOf(values) => json!({"type": "string", "enum": values}),
        };
        if let Some(default) = &self.default {
            property["default"] = default.clone();
        }
        if let Some(description) = &self.description {
            property["description"] = json!(description);
        }
        property
    }

    /// Whether the caller must give it: a required arg that is not a flag.
    pub fn is_required(&self) -> bool {
        matches!(self.shape, ArgShape::Positional { .. })
    }
}

impl ExtensionCommand {
    /// `declaration`, a command of `extension` (short name `short`),
    /// derived for every surface.
    pub fn new(extension: &str, short: &str, declaration: &CommandDescriptor) -> Self {
        let mut positional = 0;
        let args = declaration
            .args
            .iter()
            .map(|arg| ExtensionArg::derive(arg, &mut positional))
            .collect();
        ExtensionCommand {
            extension: extension.to_string(),
            short: short.to_string(),
            declaration: declaration.clone(),
            args,
            refusal: command_args::refusal(&declaration.args),
        }
    }

    /// The declaring extension's name.
    pub fn extension(&self) -> &str {
        &self.extension
    }

    /// The declaring extension's short name, which routes its commands.
    pub fn short(&self) -> &str {
        &self.short
    }

    /// The command's declared id (`milestone_completion`).
    pub fn id(&self) -> &str {
        &self.declaration.id
    }

    /// The Wasm export that runs it.
    pub fn export(&self) -> &str {
        &self.declaration.export
    }

    /// The one-line title help lists it under.
    pub fn title(&self) -> &str {
        &self.declaration.title
    }

    /// What it answers: its long help and its tool's description.
    pub fn description(&self) -> &str {
        &self.declaration.description
    }

    /// The command as declared.
    pub fn declaration(&self) -> &CommandDescriptor {
        &self.declaration
    }

    /// Its name on the command line: its id, `_` spelled `-`
    /// (`milestone_completion` is `milestone-completion`).
    pub fn cli_name(&self) -> String {
        option_name(&self.declaration.id)
    }

    /// Its MCP tool's name: `specforge.{short}.{id}`.
    pub fn tool_name(&self) -> String {
        format!("specforge.{}.{}", self.short, self.declaration.id)
    }

    /// Its args, in declaration order.
    pub fn args(&self) -> &[ExtensionArg] {
        &self.args
    }

    /// Why the host refuses it, if it does: then it runs on no surface.
    pub fn refusal(&self) -> Option<&Refusal> {
        self.refusal.as_ref()
    }

    /// The JSON Schema of its MCP tool's arguments: each arg's type, its
    /// values, its minimum, its default and its description; `required`
    /// lists the required args that are not flags (a flag is `false` unless
    /// set); no undeclared property is accepted.
    pub fn input_schema(&self) -> Value {
        let properties: Map<String, Value> = self
            .args
            .iter()
            .map(|arg| (arg.name.clone(), arg.schema()))
            .collect();
        let mut schema = json!({
            "type": "object",
            "properties": properties,
            "additionalProperties": false,
        });
        let required: Vec<&str> = self
            .args
            .iter()
            .filter(|arg| arg.is_required())
            .map(|arg| arg.name.as_str())
            .collect();
        if !required.is_empty() {
            schema["required"] = json!(required);
        }
        schema
    }

    /// The args both surfaces send its export for the arguments `given`:
    /// [`command_args::normalize_args`] over its declaration (a declared
    /// default for an absent arg, `false` for an unset flag, each value its
    /// declared type; an undeclared argument, a value of another type or a
    /// missing required arg refused).
    pub fn normalize(&self, given: &Map<String, Value>) -> Result<Map<String, Value>, ArgError> {
        command_args::normalize_args(&self.declaration.args, given)
    }
}

/// The project's extension commands, routed by `(short name, CLI name)`:
/// each once, the first in load order (then declaration order) winning; a
/// later command under a taken pair is [`shadowed`](Self::shadowed) and
/// runs on no surface.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExtensionCommands {
    commands: Vec<ExtensionCommand>,
    shadowed: Vec<ExtensionCommand>,
}

impl ExtensionCommands {
    /// The commands `build`'s declarations declare, in load order.
    pub fn build(build: &RegistryBuild) -> Self {
        let mut routed = ExtensionCommands::default();
        for declaration in build.declarations() {
            let short = declaration.short();
            for command in &declaration.surfaces.commands {
                let command = ExtensionCommand::new(declaration.name(), &short, command);
                let taken = routed
                    .commands
                    .iter()
                    .any(|c| c.short == command.short && c.cli_name() == command.cli_name());
                if taken {
                    routed.shadowed.push(command);
                } else {
                    routed.commands.push(command);
                }
            }
        }
        routed
    }

    /// Every routed command, refused ones included, in load order.
    pub fn all(&self) -> &[ExtensionCommand] {
        &self.commands
    }

    /// The routed commands of the extensions whose short name is `short`.
    pub fn of<'a>(&'a self, short: &'a str) -> impl Iterator<Item = &'a ExtensionCommand> {
        self.commands.iter().filter(move |c| c.short == short)
    }

    /// The short names that route commands, each once, in load order.
    pub fn shorts(&self) -> Vec<&str> {
        let mut shorts: Vec<&str> = Vec::new();
        for command in &self.commands {
            if !shorts.contains(&command.short.as_str()) {
                shorts.push(&command.short);
            }
        }
        shorts
    }

    /// The commands a command earlier in load order shadows: same short
    /// name, same CLI name.
    pub fn shadowed(&self) -> &[ExtensionCommand] {
        &self.shadowed
    }
}

/// What the host passes a command beside its args: the format the caller
/// asked for, the host's date when it was called (UTC, `YYYY-MM-DD`; a test
/// that pins it sets `today`), and what the project's recorded tests prove
/// ([`evidence`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandContext {
    pub format: CommandFormat,
    pub today: String,
    pub evidence: CommandEvidence,
}

impl CommandContext {
    /// The context of a command called now, in `format`: today's date in
    /// UTC (`YYYY-MM-DD`), the one clock every surface reads, and no
    /// evidence until the caller sets it.
    pub fn now(format: CommandFormat) -> Self {
        Self {
            format,
            today: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            evidence: CommandEvidence::default(),
        }
    }
}

/// What the view's recorded test report proves, as a command's input
/// carries it: `none` without a report, `unreadable` (with why) when the
/// report is there and cannot be used, else every entity that counts toward
/// coverage (its standing, ADR 0004 D2-b) scored by the one coverage rule
/// (`ProjectView::coverage`, the numbers `specforge stats` reports).
pub fn evidence(view: &ProjectView<'_>) -> CommandEvidence {
    let recorded = match view.recorded() {
        Ok(recorded) => recorded,
        Err(error) => {
            return CommandEvidence::Unreadable {
                reason: error.to_string(),
            };
        }
    };
    if recorded.report.is_none() {
        return CommandEvidence::None;
    }
    let entities = view
        .entities()
        .iter()
        .filter(|(_, standing)| standing.counts())
        .map(|(record, _)| {
            let verdict = recorded.coverage.verdict(&record.id);
            let evidence = EntityEvidence {
                obligations: verdict.map_or(0, |v| v.obligations),
                proven: verdict.map_or(0, |v| v.proven),
                failing: verdict.map_or(0, |v| v.failing),
            };
            (record.id.clone(), evidence)
        })
        .collect();
    CommandEvidence::Recorded { entities }
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
        evidence: context.evidence.clone(),
    }
}

/// Run `command`'s export with `args` (what [`ExtensionCommand::normalize`]
/// gave) over `graph`, in `context`. Err: the export did not answer a
/// `CommandOutput` (it trapped, the guest does not route it, or its answer
/// is not one): E028.
pub fn run_command(
    runtime: &dyn WasmRuntime,
    command: &ExtensionCommand,
    graph: &Graph,
    args: &Map<String, Value>,
    cwd: &Path,
    context: &CommandContext,
) -> Result<CommandOutput, CallError> {
    let input = command_input(graph, args, cwd, context);
    ExtensionCalls::new(runtime).run_command(command.extension(), command.export(), &input)
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
        }
    }

    /// `@acme/x`'s command `id`, declared with no arg.
    fn routed(id: &str) -> ExtensionCommand {
        ExtensionCommand::new(EXT, "x", &command(id))
    }

    /// A command declared as JSON (a `CommandDescriptor`), of `@acme/x`.
    fn declared(declaration: Value) -> ExtensionCommand {
        ExtensionCommand::new(EXT, "x", &serde_json::from_value(declaration).unwrap())
    }

    /// `ordered`: an enum arg with a declared default, a flag, a count with
    /// a description (R2).
    fn ordered() -> ExtensionCommand {
        declared(
            json!({"id": "ordered", "title": "Ordered", "description": "List in order",
            "export": "cmd__ordered",
            "args": [{"name": "order", "arg_type": {"enum": {"values": ["asc", "desc"]}},
                      "default_value": "desc"},
                     {"name": "all", "arg_type": "bool"},
                     {"name": "limit", "arg_type": "integer", "minimum": 0,
                      "default_value": "10", "description": "At most this many"},
                     {"name": "milestone", "arg_type": "string", "required": true}]}),
        )
    }

    /// `strict`: a required flag (R3).
    fn strict() -> ExtensionCommand {
        declared(
            json!({"id": "strict", "title": "Strict", "description": "Check strictly",
            "export": "cmd__strict",
            "args": [{"name": "strict", "arg_type": "bool", "required": true}]}),
        )
    }

    fn arg(name: &str, arg_type: &str) -> Value {
        json!({"name": name, "arg_type": arg_type})
    }

    #[specforge_test(
        behavior = "auto_promote_commands_to_mcp_tools",
        verify = "the derived input_schema states each arg's default and minimum and accepts no undeclared argument"
    )]
    fn the_input_schema_is_the_declaration() {
        assert_eq!(
            ordered().input_schema(),
            json!({
                "type": "object",
                "properties": {
                    "order": {"type": "string", "enum": ["asc", "desc"], "default": "desc"},
                    "all": {"type": "boolean", "default": false},
                    "limit": {"type": "integer", "minimum": 0, "default": 10,
                              "description": "At most this many"},
                    "milestone": {"type": "string"}
                },
                "required": ["milestone"],
                "additionalProperties": false
            })
        );
        // No arg: no property, none required, nothing else accepted.
        assert_eq!(
            routed("check").input_schema(),
            json!({"type": "object", "properties": {}, "additionalProperties": false})
        );
        let refused = ordered()
            .normalize(
                json!({"milestone": "m1", "format": "json"})
                    .as_object()
                    .unwrap(),
            )
            .unwrap_err();
        assert_eq!(refused.message(), "unknown argument 'format'");
    }

    #[specforge_test(
        behavior = "auto_promote_commands_to_mcp_tools",
        verify = "a required flag is not required over MCP, as on the command line"
    )]
    fn a_required_flag_is_a_flag() {
        let strict = strict();
        assert_eq!(
            strict.args()[0].shape,
            ArgShape::Flag {
                long: "strict".into()
            }
        );
        assert!(!strict.args()[0].is_required());
        assert!(strict.input_schema().get("required").is_none());
        assert_eq!(strict.refusal(), None);
        assert_eq!(
            Value::Object(strict.normalize(&Map::new()).unwrap()),
            json!({"strict": false})
        );
    }

    #[test]
    fn the_args_both_surfaces_send_are_normalized_with_their_defaults() {
        let sent = |given: Value| {
            ordered()
                .normalize(given.as_object().unwrap())
                .map(Value::Object)
                .map_err(|e| e.message())
        };
        assert_eq!(
            sent(json!({"milestone": "m1"})),
            Ok(json!({"order": "desc", "all": false, "limit": 10, "milestone": "m1"}))
        );
        assert_eq!(
            sent(json!({"milestone": "m1", "order": "asc", "all": "true", "limit": "3"})),
            Ok(json!({"order": "asc", "all": true, "limit": 3, "milestone": "m1"}))
        );
        assert_eq!(
            sent(json!({"milestone": "m1", "limit": -1})),
            Err("limit must be a non-negative integer, got -1".to_string())
        );
        assert_eq!(
            sent(json!({})),
            Err("missing required arg 'milestone'".to_string())
        );
    }

    #[test]
    fn each_arg_has_its_command_line_shape() {
        let ordered = ordered();
        let shapes: Vec<(&str, &ArgShape, &ArgValue)> = ordered
            .args()
            .iter()
            .map(|a| (a.name.as_str(), &a.shape, &a.value))
            .collect();
        assert_eq!(
            shapes,
            [
                (
                    "order",
                    &ArgShape::Option {
                        long: "order".into()
                    },
                    &ArgValue::OneOf(vec!["asc".into(), "desc".into()])
                ),
                (
                    "all",
                    &ArgShape::Flag { long: "all".into() },
                    &ArgValue::Flag
                ),
                (
                    "limit",
                    &ArgShape::Option {
                        long: "limit".into()
                    },
                    &ArgValue::Integer { minimum: Some(0) }
                ),
                (
                    "milestone",
                    &ArgShape::Positional { index: 0 },
                    &ArgValue::Text
                ),
            ]
        );
        assert_eq!(ordered.args()[2].default, Some(json!(10)));
        let sorted = declared(
            json!({"id": "s", "title": "s", "description": "", "export": "cmd__s",
            "args": [{"name": "sort_order", "arg_type": "path"}]}),
        );
        assert_eq!(
            sorted.args()[0].shape,
            ArgShape::Option {
                long: "sort-order".into()
            }
        );
    }

    #[specforge_test(
        behavior = "dispatch_surface_command",
        verify = "a command declaring an arg named format is refused on the command line"
    )]
    fn an_arg_taking_a_host_option_or_another_args_name_is_refused() {
        let with = |args: Vec<Value>| {
            declared(
                json!({"id": "w", "title": "w", "description": "", "export": "cmd__w",
                "args": args}),
            )
            .refusal()
            .map(ToString::to_string)
        };
        assert_eq!(with(vec![arg("milestone", "string")]), None);
        for name in HOST_OPTIONS {
            assert_eq!(
                with(vec![arg(name, "string")]),
                Some(format!("its arg '{name}' takes the host's --{name}"))
            );
        }
        assert_eq!(
            with(vec![arg("all_kinds", "bool"), arg("all-kinds", "bool")]),
            Some("it declares the arg 'all-kinds' twice".into())
        );
        // The contradictions the SDK refuses to build (D12), refused here
        // too for a guest that declares raw JSON.
        assert_eq!(
            with(vec![json!({"name": "n", "arg_type": "integer", "minimum": 0,
                "default_value": "-1"})]),
            Some(
                "its arg 'n' has a default its type refuses: n must be a non-negative integer, got -1"
                    .into()
            )
        );
        assert_eq!(
            with(vec![
                json!({"name": "all", "arg_type": "bool", "default_value": "true"})
            ]),
            Some("its flag 'all' has a default, but a flag is false unless set".into())
        );
        assert_eq!(
            with(vec![
                json!({"name": "m", "arg_type": "string", "required": true,
                "default_value": "m1"})
            ]),
            Some("its arg 'm' is both required and with a default".into())
        );
        // A refused default is no default.
        let bad = declared(
            json!({"id": "w", "title": "w", "description": "", "export": "cmd__w",
            "args": [{"name": "o", "arg_type": {"enum": {"values": ["a"]}}, "default_value": "b"}]}),
        );
        assert_eq!(bad.args()[0].default, None);
        assert!(matches!(bad.refusal(), Some(Refusal::BadDefault { .. })));
    }

    /// A build of `declarations` (`(name, short, command ids)`), in order.
    fn build_of(declarations: &[(&str, Option<&str>, &[&str])]) -> RegistryBuild {
        let declarations = declarations
            .iter()
            .map(
                |(name, short, ids)| specforge_protocol_types::ExtensionDeclaration {
                    handshake: specforge_protocol_types::HandshakeResponse {
                        name: (*name).into(),
                        version: "1.0.0".into(),
                        ext_short: short.map(str::to_string),
                        ..Default::default()
                    },
                    surfaces: specforge_protocol_types::SurfaceDescriptor {
                        commands: ids.iter().map(|id| command(id)).collect(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .collect();
        specforge_registry::build_registries(declarations)
    }

    #[test]
    fn a_command_is_routed_once_by_its_short_name_and_cli_name() {
        // R7: two extensions with one short name, and two commands of one
        // CLI name: the first in load order wins, the later is shadowed.
        let commands = ExtensionCommands::build(&build_of(&[
            ("@acme/widgets", None, &["list", "a_b"]),
            ("@other/widgets", None, &["list", "show"]),
            ("@acme/tools", Some("t"), &["a-b", "a_b"]),
        ]));
        let routed: Vec<(&str, String)> = commands
            .all()
            .iter()
            .map(|c| (c.extension(), c.tool_name()))
            .collect();
        assert_eq!(
            routed,
            [
                ("@acme/widgets", "specforge.widgets.list".to_string()),
                ("@acme/widgets", "specforge.widgets.a_b".to_string()),
                ("@other/widgets", "specforge.widgets.show".to_string()),
                ("@acme/tools", "specforge.t.a-b".to_string()),
            ]
        );
        let shadowed: Vec<(&str, &str)> = commands
            .shadowed()
            .iter()
            .map(|c| (c.extension(), c.id()))
            .collect();
        assert_eq!(
            shadowed,
            [("@other/widgets", "list"), ("@acme/tools", "a_b")]
        );
        assert_eq!(commands.shorts(), ["widgets", "t"]);
        let of_widgets: Vec<&str> = commands.of("widgets").map(|c| c.id()).collect();
        assert_eq!(of_widgets, ["list", "a_b", "show"]);
    }

    #[specforge_test(type = "ExtensionCommand", verify = "ExtensionCommand schema is valid")]
    fn an_extension_command_carries_every_derived_field() {
        let commands = ExtensionCommands::build(&build_of(&[(
            "@specforge/product",
            None,
            &["milestone_completion"],
        )]));
        let [command] = commands.all() else {
            panic!("one command: {commands:?}")
        };
        assert_eq!(command.extension(), "@specforge/product");
        assert_eq!(command.short(), "product");
        assert_eq!(command.cli_name(), "milestone-completion");
        assert_eq!(
            command.tool_name(),
            "specforge.product.milestone_completion"
        );
        assert!(command.args().is_empty());
        assert_eq!(command.input_schema()["type"], "object");
        assert_eq!(command.refusal(), None);
        assert_eq!(command.export(), "cmd__milestone_completion");
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
            &routed("ok"),
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

    #[test]
    fn now_carries_todays_utc_date() {
        let context = CommandContext::now(CommandFormat::Json);

        assert_eq!(context.format, CommandFormat::Json);
        let parsed = chrono::NaiveDate::parse_from_str(&context.today, "%Y-%m-%d")
            .unwrap_or_else(|e| panic!("{:?} is no %Y-%m-%d date: {e}", context.today));
        assert_eq!(parsed, chrono::Utc::now().date_naive());
        assert_eq!(context.evidence, CommandEvidence::default());
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
                ..CommandContext::default()
            };
            run_command(
                &runtime,
                &routed("ok"),
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
            &routed("boom"),
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
                &routed("ok"),
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
        let runtime = specforge_component::ComponentRuntime::with_user_cache();
        assert!(runtime.loaded_names().is_empty(), "a runtime starts empty");
        let env = specforge_project::Environment::load(dir.path(), Some(&runtime));
        let loaded = runtime.loaded_names();
        assert_eq!(
            loaded,
            ["@specforge/product"],
            "only what the project enables"
        );
        assert!(env.enabled.iter().all(|e| e.failure.is_none()));
        let commands = ExtensionCommands::build(&env.registries);
        let features = commands
            .all()
            .iter()
            .find(|c| c.id() == "features")
            .expect("product declares `features`");

        let json = CommandContext {
            format: CommandFormat::Json,
            today: "2026-10-03".into(),
            ..CommandContext::default()
        };
        let out = run_command(
            &runtime,
            features,
            &env.build_graph(),
            &features.normalize(&Map::new()).unwrap(),
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
        let runtime = specforge_component::ComponentRuntime::new();
        specforge_component::builtins::load_builtins_for(&runtime, &["@specforge/product".into()])
            .unwrap();
        let unrouted = CommandDescriptor {
            export: "cmd__product_no_such_command".into(),
            ..command("no_such_command")
        };
        let err = run_command(
            &runtime,
            &ExtensionCommand::new("@specforge/product", "product", &unrouted),
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

    #[specforge_test(
        behavior = "auto_promote_commands_to_mcp_tools",
        verify = "auto-promoted tool name follows specforge.{ext}.{cmd} pattern"
    )]
    fn a_command_is_named_by_its_extension_short_name_and_dashed_id() {
        // The short name is the declared one (the SDK's `short`, on the wire
        // `ext_short`), else the name's last segment.
        let commands = ExtensionCommands::build(&build_of(&[
            ("@specforge/product", None, &["milestone_completion"]),
            ("@acme/widgets", Some("w"), &["list_all"]),
        ]));
        let named: Vec<(&str, String, String)> = commands
            .all()
            .iter()
            .map(|c| (c.short(), c.cli_name(), c.tool_name()))
            .collect();
        assert_eq!(
            named,
            [
                (
                    "product",
                    "milestone-completion".to_string(),
                    "specforge.product.milestone_completion".to_string()
                ),
                (
                    "w",
                    "list-all".to_string(),
                    "specforge.w.list_all".to_string()
                ),
            ]
        );
        // `specforge w list-all` routes to it; `specforge widgets` routes
        // nothing.
        assert_eq!(commands.shorts(), ["product", "w"]);
        assert_eq!(
            commands.of("w").map(|c| c.id()).collect::<Vec<_>>(),
            ["list_all"]
        );
        assert_eq!(commands.of("widgets").count(), 0);
    }
}
