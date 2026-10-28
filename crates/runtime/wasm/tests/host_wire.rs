//! A host contribution is on the wire (ADR 0292, protocol 1.10.0): the input the host sends a `render__` or
//! `hook__*` export, what a hook answers, and the time a hook is held to.

use serde_json::Value;
use specforge_protocol_types::{HookInput, HookOutput, RawGraph, RenderInput};
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::{CallFailure, ExtensionCalls, Operation};

const EXT: &str = "@calls/host";

fn wire(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/wire")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

fn extension() -> specforge_extension_sdk::ContributionsBuilder {
    specforge_extension_sdk::ContributionsBuilder::new(specforge_extension_sdk::ExtensionMeta::new(
        EXT, "1.0.0",
    ))
}

fn runtime() -> InProcessRuntime {
    InProcessRuntime::new().with_handler(extension, |export, _| match export {
        "render__" => Some(Ok(br##"{"body":"# t-planner\n\nline two\n"}"##.to_vec())),
        "hook__x" => Some(Ok(wire("hook.output.json").to_string().into_bytes())),
        _ => None,
    })
}

#[test]
fn a_render_call_sends_the_golden_input_and_reads_the_body_with_its_lines() {
    let runtime = runtime();
    let golden = wire("render.input.json");
    let input: RenderInput = serde_json::from_value(golden.clone()).unwrap();
    let output = ExtensionCalls::new(&runtime).render(EXT, &input).unwrap();
    assert_eq!(runtime.calls().last().unwrap().input, golden);
    assert_eq!(output.body, "# t-planner\n\nline two\n");
}

#[test]
fn a_hook_call_sends_the_golden_input_and_reads_the_golden_output() {
    let runtime = runtime();
    let golden = wire("hook.input.json");
    let plain: HookInput = serde_json::from_value(golden.clone()).unwrap();
    let input = HookInput::<RawGraph> {
        event: plain.event,
        host: plain.host,
        cwd: plain.cwd,
        today: plain.today,
        now: plain.now,
        session: plain.session,
        tool: plain.tool,
        paths: plain.paths,
        shell: plain.shell,
        commit: plain.commit,
        prompt: plain.prompt,
        response: plain.response,
        status: plain.status,
        graph: None,
        revisions: plain.revisions,
        state: plain.state,
    };
    let output = ExtensionCalls::new(&runtime)
        .run_hook(EXT, "hook__x", &input, 5_000)
        .unwrap();
    assert_eq!(runtime.calls().last().unwrap().input, golden);
    let expected: HookOutput = serde_json::from_value(wire("hook.output.json")).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn a_hook_answer_that_is_not_a_hook_output_is_e028() {
    let runtime = InProcessRuntime::new().with_handler(extension, |export, _| {
        (export == "hook__x").then(|| Ok(br#"{"decision":"maybe"}"#.to_vec()))
    });
    let error = ExtensionCalls::new(&runtime)
        .run_hook(EXT, "hook__x", &HookInput::default(), 5_000)
        .unwrap_err();
    assert_eq!(error.operation, Operation::Hook);
    assert!(matches!(
        error.failure,
        CallFailure::Malformed {
            expected: "HookOutput",
            ..
        }
    ));
    assert_eq!(error.diagnostic().code, "E028");
}
