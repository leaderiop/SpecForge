//! The hook members of protocol 1.14.0 (ADR 0320-0323): what the builder declares and what a call reads.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::json;
use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta, prelude::*};

#[specforge_test_macros::test(
    behavior = "run_a_hook",
    verify = "the SDK's hook builder declares hosts, git facts and other extensions' settings, and a call reads them"
)]
fn the_hook_builder_declares_hosts_git_and_other_settings() {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/flow", "1.0.0"));
    c.hook("h", HookEvent::PreTool, |h| {
        h.hosts(&["codex", "cursor"])
            .reads_git_status()
            .reads_setting("@gsd/security", "security.injection_blocking")
            .handler(|_| Ok(HookOutput::default()));
    });
    c.hook("plain", HookEvent::PostTool, |h| {
        h.handler(|_| Ok(HookOutput::default()));
    });
    let hooks = c.declaration().surfaces.hooks;
    assert_eq!(hooks[0].hosts, ["codex", "cursor"]);
    assert!(hooks[0].reads_git && hooks[0].reads_git_status);
    assert_eq!(
        hooks[0].reads_settings,
        ["@gsd/security:security.injection_blocking"]
    );
    let plain = serde_json::to_value(&hooks[1]).unwrap();
    for key in ["hosts", "reads_git", "reads_git_status", "reads_settings"] {
        assert!(plain.get(key).is_none(), "{key}: {plain}");
    }
}

#[test]
fn hook_call_reads_another_extensions_setting() {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/flow", "1.0.0"));
    c.hook("h", HookEvent::PreTool, |h| {
        h.handler(|call| {
            let on = call
                .other_setting("@gsd/security", "security.injection_blocking")
                .cloned();
            let other = call.other_setting("@gsd/security", "other").cloned();
            Ok(HookOutput::allow().with_context(format!("{on:?} {other:?}")))
        });
    });
    let input = json!({
        "event": "pre_tool", "host": "codex", "cwd": "/p", "today": "2026-10-31",
        "other_settings": {"@gsd/security:security.injection_blocking": true}
    });
    let out = c
        .dispatch_export("hook__h", input.to_string().as_bytes())
        .unwrap()
        .unwrap();
    let out: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(out["context"], "Some(Bool(true)) None");
}
