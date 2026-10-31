//! A prompt's render (ADR 0291): its typed arguments read by their field types, its entry gate, its export's
//! messages, and the host message that follows them. The fixture is `@t/core` ([`specforge_project::testing::GsdPrompts`]);
//! its prompts print the args, host and settings they got.
//!
//! [`render`] is the one place a test renders a prompt.

use serde_json::{Value, json};
use specforge_ops::OpErrorKind;
use specforge_ops::extension::prompt::{
    ExtensionPrompts, PromptArgs, RenderError, render as render_prompt,
};
use specforge_ops::view::ProjectView;
use specforge_package::Sha256;
use specforge_project::testing::{CORE, GsdPrompts, NO_CONTEXT, TestProject};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::{WasmCallResult, WasmTrapInfo};

use crate::gates::{Refused, Served, project, served, served_with};

/// The arguments a prompt is given.
pub enum Args<'a> {
    /// By name, as MCP sends them (every value a string) or `specforge.prompt` (any JSON value).
    Named(Value),
    /// A `$ARGUMENTS` string or the command line's words.
    Argv(&'a str),
}

/// A rendered prompt: the guest's messages, and the host's payload message parsed.
#[derive(Debug)]
pub struct Rendered {
    pub messages: Vec<String>,
    pub payload: Value,
}

/// Render the prompt `id` of the fixture over `served`.
pub fn render(served: &Served, id: &str, args: Args<'_>) -> Result<Rendered, Refused> {
    render_for(served, id, args, None)
}

/// [`render`] for `host`.
pub fn render_for(
    served: &Served,
    id: &str,
    args: Args<'_>,
    host: Option<specforge_ops::host::Host>,
) -> Result<Rendered, Refused> {
    let view = ProjectView::of(&served.compiled);
    let prompts = ExtensionPrompts::build(&view);
    let prompt = prompts
        .by_mcp_name(&format!("specforge.t.{id}"))
        .unwrap_or_else(|| panic!("no prompt t {id}"));
    let words;
    let given = match &args {
        Args::Named(named) => PromptArgs::Named(named.as_object().unwrap()),
        Args::Argv(argv) => {
            words = specforge_ops::extension::prompt::words(argv).map_err(|e| Refused {
                code: e.code.to_string(),
                data: json!({"message": e.message}),
            })?;
            PromptArgs::Words(&words)
        }
    };
    let rendered = render_prompt(&view, prompt, given, host).map_err(|error| match error {
        RenderError::Refused(e) | RenderError::Gate(e) => Refused {
            code: if e.kind == OpErrorKind::EntityNotFound {
                "entity_not_found".into()
            } else {
                e.code.to_string()
            },
            data: e.data.map_or_else(|| json!({"message": e.message}), |d| *d),
        },
        RenderError::Call(e) => Refused {
            code: e.diagnostic().code,
            data: json!({"message": e.diagnostic().message}),
        },
    })?;
    Ok(Rendered {
        messages: rendered.messages.into_iter().map(|m| m.text).collect(),
        payload: serde_json::to_value(rendered.payload).unwrap(),
    })
}

fn plan_phase(served: &Served, args: Args<'_>) -> Result<Rendered, Refused> {
    render(served, "plan_phase", args)
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt argument is read by its field type and a reference is sent as its full path"
)]
fn a_prompt_argument_is_read_by_its_field_type() {
    let served = served(project("fine", "ok", json!({})));
    let sent = |served: &Served| served.input("prompt__plan_phase")["args"].clone();

    plan_phase(
        &served,
        Args::Named(json!({
            "phase": "03", "research": "true", "depth": "2", "mode": "fast", "labels": "a,b",
        })),
    )
    .expect("every value is read by its type");
    let expected = json!({
        "phase": "v1.0/03", "research": true, "depth": 2, "mode": "fast", "labels": ["a", "b"],
    });
    assert_eq!(sent(&served), expected);

    plan_phase(
        &served,
        Args::Argv("03 --research --depth 2 --mode fast --labels a,b"),
    )
    .unwrap();
    assert_eq!(sent(&served), expected, "the command line reads the same");

    plan_phase(
        &served,
        Args::Named(json!({"phase": "03", "research": true, "depth": 3})),
    )
    .unwrap();
    assert_eq!(
        sent(&served),
        json!({"phase": "v1.0/03", "research": true, "depth": 3}),
        "a JSON value is read as itself, and an absent argument is absent"
    );
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt argument outside its field type is invalid input on that argument"
)]
fn a_prompt_argument_outside_its_form_is_invalid_input() {
    let served = served(project("fine", "ok", json!({})));
    let cases = [
        (json!({"phase": "03", "research": "maybe"}), "research"),
        (json!({"phase": "03", "depth": "9"}), "depth"),
        (json!({"phase": "03", "mode": "slowish"}), "mode"),
        (json!({"phase": "99"}), "phase"),
        (json!({"phase": "03", "bogus": "x"}), "bogus"),
        (json!({}), "phase"),
    ];
    for (args, name) in cases {
        let refused = plan_phase(&served, Args::Named(args.clone())).unwrap_err();
        assert!(
            ["invalid_input", "entity_not_found"].contains(&refused.code.as_str()),
            "{args}: {refused:?}"
        );
        assert!(
            refused.data.to_string().contains(name),
            "{args}: the refusal names '{name}': {refused:?}"
        );
    }
    assert!(!served.called("prompt__plan_phase"), "no export was called");
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt whose entry gate fails is refused and its export is not called"
)]
fn a_prompt_whose_entry_gate_fails_is_refused() {
    let args = || Args::Named(json!({"phase": "03"}));
    let empty = served(project(NO_CONTEXT, "ok", json!({})));
    let refused =
        plan_phase(&empty, args()).expect_err("a phase with no context cannot be planned");
    assert_eq!(refused.code, "gate_failed");
    assert_eq!(refused.data["failing"][0]["code"], "t/E901", "{refused:?}");
    assert!(
        !empty.called("prompt__plan_phase"),
        "the export is never called"
    );

    let off = served(project(NO_CONTEXT, "ok", json!({"workflow.gates": false})));
    plan_phase(&off, args()).expect("a gate its setting turns off passes");
    assert!(off.called("prompt__plan_phase"));
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a rendered prompt shows its version, digest, attribution and each step with its gate"
)]
fn a_rendered_prompt_shows_each_steps_gate_and_its_attribution() {
    let served = served(project("fine", "blocked", json!({})));
    let args = || Args::Named(json!({"phase": "03"}));
    let rendered = plan_phase(&served, args()).expect("the entry gate is E901, which holds");

    let digest = format!(
        "sha256:{}",
        Sha256::of(rendered.messages.concat().as_bytes())
    );
    assert_eq!(
        rendered.payload,
        json!({
            "prompt": "@t/core@0.4.0:plan_phase",
            "digest": digest,
            "attribution": {
                "upstream": "gsd-core", "path": "gsd-core/workflows/plan-phase.md", "version": "1.18.0",
                "license": "MIT", "copyright": "Copyright (c) 2026 Open GSD",
            },
            "subject": "v1.0/03",
            "steps": [
                {"id": "plan", "title": "Write the plans", "tool": "specforge.t.record_plan",
                 "cli": "specforge t record-plan", "gate": []},
                {"id": "execute", "title": "Execute the plans", "tool": "specforge.t.execute_phase",
                 "cli": "specforge t execute-phase", "gate": [
                    {"code": "t/E902", "off": false, "passes": false},
                    {"code": "t/W904", "off": false, "passes": true},
                ]},
                {"id": "check", "title": "Check the plans", "agent": "checker"},
            ],
        })
    );
    let again = plan_phase(&served, args()).unwrap();
    assert_eq!(
        again.payload["digest"], rendered.payload["digest"],
        "the digest is stable"
    );
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a command step's cli names the host the prompt is rendered for"
)]
fn a_command_steps_cli_names_the_host() {
    use specforge_ops::host::Host;
    let served = served(project("fine", "ok", json!({})));
    let cli = |host| {
        let args = Args::Named(json!({"phase": "03"}));
        let rendered = render_for(&served, "plan_phase", args, host).unwrap();
        rendered.payload["steps"][0]["cli"].clone()
    };
    assert_eq!(
        cli(Some(Host::Codex)),
        "specforge t record-plan --host codex"
    );
    assert_eq!(cli(None), "specforge t record-plan");
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a rendered step names the agent that runs it, and a step without one has no agent key"
)]
fn a_step_without_an_agent_has_no_agent_key() {
    let served = served(project("fine", "ok", json!({})));
    let rendered = plan_phase(&served, Args::Named(json!({"phase": "03"}))).unwrap();
    let steps = rendered.payload["steps"].as_array().unwrap();
    assert!(steps[0].get("agent").is_none(), "{:?}", steps[0]);
    assert_eq!(steps[2]["agent"], "checker");
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt export that traps is E028"
)]
fn a_prompt_export_that_traps_is_e028() {
    let trap = WasmCallResult::Trap(WasmTrapInfo {
        kind: "trap".into(),
        message: "unreachable".into(),
        export_name: "prompt__plan_phase".into(),
    });
    let fixture = GsdPrompts::new().answering(CORE, "prompt__plan_phase", trap);
    let served = served_with(project("fine", "ok", json!({})), fixture);
    let refused = plan_phase(&served, Args::Named(json!({"phase": "03"}))).unwrap_err();
    assert_eq!(refused.code, "E028");
    assert!(
        refused.data["message"]
            .as_str()
            .unwrap_or_default()
            .starts_with("MCP prompt prompt__plan_phase() of '@t/core' trapped"),
        "{refused:?}"
    );
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a reference argument names an entity of its target kind and a quoted word is one word"
)]
fn a_reference_argument_must_name_its_target_kind() {
    let served = served(project("fine", "ok", json!({})));
    let refused = plan_phase(&served, Args::Named(json!({"phase": "v1.0"}))).unwrap_err();
    assert_eq!(refused.code, "invalid_input", "{refused:?}");
    assert_eq!(refused.data["argument"], "phase");
    assert!(!served.called("prompt__plan_phase"));

    assert_eq!(
        specforge_ops::extension::prompt::words(r#"03 --labels "a b",c"#).unwrap(),
        ["03", "--labels", "a b,c"]
    );
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "an unterminated quote is refused, a quoted word is a value, --name=value and -- are read, and null or an empty string is an unfilled optional"
)]
fn the_words_of_a_command_line_are_read_exactly() {
    let served = served(project("fine", "ok", json!({})));
    let sent = |served: &Served| served.input("prompt__plan_phase")["args"].clone();

    let open = plan_phase(&served, Args::Argv(r#"03 --labels "a b"#)).unwrap_err();
    assert_eq!(open.code, "invalid_input", "{open:?}");

    plan_phase(&served, Args::Argv(r#"03 --labels "--research""#)).unwrap();
    assert_eq!(
        sent(&served),
        json!({"phase": "v1.0/03", "labels": ["--research"]}),
        "a quoted word is a value, never an option"
    );
    plan_phase(&served, Args::Argv("03 --depth=2 --mode=fast")).unwrap();
    assert_eq!(
        sent(&served),
        json!({"phase": "v1.0/03", "depth": 2, "mode": "fast"})
    );
    plan_phase(&served, Args::Argv("--depth 1 -- 03")).unwrap();
    assert_eq!(sent(&served), json!({"phase": "v1.0/03", "depth": 1}));

    plan_phase(
        &served,
        Args::Named(json!({"phase": "03", "depth": null, "mode": "", "labels": ""})),
    )
    .expect("an unfilled optional is absent");
    assert_eq!(sent(&served), json!({"phase": "v1.0/03"}));
    let required = plan_phase(&served, Args::Named(json!({"phase": null}))).unwrap_err();
    assert_eq!(required.code, "invalid_input", "{required:?}");
}

/// The fixture with the prompt `quick`: a `--full` flag and a free-text tail `task`.
fn quick() -> Served {
    let quick = json!({
        "id": "quick", "title": "Quick", "description": "A quick task", "export": "prompt__quick",
        "args": [
            {"name": "full", "field_type": "bool"},
            {"name": "task", "field_type": "string", "rest": true},
        ],
    });
    served_with(
        project("fine", "ok", json!({})),
        GsdPrompts::new().prompting(quick),
    )
}

fn quick_args(served: &Served, argv: &str) -> Result<Value, Refused> {
    render(served, "quick", Args::Argv(argv))?;
    Ok(served.input("prompt__quick")["args"].clone())
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt's free-text tail takes the words no option takes, joined by one space"
)]
fn the_tail_takes_the_words_no_option_takes() {
    let served = quick();
    assert_eq!(
        quick_args(&served, "--full fix the login bug").unwrap(),
        json!({"full": true, "task": "fix the login bug"})
    );
    assert_eq!(
        quick_args(&served, r#""fix  it" now"#).unwrap(),
        json!({"task": "fix  it now"})
    );
    assert_eq!(
        quick_args(&served, "fix --full it").unwrap(),
        json!({"full": true, "task": "fix it"})
    );
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt's tail given by name and by words is refused, an unknown option is not tail, and after -- every word is tail"
)]
fn the_tail_given_twice_or_swallowing_an_option_is_refused() {
    let served = quick();
    let twice = quick_args(&served, "--task a b").unwrap_err();
    assert_eq!(twice.code, "invalid_input");
    assert_eq!(twice.data["argument"], "task");
    let unknown = quick_args(&served, "--nope x").unwrap_err();
    assert_eq!(unknown.code, "invalid_input");
    assert_eq!(unknown.data["argument"], "nope");
    assert_eq!(
        quick_args(&served, "-- --full x").unwrap(),
        json!({"task": "--full x"})
    );
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a named (MCP) tail is an ordinary string"
)]
fn a_named_tail_is_an_ordinary_string() {
    let served = quick();
    render(&served, "quick", Args::Named(json!({"task": "do it"}))).unwrap();
    assert_eq!(
        served.input("prompt__quick")["args"],
        json!({"task": "do it"})
    );
}

/// The fixture with the prompt `next_phase`: an optional `phase` subject (gate `E901`) its export may pick.
fn next_phase(project: TestProject, answer: Option<Value>) -> Served {
    let descriptor = json!({
        "id": "next_phase", "title": "Next", "description": "Plan the next phase",
        "export": "prompt__next_phase",
        "args": [{"name": "phase", "field_type": "reference", "target_kind": "phase"}],
        "gates": ["E901"], "subject": "phase",
    });
    let mut fixture = GsdPrompts::new().prompting(descriptor);
    if let Some(answer) = answer {
        let bytes = answer.to_string().into_bytes();
        fixture = fixture.answering(CORE, "prompt__next_phase", WasmCallResult::Ok(bytes));
    }
    served_with(project, fixture)
}

fn picks(subject: Option<&str>) -> Option<Value> {
    let mut answer = json!({"messages": [{"role": "user", "text": "next"}]});
    if let Some(subject) = subject {
        answer["subject"] = json!(subject);
    }
    Some(answer)
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt with an optional subject and none given renders for the subject its export picks"
)]
fn an_absent_subject_is_picked_by_the_export() {
    let served = next_phase(project("fine", "ok", json!({})), picks(Some("v1.0/03")));
    let rendered = render(&served, "next_phase", Args::Named(json!({}))).unwrap();
    assert_eq!(rendered.payload["subject"], "v1.0/03");
    assert!(
        served.input("prompt__next_phase")["args"]
            .get("phase")
            .is_none(),
        "the export is asked before a subject is known"
    );
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a picked subject meets the prompt's gate, and a given subject wins over the pick"
)]
fn a_picked_subject_meets_its_gate_and_a_given_one_wins() {
    let blocked = next_phase(project(NO_CONTEXT, "ok", json!({})), picks(Some("v1.0/03")));
    let refused = render(&blocked, "next_phase", Args::Named(json!({}))).unwrap_err();
    assert_eq!(refused.code, "gate_failed", "{refused:?}");
    assert!(refused.data.to_string().contains("t/E901"), "{refused:?}");

    let served = next_phase(project("fine", "ok", json!({})), picks(Some("v1.0/04")));
    let rendered = render(&served, "next_phase", Args::Named(json!({"phase": "03"}))).unwrap();
    assert_eq!(rendered.payload["subject"], "v1.0/03");
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a picked subject must be an entity of the subject argument's kind, else the answer is E028"
)]
fn a_picked_subject_must_be_of_the_subjects_kind() {
    for wrong in ["v1.0/03/03-01", "v1.0/99"] {
        let served = next_phase(project("fine", "ok", json!({})), picks(Some(wrong)));
        let refused = render(&served, "next_phase", Args::Named(json!({}))).unwrap_err();
        assert_eq!(refused.code, "E028", "{wrong}: {refused:?}");
    }
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt with an optional subject and no pick is project-wide, and the first word is the subject"
)]
fn no_picked_subject_is_project_wide_and_a_word_is_the_subject() {
    let served = next_phase(project("fine", "ok", json!({})), picks(None));
    let rendered = render(&served, "next_phase", Args::Named(json!({}))).unwrap();
    assert!(rendered.payload.get("subject").is_none(), "{rendered:?}");

    let echo = next_phase(project("fine", "ok", json!({})), None);
    let rendered = render(&echo, "next_phase", Args::Argv("03")).unwrap();
    assert_eq!(rendered.payload["subject"], "v1.0/03");
    assert_eq!(echo.input("prompt__next_phase")["args"]["phase"], "v1.0/03");
}

/// The fixture with the prompt `router`, which reads the slash commands the project serves.
fn router(settings: Value) -> Served {
    let descriptor = json!({
        "id": "router", "title": "Router", "description": "Routes", "export": "prompt__router",
        "args": [], "reads_slash_commands": true,
    });
    served_with(
        project("fine", "ok", settings),
        GsdPrompts::new().prompting(descriptor),
    )
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt that reads the slash commands is sent the ones the project serves, as an install writes them"
)]
fn a_prompt_reads_the_served_slash_commands() {
    let served = router(json!({}));
    render(&served, "router", Args::Named(json!({}))).unwrap();
    assert_eq!(
        served.input("prompt__router")["slash_commands"],
        json!([
            {"name": "t-plan-phase", "extension": "@t/core", "prompt": "specforge.t.plan_phase",
             "cli": "specforge prompt t.plan_phase"},
            {"name": "t-research-phase", "extension": "@t/core", "prompt": "specforge.t.research_phase",
             "cli": "specforge prompt t.research_phase"},
        ])
    );
    // A slash whose `when` setting is off is not served.
    let off = router(json!({"workflow.research": false}));
    render(&off, "router", Args::Named(json!({}))).unwrap();
    let sent = off.input("prompt__router");
    let names: Vec<&str> = sent["slash_commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["t-plan-phase"]);
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt that does not ask for the slash commands is sent none"
)]
fn a_prompt_that_does_not_ask_gets_none() {
    let served = served(project("fine", "ok", json!({})));
    plan_phase(&served, Args::Named(json!({"phase": "03"}))).unwrap();
    assert!(
        served
            .input("prompt__plan_phase")
            .get("slash_commands")
            .is_none()
    );
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "a contribution whose when names several settings is served only while all of them are on"
)]
fn a_slash_is_served_while_every_when_setting_is_on() {
    let names = |settings: Value| -> Vec<String> {
        let served = router(settings);
        render(&served, "router", Args::Named(json!({}))).unwrap();
        served.input("prompt__router")["slash_commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["name"].as_str().unwrap().to_string())
            .collect()
    };
    let both = ["t-plan-phase", "t-research-phase"];
    assert_eq!(names(json!({})), both);
    assert_eq!(names(json!({"workflow.research": false})), ["t-plan-phase"]);
    assert_eq!(names(json!({"workflow.gates": false})), ["t-plan-phase"]);
}
