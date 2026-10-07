//! How a core tool's arguments are listed and read (plan 08, ADR 0033).
//!
//! The first two tests pin today's behaviour, bugs included: the core
//! tools' listing as a client receives it, and what a call answers (and
//! writes) for each way of sending an argument. They are unlinked
//! characterisation tests; the ticket that changes a line of either
//! snapshot re-blesses it in its own commit, so the diff shows the
//! user-visible change.

use crate::support::*;
use serde_json::{Value, json};
use specforge_test::prelude::*;
use std::path::{Path, PathBuf};

#[test]
fn core_tool_listing_today() {
    let listing = serde_json::to_string_pretty(&core_tools()).expect("a listing");
    insta::assert_snapshot!(listing);
}

/// `main.spec`: `alpha`, misformatted on purpose, and `beta`, which refines it.
const MAIN: &str = concat!(
    "behavior alpha \"Alpha\" {\n",
    "      contract    \"The system MUST work\"\n",
    "}\n",
    "behavior beta \"Beta\" {\n",
    "  refines [alpha]\n",
    "}\n",
);

/// A fresh served project holding [`MAIN`], `@specforge/software` enabled.
fn served() -> Served {
    TestProject::new()
        .enabling(&["@specforge/software"])
        .file("main.spec", MAIN)
        .serve_components()
}

/// The text of a refusal: `refused <code> argument=<a>: <message>`.
fn refusal(error: &Value) -> String {
    format!(
        "refused {} argument={}: {}",
        error["code"].as_str().unwrap_or("?"),
        error["argument"].as_str().unwrap_or("-"),
        error["message"].as_str().unwrap_or("?"),
    )
}

/// The one fact a row records about a successful reply of `tool`.
fn fact(tool: &str, result: &Value, response: &Value) -> String {
    let count = |value: &Value| value.as_array().map_or(0, Vec::len);
    match tool {
        "specforge.format" => format!("check_only={}", result["check_only"]),
        "specforge.rename" => format!("dry_run={}", result["dry_run"]),
        "specforge.migrate" => format!("dry_run={}", result["dry_run"]),
        "specforge.query" | "specforge.export" => format!("nodes={}", count(&result["nodes"])),
        "specforge.search" => format!("results={}", count(result)),
        "specforge.validate" => {
            let verdict = &response["result"]["_meta"]["specforge/check"];
            format!(
                "ok={} errors={} warnings={}",
                verdict["ok"], verdict["errors"], verdict["warnings"]
            )
        }
        _ => String::new(),
    }
}

/// One tool call from a fresh project: `tool | arguments | outcome`.
fn tool_row(tool: &str, label: &str, arguments: impl FnOnce(&Path) -> Value) -> String {
    let mut served = served();
    let root = served.root().to_path_buf();
    let before = files_under(&root);
    let response = call_tool(&mut served, tool, arguments(&root));
    let after = files_under(&root);
    let wrote: Vec<PathBuf> = changed_files(&root, &before, &after);
    let outcome = if response["result"]["isError"] == true {
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        let error: Value = serde_json::from_str(text).unwrap_or_else(|_| json!({"message": text}));
        refusal(&error)
    } else if let Some(error) = response.get("error") {
        format!("refused rpc {error}")
    } else {
        let result = tool_json(&response);
        let fact = fact(tool, &result, &response);
        format!("ok {fact}").trim_end().to_string()
    };
    format!(
        "{} | {label} | {outcome} | wrote {:?}",
        tool.trim_start_matches("specforge."),
        wrote
    )
}

/// One prompt request from a fresh project.
fn prompt_row(prompt: &str, arguments: Value) -> String {
    let mut served = served();
    let label = arguments.to_string();
    let response = get_prompt(
        &mut served,
        &format!("specforge://prompts/{prompt}"),
        arguments,
    );
    let outcome = match response.get("error") {
        Some(error) => format!(
            "refused {} argument={}: {}",
            error["code"],
            error["data"]["argument"].as_str().unwrap_or("-"),
            error["message"].as_str().unwrap_or("?"),
        ),
        None => "ok".to_string(),
    };
    format!("prompt {prompt} | {label} | {outcome}")
}

#[test]
fn argument_reading_today() {
    let fixed = |value: Value| move |_: &Path| value.clone();
    let rows = [
        tool_row(
            "specforge.format",
            r#"{"check":true}"#,
            fixed(json!({"check": true})),
        ),
        tool_row(
            "specforge.format",
            r#"{"check":true,"write":true}"#,
            fixed(json!({"check": true, "write": true})),
        ),
        tool_row(
            "specforge.format",
            r#"{"check":"true"}"#,
            fixed(json!({"check": "true"})),
        ),
        tool_row(
            "specforge.format",
            r#"{"diff":"true"}"#,
            fixed(json!({"diff": "true"})),
        ),
        tool_row(
            "specforge.rename",
            r#"{"entity_id":"alpha","new_name":"gamma","dry_run":"true"}"#,
            fixed(json!({"entity_id": "alpha", "new_name": "gamma", "dry_run": "true"})),
        ),
        tool_row(
            "specforge.migrate",
            r#"{"dry_run":"true"}"#,
            fixed(json!({"dry_run": "true"})),
        ),
        tool_row(
            "specforge.query",
            r#"{"entity_id":"alpha","depth":"0"}"#,
            fixed(json!({"entity_id": "alpha", "depth": "0"})),
        ),
        tool_row(
            "specforge.query",
            r#"{"entity_id":"alpha","kinds":"behavior"}"#,
            fixed(json!({"entity_id": "alpha", "kinds": "behavior"})),
        ),
        tool_row(
            "specforge.search",
            r#"{"query":"a","limit":"1"}"#,
            fixed(json!({"query": "a", "limit": "1"})),
        ),
        tool_row(
            "specforge.list",
            r#"{"limit":"1"}"#,
            fixed(json!({"limit": "1"})),
        ),
        tool_row(
            "specforge.validate",
            r#"{"strict":"yes"}"#,
            fixed(json!({"strict": "yes"})),
        ),
        tool_row(
            "specforge.export",
            r#"{"format":"brief","scop":"alpha"}"#,
            fixed(json!({"format": "brief", "scop": "alpha"})),
        ),
        tool_row(
            "specforge.stats",
            r#"{"use_cached":true}"#,
            fixed(json!({"use_cached": true})),
        ),
        tool_row(
            "specforge.stats",
            r#"{"path":"<served root>"}"#,
            |root| json!({"path": root.to_str().expect("a UTF-8 root")}),
        ),
        tool_row(
            "specforge.doctor",
            r#"{"use_cached":"true"}"#,
            fixed(json!({"use_cached": "true"})),
        ),
        tool_row("specforge.infer_session", "{}", fixed(json!({}))),
        prompt_row("context", json!({"entity_id": 42})),
        prompt_row("review", json!({"entity_id": "alpha", "depth": "two"})),
        prompt_row("context", json!({"entity_id": "alpha", "bogus": "x"})),
    ];
    insta::assert_snapshot!(rows.join("\n"));
}

fn format_changes(arguments: Value) -> Vec<PathBuf> {
    let mut served = served();
    let root = served.root().to_path_buf();
    let before = files_under(&root);
    let reply = call_tool(&mut served, "specforge.format", arguments);
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    changed_files(&root, &before, &files_under(&root))
}

#[specforge_test(
    behavior = "provide_mcp_format_tool",
    verify = "the format tool advertises no default for write, and check or diff without write writes nothing"
)]
fn format_states_the_write_rule_not_a_default() {
    let format = core_tools()
        .into_iter()
        .find(|tool| tool.name == "specforge.format")
        .expect("the format tool");
    let write = &format.input_schema["properties"]["write"];
    assert_eq!(write["type"], "boolean");
    assert!(write.get("default").is_none(), "{write}");

    let none: Vec<PathBuf> = Vec::new();
    assert_eq!(format_changes(json!({"check": true})), none);
    assert_eq!(format_changes(json!({"diff": true})), none);
    assert_eq!(
        format_changes(json!({})),
        [PathBuf::from("main.spec")],
        "an absent write writes"
    );
}

// --- the derive, over probe structs ---

mod derived {
    use super::*;
    use specforge_mcp::args::{
        AgentPlan, Arg, Argument, Arguments, EntityIds, choice_schema, input_schema, read,
    };
    use specforge_mcp::target::TargetSpec;
    use specforge_ops::model::ModelFormat;
    use specforge_project::coverage::Status;
    use specforge_protocol_types::command_args::normalize_arg;
    use specforge_protocol_types::{CommandArgDescriptor, CommandArgType};

    #[derive(Debug, Arguments)]
    struct Probe {
        /// A name
        name: String,
        /// Hops
        #[arg(default = 3)]
        hops: usize,
        /// A flag
        flag: bool,
        /// Maybe
        maybe: Option<String>,
        /// Kinds
        kinds: Vec<String>,
        /// Format
        #[arg(choice = specforge_ops::model::MODEL_FORMAT)]
        format: ModelFormat,
        /// Status
        #[arg(choice = specforge_ops::coverage::STATUS)]
        status: Option<Status>,
        /// Profiles
        #[arg(names = &["a", "b"])]
        profiles: Vec<String>,
    }

    fn properties(declared: &[Argument]) -> Vec<(&'static str, Value)> {
        declared
            .iter()
            .map(|argument| (argument.name, argument.schema.clone()))
            .collect()
    }

    #[specforge_test(
        behavior = "read_mcp_arguments_as_declared",
        verify = "a listing is derived from the typed arguments: each argument's type, description, default, enumerated values and whether it is required"
    )]
    fn a_listing_is_derived_from_the_typed_arguments() {
        let declared = Probe::declared();
        assert_eq!(
            properties(&declared),
            [
                ("name", json!({"type": "string", "description": "A name"})),
                (
                    "hops",
                    json!({"type": "integer", "minimum": 0, "default": 3, "description": "Hops"})
                ),
                (
                    "flag",
                    json!({"type": "boolean", "default": false, "description": "A flag"})
                ),
                ("maybe", json!({"type": "string", "description": "Maybe"})),
                (
                    "kinds",
                    json!({"type": "array", "items": {"type": "string"}, "description": "Kinds"})
                ),
                (
                    "format",
                    choice_schema(&specforge_ops::model::MODEL_FORMAT, "Format")
                ),
                (
                    "status",
                    choice_schema(&specforge_ops::coverage::STATUS, "Status")
                ),
                (
                    "profiles",
                    json!({
                        "type": "array",
                        "items": {"type": "string", "enum": ["a", "b"]},
                        "description": "Profiles",
                    })
                ),
            ]
        );
        let required: Vec<&str> = declared
            .iter()
            .filter(|argument| argument.required)
            .map(|argument| argument.name)
            .collect();
        assert_eq!(required, ["name"]);

        let schema = input_schema(&declared, TargetSpec::SERVED);
        assert_eq!(schema["required"], json!(["name"]));
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["properties"].as_object().expect("properties").len(),
            8
        );
        assert_eq!(
            schema["properties"]["format"]["default"], "markdown",
            "a table's default is the argument's"
        );
        assert!(schema["properties"]["status"].get("default").is_none());
    }

    fn refusal(arguments: Value) -> specforge_mcp::tool::McpError {
        *read::<Probe>(&arguments).expect_err("a refusal")
    }

    #[specforge_test(
        behavior = "read_mcp_arguments_as_declared",
        verify = "an absent or null argument reads as the default the listing advertises, and a missing required one is refused naming it"
    )]
    fn an_absent_argument_is_its_default() {
        for given in [json!({"name": "x"}), json!({"name": "x", "hops": null})] {
            let probe: Probe = read(&given).expect("a read");
            assert_eq!(probe.name, "x");
            assert_eq!(probe.hops, 3);
            assert!(!probe.flag);
            assert_eq!(probe.maybe, None);
            assert!(probe.kinds.is_empty());
            assert_eq!(probe.format, ModelFormat::Markdown);
            assert_eq!(probe.status, None);
            assert!(probe.profiles.is_empty());
        }

        let error = refusal(json!({}));
        assert_eq!(error.code.as_str(), "invalid_input");
        assert_eq!(error.message, "Missing required parameter: name");
        assert_eq!(error.argument.as_deref(), Some("name"));
    }

    #[specforge_test(
        behavior = "read_mcp_arguments_as_declared",
        verify = "a boolean or count sent as a string is read as one, as an extension command reads it; any other value of the wrong type is refused naming the argument"
    )]
    fn a_value_is_read_by_its_type() {
        let probe: Probe = read(&json!({
            "name": "x", "hops": "2", "flag": "true", "kinds": ["a"], "maybe": "m",
            "format": "dot", "status": "partial", "profiles": ["a"],
        }))
        .expect("a read");
        assert_eq!((probe.hops, probe.flag), (2, true));
        assert_eq!(probe.format, ModelFormat::Dot);
        assert_eq!(probe.status, Some(Status::Partial));
        assert_eq!(probe.kinds, ["a"]);

        for (given, argument, message) in [
            (
                json!({"name": "x", "flag": "yes"}),
                "flag",
                "flag must be true or false, got 'yes'",
            ),
            (
                json!({"name": "x", "hops": -1}),
                "hops",
                "hops must be a non-negative integer, got -1",
            ),
            (
                json!({"name": "x", "hops": "two"}),
                "hops",
                "hops must be a non-negative integer, got 'two'",
            ),
            (json!({"name": 42}), "name", "name must be a string, got 42"),
            (
                json!({"name": "x", "kinds": "k"}),
                "kinds",
                "kinds must be a list of strings, got 'k'",
            ),
        ] {
            let error = refusal(given.clone());
            assert_eq!(error.code.as_str(), "invalid_input", "{given}");
            assert_eq!(error.message, message, "{given}");
            assert_eq!(error.argument.as_deref(), Some(argument), "{given}");
        }
    }

    /// `T`'s reading of each value is `normalize_arg`'s, value and wording.
    fn parity<T: Arg + std::fmt::Debug>(
        arg_type: CommandArgType,
        minimum: Option<i64>,
        values: &[Value],
        same: impl Fn(&T, &Value) -> bool,
    ) {
        let arg = CommandArgDescriptor {
            name: "x".into(),
            arg_type,
            required: false,
            default_value: None,
            description: None,
            minimum,
        };
        for value in values {
            match (normalize_arg(&arg, value), T::read("x", value)) {
                (Ok(normalized), Ok(read)) => {
                    assert!(
                        same(&read, &normalized),
                        "{value}: {read:?} vs {normalized}"
                    )
                }
                (Err(error), Err(message)) => assert_eq!(error.message(), message, "{value}"),
                (rule, read) => panic!("{value}: rule {rule:?}, adapter {read:?}"),
            }
        }
    }

    #[test]
    fn the_flag_count_and_string_adapters_are_the_commands_rule() {
        let values = [
            json!(true),
            json!(false),
            json!("true"),
            json!("false"),
            json!("yes"),
            json!(""),
            json!(0),
            json!(1),
            json!(2),
            json!(-1),
            json!("2"),
            json!("-1"),
            json!("two"),
            json!(1.5),
            json!([]),
            json!({}),
        ];
        parity::<bool>(CommandArgType::Bool, None, &values, |read, normalized| {
            normalized == &Value::Bool(*read)
        });
        parity::<usize>(
            CommandArgType::Integer,
            Some(0),
            &values,
            |read, normalized| normalized == &Value::from(*read),
        );
        parity::<String>(CommandArgType::String, None, &values, |read, normalized| {
            normalized == &Value::from(read.as_str())
        });
    }

    #[test]
    fn a_table_refusal_names_the_argument() {
        let error = refusal(json!({"name": "x", "format": "svg"}));
        assert_eq!(error.code.as_str(), "invalid_input");
        assert!(
            error.message.starts_with("Unknown format: svg. Expected: "),
            "{}",
            error.message
        );
        assert_eq!(error.argument.as_deref(), Some("format"));
        let close = refusal(json!({"name": "x", "format": "marcdown"}));
        assert_eq!(
            close.data.as_ref().map(|data| &data["suggestion"]),
            Some(&json!("did you mean 'markdown'?"))
        );
        let wrong_type = refusal(json!({"name": "x", "format": 5}));
        assert_eq!(wrong_type.argument.as_deref(), Some("format"));
    }

    #[derive(Debug, Arguments)]
    struct Raw {
        /// The where
        r#where: Option<serde_json::Map<String, Value>>,
        /// A plan
        plan: Option<AgentPlan>,
        /// Ids
        ids: EntityIds,
        /// A required renderer
        #[arg(choice = specforge_ops::export::FORMAT)]
        renderer: String,
    }

    #[test]
    fn the_derive_reads_fields_in_order_and_unraws_names() {
        let declared = Raw::declared();
        let names: Vec<&str> = declared.iter().map(|argument| argument.name).collect();
        assert_eq!(names, ["where", "plan", "ids", "renderer"]);
        assert_eq!(
            declared[0].schema,
            json!({"type": "object", "additionalProperties": true, "description": "The where"})
        );
        assert!(!declared[0].required && !declared[2].required && declared[3].required);

        let raw: Raw = read(&json!({
            "where": {"status": "done"}, "ids": "a, b,,c ", "renderer": "markdown",
        }))
        .expect("a read");
        assert_eq!(raw.r#where.expect("a where")["status"], "done");
        assert!(raw.plan.is_none());
        assert_eq!(raw.ids.0, ["a", "b", "c"]);
        assert_eq!(raw.renderer, "markdown");
        let missing = *read::<Raw>(&json!({})).expect_err("a refusal");
        assert_eq!(missing.message, "Missing required parameter: renderer");
    }
}
