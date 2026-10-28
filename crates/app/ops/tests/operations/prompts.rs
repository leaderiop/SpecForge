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
use specforge_project::testing::{CORE, GsdPrompts, NO_CONTEXT};
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
    let rendered = render_prompt(&view, prompt, given, None).map_err(|error| match error {
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
                {"id": "check", "title": "Check the plans"},
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
