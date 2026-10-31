//! The GSD-shaped extensions the K8 tests share (ADR 0291), served in process and written as the wire carries
//! them, so an attribute the host does not read yet is dropped by the descriptor and the test that needs it fails,
//! not the fixture:
//!
//! - [`CORE`] (`@t/core`, short name `t`): the kinds `milestone`, `phase` (scoped by a milestone, with a `goal`) and
//!   `plan` (scoped by a phase, with a `state`); the codes `E901`, `E902`, `E903`, `W904`; a check pass `audit`
//!   (a phase whose goal is [`NO_CONTEXT`] is `E901`, a plan in state `blocked` is `E902`, one `stale` is `W904`)
//!   and a guard pass `ship_guard` that reports `E903` for a plan in state `broken`, at a gate only; the settings
//!   `workflow.gates` (a bool gating `E901` and `E902`), `workflow.research` (a bool) and `profile` (`compat`, or
//!   `strict`, which raises `W904` to an error); the commands `record_plan`, `execute_phase` (gates `E902` and `W904`, subject
//!   `phase`) and `ship` (gate `E903`); the prompts `plan_phase` (a required `phase` reference, a `research`
//!   flag, a `depth` from 1 to 3, a `mode` and `labels`; gate `E901`, subject `phase`; a step that runs `record_plan`, one that runs `execute_phase` and one the
//!   agent `checker` does; the attribution of `gsd-core/workflows/plan-phase.md` at 1.18.0) and `research_phase`
//!   (served while `workflow.research` is not false)
//!   and the slash commands `t-plan-phase` and `t-research-phase` (the latter served while `workflow.research` and `workflow.gates` are both on)
//! - [`OTHER`] (`@t/other`, short name `t`): a command `plan_phase`, which a prompt of that route meets (E039)
//!
//! Every export answers from its input, so a test sees what the host sent: a command prints its `args`, a prompt
//! prints its `args`, `host`, `settings` and whether it got the graph. [`GsdPrompts::answering`] replaces one.

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta};
use specforge_wasm::WasmCallResult;
use specforge_wasm::testing::InProcessRuntime;

pub use super::gsd_writes::{CORE, OTHER};

/// The goal that makes `audit` report `E901` for a phase: it has no context.
pub const NO_CONTEXT: &str = "NOCONTEXT";

/// The two extensions' names, as a project enables them.
pub const GSD_PROMPTS_EXTENSIONS: [&str; 2] = [CORE, OTHER];

/// The sources of a project with phase 03 in milestone v1.0 and one plan, in `state`.
pub fn gsd_prompts_sources(goal: &str, state: &str) -> [(&'static str, String); 2] {
    [
        (
            "m.spec",
            format!(
                "milestone v1.0 \"One\" {{}}\n\nphase 03 \"Three\" {{\n  milestone v1.0\n  goal \"{goal}\"\n}}\n"
            ),
        ),
        (
            "03-01.spec",
            format!("plan 03-01 \"First\" {{\n  phase 03\n  state \"{state}\"\n}}\n"),
        ),
    ]
}

/// The kinds of [`CORE`], as the wire carries them.
fn kinds() -> Value {
    json!([
        {"name": "milestone", "keyword": "milestone"},
        {"name": "phase", "keyword": "phase", "scoped_by": "milestone", "fields": [
            {"name": "milestone", "field_type": "reference", "target_kind": "milestone"},
            {"name": "goal", "field_type": "string"},
        ]},
        {"name": "plan", "keyword": "plan", "scoped_by": "phase", "fields": [
            {"name": "phase", "field_type": "reference", "target_kind": "phase"},
            {"name": "state", "field_type": "string"},
        ]},
    ])
}

fn codes() -> Value {
    let code = |code: &str, title: &str| json!({"code": code, "title": title, "explanation": format!("{title}: a finding the gate tests read.")});
    json!([
        code("E901", "A phase has no context"),
        code("E902", "A plan is blocked"),
        code("E903", "A broken window is open"),
        code("W904", "A plan is stale"),
    ])
}

fn settings() -> Value {
    json!([
        {"kind": "setting", "name": "workflow.gates", "field_type": "bool", "default": true,
         "gates": ["E901", "E902"]},
        {"kind": "setting", "name": "workflow.research", "field_type": "bool", "default": true},
        {"kind": "setting", "name": "profile", "field_type": "enum", "enum_values": ["compat", "strict"],
         "default": "compat"},
        {"kind": "preset", "source": "profile", "values": {"strict": {"severities": {"W904": "error"}}}},
    ])
}

fn passes() -> Value {
    json!([
        {"name": "audit", "phase": "check"},
        {"name": "ship_guard", "phase": "guard", "reports": ["E903"]},
    ])
}

fn command(id: &str, args: Value, gates: &[&str], subject: Option<&str>) -> Value {
    let mut command = json!({
        "id": id, "title": id, "description": id, "export": format!("cmd__{id}"), "args": args,
    });
    if !gates.is_empty() {
        command["gates"] = json!(gates);
    }
    if let Some(subject) = subject {
        command["subject"] = json!(subject);
    }
    command
}

fn phase_arg() -> Value {
    json!([{"name": "phase", "arg_type": "string", "required": true}])
}

/// The prompts of [`CORE`], as the wire carries them.
fn prompts() -> Value {
    json!([
        {
            "id": "plan_phase", "title": "Plan a phase", "description": "Plan the phase, then check the plans",
            "export": "prompt__plan_phase",
            "args": [
                {"name": "phase", "field_type": "reference", "target_kind": "phase", "required": true,
                 "description": "The phase to plan"},
                {"name": "research", "field_type": "bool"},
                {"name": "depth", "field_type": "integer", "minimum": 1, "maximum": 3},
                {"name": "mode", "field_type": "enum", "enum_values": ["fast", "slow"]},
                {"name": "labels", "field_type": "string_list"},
            ],
            "gates": ["E901"], "subject": "phase",
            "steps": [
                {"id": "plan", "title": "Write the plans", "command": "record_plan"},
                {"id": "execute", "title": "Execute the plans", "command": "execute_phase"},
                {"id": "check", "title": "Check the plans", "agent": "checker"},
            ],
            "attribution": {
                "upstream": "gsd-core", "path": "gsd-core/workflows/plan-phase.md", "version": "1.18.0",
                "license": "MIT", "copyright": "Copyright (c) 2026 Open GSD",
            },
        },
        {
            "id": "research_phase", "title": "Research a phase", "description": "Research the phase",
            "export": "prompt__research_phase",
            "args": [{"name": "phase", "field_type": "reference", "target_kind": "phase", "required": true}],
            "when": "workflow.research",
        },
    ])
}

fn surfaces(extra: &[Value]) -> Value {
    let mut prompts = prompts();
    prompts
        .as_array_mut()
        .expect("a list")
        .extend(extra.iter().cloned());
    json!([{
        "commands": [
            command("record_plan", phase_arg(), &[], None),
            command("execute_phase", phase_arg(), &["E902", "W904"], Some("phase")),
            command("ship", json!([]), &["E903"], None),
            // Declares an arg named `host`, which the CLI's own `--host` must leave to it.
            command("name_host", json!([{"name": "host", "arg_type": "string"}]), &[], None),
        ],
        "prompts": prompts,
        "agents": [{"id": "checker", "description": "Checks the plans"}],
        "slash_commands": [
            {"prompt": "plan_phase"},
            {"prompt": "research_phase", "when": ["workflow.research", "workflow.gates"]},
        ],
    }])
}

fn core(extra: &[Value]) -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta {
        short: Some("t".into()),
        ..ExtensionMeta::new(CORE, "0.4.0")
    });
    c.raw_category(DeclaredCategory::Entities, kinds());
    c.raw_category(DeclaredCategory::Codes, codes());
    c.raw_category(DeclaredCategory::Settings, settings());
    c.raw_category(DeclaredCategory::Passes, passes());
    c.raw_category(DeclaredCategory::Surfaces, surfaces(extra));
    c
}

fn other() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta {
        short: Some("t".into()),
        ..ExtensionMeta::new(OTHER, "1.0.0")
    });
    c.raw_category(
        DeclaredCategory::Surfaces,
        json!([{"commands": [command("plan_phase", json!([]), &[], None)]}]),
    );
    c
}

fn field<'a>(entity: &'a Value, name: &str) -> Option<&'a str> {
    entity["fields"][name].as_str()
}

fn finding(code: &str, severity: &str, message: &str, entity: &Value) -> Value {
    json!({"code": code, "severity": severity, "message": message, "entity": entity["id"]})
}

/// `audit`: what the check pass finds in the entities it is handed.
fn audit(input: &Value) -> Vec<Value> {
    let mut found = Vec::new();
    for entity in input["entities"].as_array().into_iter().flatten() {
        match (
            entity["kind"].as_str(),
            field(entity, "state"),
            field(entity, "goal"),
        ) {
            (Some("phase"), _, Some(NO_CONTEXT)) => {
                found.push(finding("E901", "Error", "the phase has no context", entity))
            }
            (Some("plan"), Some("blocked"), _) => {
                found.push(finding("E902", "Error", "the plan is blocked", entity))
            }
            (Some("plan"), Some("stale"), _) => {
                found.push(finding("W904", "Warning", "the plan is stale", entity))
            }
            _ => {}
        }
    }
    found
}

/// `ship_guard`: a broken window is open while a plan is `broken`; asked at a gate only, and silent on a write.
fn ship_guard(input: &Value) -> Vec<Value> {
    if input["trigger"]["kind"] != "gate" {
        return Vec::new();
    }
    input["entities"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["kind"] == "plan" && field(e, "state") == Some("broken"))
        .map(|e| finding("E903", "Error", "a broken window is open", e))
        .collect()
}

/// The args a command got, one line; and, when it was told the host that called it, `host <name>`.
fn command_output(input: &Value) -> Value {
    let host = input["host"]
        .as_str()
        .map_or(String::new(), |h| format!("host {h}\n"));
    json!({"exit_code": 0, "stdout": format!("{}\n{host}", input["args"]), "stderr": ""})
}

fn prompt_output(export: &str, input: &Value) -> Value {
    let mut echoed = json!({
        "args": input["args"], "host": input["host"], "settings": input["settings"],
        "graph": input.get("graph").is_some(),
    });
    // What a prompt that reads state or the served slash commands was sent; absent otherwise.
    for key in ["state", "slash_commands"] {
        if let Some(value) = input.get(key) {
            echoed[key] = value.clone();
        }
    }
    json!({"messages": [{"role": "user", "text": format!("{export} {echoed}")}]})
}

fn handle(export: &str, input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    let input: Value = serde_json::from_slice(input).unwrap_or(Value::Null);
    let answer = match export {
        "__pass_audit" => json!({"diagnostics": audit(&input), "summary": {}, "values": []}),
        "__pass_ship_guard" => {
            json!({"diagnostics": ship_guard(&input), "summary": {}, "values": []})
        }
        "cmd__record_plan" | "cmd__execute_phase" | "cmd__ship" | "cmd__name_host" => {
            command_output(&input)
        }
        export if export.starts_with("prompt__") => prompt_output(export, &input),
        _ => return None,
    };
    Some(Ok(answer.to_string().into_bytes()))
}

/// The two extensions, served in process.
pub struct GsdPrompts {
    answers: Vec<(String, String, WasmCallResult)>,
    prompts: Vec<Value>,
}

impl GsdPrompts {
    pub fn new() -> Self {
        GsdPrompts {
            answers: Vec::new(),
            prompts: Vec::new(),
        }
    }

    /// [`CORE`] also declares the prompt `descriptor` (as the wire carries it); its export echoes what it is sent.
    pub fn prompting(mut self, descriptor: Value) -> Self {
        self.prompts.push(descriptor);
        self
    }

    /// `export` of `extension` answers `result`, whatever it is sent.
    pub fn answering(mut self, extension: &str, export: &str, result: WasmCallResult) -> Self {
        self.answers
            .push((extension.to_string(), export.to_string(), result));
        self
    }

    /// The runtime serving [`CORE`] and [`OTHER`] and the answers given.
    pub fn runtime(self) -> InProcessRuntime {
        let extra = self.prompts;
        let mut runtime = InProcessRuntime::new()
            .with_handler(move || core(&extra), handle)
            .with(other);
        for (extension, export, result) in self.answers {
            runtime = runtime.answer_raw(&extension, &export, result);
        }
        runtime
    }
}

impl Default for GsdPrompts {
    fn default() -> Self {
        Self::new()
    }
}
