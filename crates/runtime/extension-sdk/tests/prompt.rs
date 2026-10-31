//! Prompts, gates and reported codes as an extension declares them (ADR 0291): what reaches the wire, and the export
//! that renders the prompt.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;
use specforge_test_macros::test as specforge_test;

fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/flow", "1.0.0"));
    c.command("execute_phase", |cmd| {
        cmd.title("Execute")
            .arg("phase", |a| {
                a.required();
            })
            .gate("E902")
            .gate("W904")
            .subject("phase")
            .handler(|call| call.render(&json!({}), |_| {}));
    });
    c.pass("ship_guard", |p| {
        p.phase("guard")
            .reports(&["E903"])
            .run(|_| Vec::<PassDiagnostic>::new());
    });
    c.prompt("plan_phase", |p| {
        p.title("Plan a phase")
            .description("Plan it")
            .arg("phase", |a| {
                a.reference("phase").required().description("The phase");
            })
            .arg("depth", |a| {
                a.integer().minimum(1).maximum(3);
            })
            .arg("mode", |a| {
                a.one_of(&["fast", "slow"]).default("fast");
            })
            .gate("E901")
            .subject("phase")
            .step("plan", |s| {
                s.title("Plan").command("execute_phase");
            })
            .when("workflow.research")
            .attribution(Attribution {
                upstream: "gsd-core".into(),
                path: "gsd-core/workflows/plan-phase.md".into(),
                version: "1.18.0".into(),
                license: "MIT".into(),
                copyright: "Copyright (c) 2026 Open GSD".into(),
            })
            .handler(|call| {
                Ok(PromptOutput::user(format!(
                    "{} {:?} {:?} {}",
                    call.str("phase").unwrap_or("-"),
                    call.integer("depth"),
                    call.host(),
                    call.str("mode").unwrap_or("-"),
                )))
            });
    });
    c
}

#[specforge_test(
    behavior = "gate_a_command",
    verify = "the SDK declares a command's gates and subject, a guard pass's reports and a prompt on the wire"
)]
fn gates_reports_and_prompts_reach_the_wire() {
    let declaration = extension().declaration();
    let command = &declaration.surfaces.commands[0];
    assert_eq!(command.gates, ["E902", "W904"]);
    assert_eq!(command.subject.as_deref(), Some("phase"));
    assert_eq!(declaration.passes[0].reports, ["E903"]);
    let wire = serde_json::to_value(&declaration.surfaces.prompts).unwrap();
    assert_eq!(
        wire,
        json!([{
            "id": "plan_phase", "title": "Plan a phase", "description": "Plan it",
            "export": "prompt__plan_phase",
            "args": [
                {"name": "phase", "field_type": "reference", "target_kind": "phase", "required": true,
                 "description": "The phase"},
                {"name": "depth", "field_type": "integer", "minimum": 1, "maximum": 3},
                {"name": "mode", "field_type": "enum", "enum_values": ["fast", "slow"], "default": "fast"},
            ],
            "gates": ["E901"], "subject": "phase",
            "steps": [{"id": "plan", "title": "Plan", "command": "execute_phase"}],
            "when": "workflow.research",
            "attribution": {"upstream": "gsd-core", "path": "gsd-core/workflows/plan-phase.md",
                            "version": "1.18.0", "license": "MIT", "copyright": "Copyright (c) 2026 Open GSD"},
        }])
    );
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt declared with the SDK is rendered by its export from the arguments and host the call carries"
)]
fn a_declared_prompt_is_rendered_by_its_export() {
    let input = json!({
        "args": {"phase": "v1.0/03", "depth": 2, "mode": "slow"},
        "cwd": "/p", "today": "2026-10-28", "host": "codex",
    });
    let answer = extension()
        .dispatch_export("prompt__plan_phase", input.to_string().as_bytes())
        .expect("the export is routed")
        .unwrap();
    let answer: Value = serde_json::from_slice(&answer).unwrap();
    assert_eq!(
        answer,
        json!({"messages": [{"role": "user", "text": "v1.0/03 Some(2) Some(\"codex\") slow"}]})
    );
    let refused = extension()
        .dispatch_export("prompt__plan_phase", b"nope")
        .unwrap();
    assert!(refused.unwrap_err().starts_with("invalid prompt input"));
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "the SDK refuses a prompt declared twice or without a handler"
)]
fn a_prompt_declared_twice_or_without_a_handler_panics() {
    let twice = std::panic::catch_unwind(|| {
        let mut c = extension();
        c.prompt("plan_phase", |p| {
            p.handler(|_| Ok(PromptOutput::user("x")));
        });
    });
    assert!(twice.is_err());
    let bare = std::panic::catch_unwind(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/flow", "1.0.0"));
        c.prompt("p", |_| {});
    });
    assert!(bare.is_err());
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt reads the state the host sends it"
)]
fn a_prompt_reads_the_state_it_is_sent() {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/flow", "1.0.0"));
    c.prompt("status", |p| {
        p.title("Status")
            .description("Status")
            .reads_state()
            .handler(|call| {
                let active = call
                    .state()
                    .and_then(|state| state.project.get("active"))
                    .and_then(Value::as_str);
                Ok(PromptOutput::user(format!("active {active:?}")))
            });
    });
    let text = |input: Value| -> String {
        let answer = c
            .dispatch_export("prompt__status", input.to_string().as_bytes())
            .unwrap()
            .unwrap();
        let answer: Value = serde_json::from_slice(&answer).unwrap();
        answer["messages"][0]["text"].as_str().unwrap().to_string()
    };
    let base = json!({"args": {}, "cwd": "/p", "today": "2026-10-31"});
    let mut sent = base.clone();
    sent["state"] = json!({"project": {"active": "a"}});
    assert_eq!(text(sent), r#"active Some("a")"#);
    assert_eq!(text(base), "active None");
}

#[specforge_test(
    behavior = "gate_a_command",
    verify = "an extension compares a pass's trigger kind to the SDK's TriggerKind"
)]
fn the_prelude_names_the_trigger_kind() {
    assert_eq!(TriggerKind::default(), TriggerKind::Write);
    let input = PassInput {
        trigger: Some(Trigger::gate("execute_phase", None)),
        ..Default::default()
    };
    assert!(input.trigger.is_some_and(|t| t.kind == TriggerKind::Gate));
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "the SDK declares a prompt's free-text tail as a string argument"
)]
fn a_rest_argument_is_a_string_tail_on_the_wire() {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/flow", "1.0.0"));
    c.prompt("quick", |p| {
        p.title("Quick")
            .description("Quick")
            .arg("task", |a| {
                a.rest();
            })
            .handler(|_| Ok(PromptOutput::user("x")));
    });
    let wire = serde_json::to_value(&c.declaration().surfaces.prompts[0].args).unwrap();
    assert_eq!(
        wire,
        json!([{"name": "task", "field_type": "string", "rest": true}])
    );
}
