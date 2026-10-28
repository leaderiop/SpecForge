//! Which inputs carry the clock (ADR 0294): a command, a hook and a writing collector carry `now`; a prompt, a render
//! and a pass do not. `today` is kept beside it.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::{Value, json};
use specforge_protocol_types::{
    CollectInput, CommandInput, HookInput, PassInput, PromptInput, RawGraph, RenderInput,
};

fn keys(value: impl serde::Serialize) -> Vec<String> {
    let value: Value = serde_json::to_value(value).unwrap();
    value.as_object().unwrap().keys().cloned().collect()
}

#[test]
fn now_is_not_sent_while_empty() {
    assert!(!keys(CommandInput::<RawGraph>::default()).contains(&"now".to_string()));
    assert!(!keys(HookInput::<RawGraph>::default()).contains(&"now".to_string()));
    assert!(!keys(CollectInput::<RawGraph>::default()).contains(&"now".to_string()));
}

#[test]
fn a_command_and_a_hook_say_now_beside_today() {
    let command = CommandInput::<RawGraph> {
        today: "2026-10-28".into(),
        now: "2026-10-28T09:30:00Z".into(),
        ..Default::default()
    };
    let sent = serde_json::to_value(&command).unwrap();
    assert_eq!(
        (&sent["today"], &sent["now"]),
        (&json!("2026-10-28"), &json!("2026-10-28T09:30:00Z"))
    );
    let hook = HookInput::<RawGraph> {
        now: "2026-10-28T09:30:00Z".into(),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(&hook).unwrap()["now"],
        "2026-10-28T09:30:00Z"
    );
}

#[test]
fn a_prompt_and_a_render_input_carry_no_now() {
    // A rendered prompt is reproducible; `RenderInput.today` is blank on purpose.
    assert!(!keys(PromptInput::<RawGraph>::default()).contains(&"now".to_string()));
    assert!(!keys(RenderInput::default()).contains(&"now".to_string()));
}

#[test]
fn a_pass_input_has_no_clock() {
    let keys = keys(PassInput::default());
    assert!(
        !keys.contains(&"now".to_string()) && !keys.contains(&"today".to_string()),
        "{keys:?}"
    );
}
