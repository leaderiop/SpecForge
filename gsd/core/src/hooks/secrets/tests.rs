//! Ported from gsd-core v1.19.0 tests/gsd-secret-read-guard.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The suite's every call to the hook is replayed: `parity/upstream/v1.19.0/hook-cases/gsd-secret-read-guard.json`
//! holds the payloads upstream's tests sent and the answers upstream's hook gave (recorded by
//! `gsd/parity/record-hook-cases.cjs`). Each is run through `secret_read_guard`, which must deny exactly where
//! upstream blocked and name the same file. The calls too long to record (over 2000 characters: the 1 MiB limit,
//! the 20000-unit dense values) are written out below and in `v1_19`; the rest are skipped by describe block, each
//! with its reason.

use serde_json::Value;

use super::*;
use crate::hooks::{accounting, corpus};

mod depth;
mod v1_19;

const SUITE: &str = include_str!(gsd_kit::snapshot!("tests/gsd-secret-read-guard.test.cjs"));
const RECORDING: &str = include_str!(gsd_kit::snapshot!("hook-cases/gsd-secret-read-guard.json"));

const SKIPPED: &[(&str, &str)] = &[
    (
        "gsd-secret-read-guard: Kimi vocabulary",
        "runtime: Kimi's tool names (ReadFile, Shell) have no renderer here",
    ),
    (
        "gsd-secret-read-guard: gsd-debugger CLI spawn (#5214) > the Kimi Shell vocabulary is covered too",
        "runtime: Kimi's Shell tool",
    ),
    (
        "gsd-secret-read-guard: scope and crash policy > non-object payloads and a missing tool_name exit 0",
        "runtime: the kernel reads the event; a payload with no tool is no tool event",
    ),
    (
        "gsd-secret-read-guard: scope and crash policy > malformed JSON fails OPEN (declared HOOK_ON_CRASH.ALLOW)",
        "runtime: the kernel reads the event and applies on_failure, which is deny here (F7: SpecForge fails closed)",
    ),
];

/// The answer a recorded call expects of our hook: deny with upstream's typed `code` and `path`, or allow.
fn judge(call: &corpus::Call) -> Option<Result<(), String>> {
    let facts = corpus::claude_facts(call.input.as_ref()?)?;
    let answer = secret_read_guard(&facts);
    let blocked = call.status == 2;
    let denied = answer.decision == Some(Decision::Deny);
    if blocked != denied {
        return Some(Err(format!(
            "upstream {} and we {}: {}",
            if blocked { "blocked" } else { "allowed" },
            if denied { "deny" } else { "allow" },
            call.input
                .as_ref()
                .map(Value::to_string)
                .unwrap_or_default()
        )));
    }
    if blocked {
        let reason = answer.reason.unwrap_or_default();
        let (code, path) = (
            call.out["code"].as_str().unwrap_or_default(),
            call.out["path"].as_str().unwrap_or_default(),
        );
        let said = match code {
            "secret-read" => reason.contains(&format!("would read '{path}'")),
            "glob-too-complex" => reason.contains("more than 64 alternatives"),
            "command-too-large" => reason.contains("over 1048576 characters"),
            "debugger-cli-spawn" => reason.contains(&format!("'--agent {path}'")),
            other => panic!("a code we do not know: {other}"),
        };
        if !said {
            return Some(Err(format!("upstream {code} {path}; we: {reason}")));
        }
    }
    Some(Ok(()))
}

fn ported() -> Vec<String> {
    let mut ported = corpus::replay(SUITE, RECORDING, judge);
    // The two cases about the 1 MiB limit send a command too large to record.
    let big = |n: usize| Facts {
        class: "shell".into(),
        tool: "Bash".into(),
        input: serde_json::json!({"command": format!("echo {}", "a".repeat(n - 5))}),
        ..Facts::default()
    };
    let over = secret_read_guard(&big(MAX_COMMAND_LENGTH + 1));
    assert!(
        over.reason
            .unwrap_or_default()
            .contains("over 1048576 characters")
    );
    ported.push("gsd-secret-read-guard: Bash blocks > denies a command over 1 MiB as command-too-large without scanning it".into());
    for n in [MAX_COMMAND_LENGTH, MAX_COMMAND_LENGTH - 1] {
        assert_eq!(secret_read_guard(&big(n)).decision, None, "{n} bytes");
    }
    ported.push("gsd-secret-read-guard: Bash allows > allows benign commands at exactly 1 MiB and 1 MiB - 1".into());
    ported.extend(
        v1_19::PORTED
            .iter()
            .flat_map(|(_, cases)| cases.iter().map(ToString::to_string)),
    );
    ported
}

#[test]
fn secret_read_guard_matches_upstreams_cases() {
    // 1230 calls recorded: 10 over the input cap (`input: null`), 1 malformed JSON; 102 cases ported
    let calls = corpus::calls(RECORDING)
        .iter()
        .filter(|c| c.input.is_some())
        .count();
    let cases = ported();
    assert!(calls >= 1219, "{calls} recorded payloads");
    assert!(cases.len() >= 102, "{} cases ported", cases.len());
}

#[path = "../../../tests/common/pending.rs"]
mod pending;

#[test]
fn secret_read_guard_accounts_for_every_upstream_case() {
    let path = "tests/gsd-secret-read-guard.test.cjs";
    if pending::is_pending(path) {
        pending::assert_still_pending(path, || accounting::account(SUITE, &ported(), SKIPPED));
        return;
    }
    accounting::account(SUITE, &ported(), SKIPPED);
}
