#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]
//! Each host's recorded hook stdin, the neutral event it reads into, a neutral answer and what the host is written
//! (`tests/protocol/<host>/*.json`).

use std::path::Path;

use serde_json::Value;
use specforge_hosts::renderer;
use specforge_ops::host::{HookAnswer, Host};

fn check(host: Host) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/protocol")
        .join(host.as_str());
    let adapter = renderer(host).expect("an adapter");
    let mut cases = 0;
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.unwrap().path();
        let case: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let stdin = serde_json::to_vec(&case["stdin"]).unwrap();
        let event = adapter
            .read_event(&stdin)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            serde_json::to_value(&event).unwrap(),
            case["event"],
            "{name}: the event read"
        );
        let answer: HookAnswer = serde_json::from_value(case["answer"].clone()).unwrap();
        let reply = adapter.write_answer(&event, &answer);
        assert_eq!(
            reply.stdout,
            case["stdout"].as_str().unwrap(),
            "{name}: stdout"
        );
        assert_eq!(
            i64::from(reply.exit_code),
            case["exit_code"].as_i64().unwrap(),
            "{name}: exit code"
        );
        cases += 1;
    }
    assert!(cases >= 5, "{} holds {cases} cases", dir.display());
}

#[specforge_test_macros::test(
    behavior = "run_a_hook",
    verify = "each host reads its hook stdin into the neutral event and writes the neutral answer as the host expects it"
)]
fn claude_code_reads_and_answers_its_hook_protocol() {
    check(Host::ClaudeCode);
}

#[test]
fn a_host_input_that_is_no_event_is_refused() {
    let adapter = renderer(Host::ClaudeCode).unwrap();
    assert!(adapter.read_event(b"not json").is_err());
    assert!(adapter.read_event(b"[1]").is_err());
    assert!(
        adapter
            .read_event(br#"{"hook_event_name":"Elicitation"}"#)
            .is_err()
    );
}

#[test]
fn codex_reads_and_answers_its_hook_protocol() {
    check(Host::Codex);
}

#[test]
fn cursor_reads_and_answers_its_hook_protocol() {
    check(Host::Cursor);
}
