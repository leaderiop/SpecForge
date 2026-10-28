//! A prompt is on the wire (ADR 0291, protocol 1.10.0): the input the host sends a `prompt__*` export and the
//! messages it answers, and the `trigger` of a guard pass's input.

use std::cell::RefCell;

use serde_json::Value;
use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::{PassInput, PromptInput, PromptOutput, RawGraph};
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::{CallFailure, ExtensionCalls};

const EXT: &str = "@calls/prompt";

thread_local! {
    static RECEIVED: RefCell<Option<Value>> = const { RefCell::new(None) };
}

fn wire(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/wire")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

/// `calls.render_prompt` on an empty input, its answer dropped: the `Operation::Prompt` arm of `calls.rs`.
pub fn render(
    calls: &ExtensionCalls,
    extension: &str,
    export: &str,
) -> Result<(), specforge_wasm::CallError> {
    let input = PromptInput::<RawGraph>::default();
    calls.render_prompt(extension, export, &input).map(drop)
}

fn extension() -> ContributionsBuilder {
    ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"))
}

/// `prompt__x` decodes the host's input as the SDK's type and answers the golden messages.
fn guest(export: &str, input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    (export == "prompt__x").then(|| {
        let input: PromptInput = serde_json::from_slice(input).map_err(|e| e.to_string())?;
        RECEIVED.with(|r| *r.borrow_mut() = Some(serde_json::to_value(&input).unwrap()));
        Ok(wire("prompt.output.json").to_string().into_bytes())
    })
}

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "the host sends a prompt export its typed arguments and host and reads back its messages"
)]
fn a_prompt_call_sends_the_golden_input_and_reads_the_golden_output() {
    let runtime = InProcessRuntime::new().with_handler(extension, guest);
    let golden = wire("prompt.input.json");
    let input: PromptInput<RawGraph> = serde_json::from_value(
        // `RawGraph` only serializes: the golden has no graph, so read it as the plain shape.
        golden.clone(),
    )
    .map(|plain: PromptInput| PromptInput {
        args: plain.args,
        cwd: plain.cwd,
        today: plain.today,
        graph: None,
        host: plain.host,
        state: None,
    })
    .unwrap();
    let output = ExtensionCalls::new(&runtime)
        .render_prompt(EXT, "prompt__x", &input)
        .unwrap();
    assert_eq!(runtime.calls().last().unwrap().input, golden);
    assert_eq!(RECEIVED.with(|r| r.borrow().clone()).unwrap(), golden);
    let expected: PromptOutput = serde_json::from_value(wire("prompt.output.json")).unwrap();
    assert_eq!(output, expected);
    assert_eq!(output.messages.len(), 2);
}

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "a prompt export that answers no message is E028"
)]
fn a_prompt_that_answers_no_message_is_malformed() {
    let runtime = InProcessRuntime::new().with_handler(extension, |export, _| {
        (export == "prompt__x").then(|| Ok(br#"{"messages":[]}"#.to_vec()))
    });
    let error = ExtensionCalls::new(&runtime)
        .render_prompt(EXT, "prompt__x", &PromptInput::<RawGraph>::default())
        .unwrap_err();
    assert!(matches!(
        error.failure,
        CallFailure::Malformed {
            expected: "PromptOutput",
            ..
        }
    ));
    assert_eq!(error.diagnostic().code, "E028");
}

#[specforge_test_macros::test(
    behavior = "gate_a_command",
    verify = "a guard pass's input carries the trigger that made the host run it"
)]
fn a_guard_pass_input_carries_its_trigger() {
    let golden = wire("pass.guard.input.json");
    let input: PassInput = serde_json::from_value(golden.clone()).unwrap();
    let trigger = input.trigger.as_ref().unwrap();
    assert_eq!(trigger.command.as_deref(), Some("ship"));
    assert_eq!(trigger.subject.as_deref(), Some("v1.0/03"));
    assert_eq!(serde_json::to_value(&input).unwrap(), golden);
    assert!(
        wire("pass.input.json").get("trigger").is_none(),
        "a write sends none"
    );
}

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "a prompt answer that is not a PromptOutput is E028"
)]
fn a_prompt_answer_that_does_not_decode_is_e028() {
    use specforge_wasm::WasmCallResult;
    for raw in [
        &br"not json"[..],
        br#"{"messages":[{"role":"system","text":"t"}]}"#,
        br#"{"messages":[{"text":"t"}]}"#,
        br#"{"messages":"hi"}"#,
    ] {
        let runtime = InProcessRuntime::new()
            .with_handler(extension, guest)
            .answer_raw(EXT, "prompt__x", WasmCallResult::Ok(raw.to_vec()));
        let error = render(&ExtensionCalls::new(&runtime), EXT, "prompt__x").unwrap_err();
        let shown = String::from_utf8_lossy(raw);
        assert!(
            matches!(
                error.failure,
                CallFailure::Malformed {
                    expected: "PromptOutput",
                    ..
                }
            ),
            "{shown}: {error:?}"
        );
        assert!(
            error.diagnostic().message.starts_with(&format!(
                "MCP prompt prompt__x() of '{EXT}' answered output that is not a PromptOutput: "
            )),
            "{shown}"
        );
    }
}

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "a prompt message keeps its lines and tabs, and every other control character is escaped"
)]
fn a_prompt_message_keeps_its_lines() {
    use specforge_wasm::WasmCallResult;
    let raw = serde_json::json!({"messages": [{"role": "user", "text": "# Title\n\n\tstep\r\nnext \u{1b}[31mred\u{1b}[0m\r"}]});
    let runtime = InProcessRuntime::new().answer_raw(
        EXT,
        "prompt__x",
        WasmCallResult::Ok(raw.to_string().into_bytes()),
    );
    let output = ExtensionCalls::new(&runtime)
        .render_prompt(EXT, "prompt__x", &PromptInput::<RawGraph>::default())
        .unwrap();
    assert_eq!(
        output.messages[0].text,
        "# Title\n\n\tstep\r\nnext \\u{1b}[31mred\\u{1b}[0m\\r"
    );
}
