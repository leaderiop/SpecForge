//! `specforge hook` over the host fixture (ADR 0292): the facts a guest cannot read, the failure policy, the plan
//! a writing hook answers and a hook the settings turn off. [`run_hook`] is the one place a test calls the host.

use serde_json::json;
use specforge_project::testing::{CORE, GsdHosts};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::{WasmCallResult, WasmTrapInfo};

use crate::host_support::{HookRun, hosted, hosts_project, run_hook, write_event};

fn trap() -> WasmCallResult {
    WasmCallResult::Trap(WasmTrapInfo {
        kind: "timeout".into(),
        message: "the call exceeded 50 ms".into(),
        export_name: "hook__write_guard".into(),
    })
}

fn root_of(hosted: &crate::host_support::Hosted) -> String {
    hosted.project.root().display().to_string()
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "a hook is given the facts of every path its tool names that a guest cannot read"
)]
fn a_hook_sees_path_facts_it_cannot_read() {
    let hosted = hosted(hosts_project(json!({}), json!({})), GsdHosts::new());
    let root = root_of(&hosted);
    let existing = hosted.project.read("m.spec").len();
    let run = run_hook(
        &hosted,
        "write_guard",
        &write_event("claude-code", &format!("{root}/m.spec"), "x"),
    );
    assert_eq!(run.answer["decision"], "deny", "{run:?}");
    let facts = hosted.input("hook__write_guard")["paths"].clone();
    assert_eq!(
        facts,
        json!([{"path": "m.spec", "in_project": true, "in_spec_root": true, "exists": true,
                "bytes": existing, "new_bytes": 1}])
    );
    let outside = run_hook(
        &hosted,
        "write_guard",
        &write_event("claude-code", "/elsewhere/x.txt", "x"),
    );
    assert_eq!(outside.answer["decision"], "allow", "{outside:?}");
    assert_eq!(
        hosted.input("hook__write_guard")["paths"][0],
        json!({"path": "/elsewhere/x.txt", "in_project": false, "in_spec_root": false, "exists": false,
               "bytes": null, "new_bytes": 1})
    );
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "a hook that times out or traps applies its failure policy: allow is a no-op with the E028 line, deny is a deny with its text"
)]
fn a_hook_that_times_out_applies_its_failure_policy() {
    // `write_guard` declares `on_failure: deny`; `injection_scan` declares nothing, so it allows.
    let hosted = hosted(
        hosts_project(json!({}), json!({})),
        GsdHosts::new()
            .answering(CORE, "hook__write_guard", trap())
            .answering(CORE, "hook__injection_scan", trap()),
    );
    let root = root_of(&hosted);
    let denied = run_hook(
        &hosted,
        "write_guard",
        &write_event("claude-code", &format!("{root}/m.spec"), "x"),
    );
    assert_eq!(denied.answer["decision"], "deny", "{denied:?}");
    assert!(
        denied.answer["reason"].as_str().unwrap().contains("E028"),
        "{denied:?}"
    );

    let read = json!({"event": "post_tool", "host": "claude-code", "session": "s1",
        "tool": {"name": "Read", "class": "read", "input": {"file_path": format!("{root}/m.spec")}},
        "response": {"content": "hello"}});
    let allowed = run_hook(&hosted, "injection_scan", &read);
    assert!(
        allowed.answer["decision"].is_null() && allowed.answer["context"].is_null(),
        "{allowed:?}"
    );
    assert!(
        allowed.stderr.contains("E028"),
        "the failure is on stderr: {allowed:?}"
    );
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "pre_commit fires only for git commit, with its message"
)]
fn pre_commit_fires_only_for_git_commit_with_its_message() {
    let hosted = hosted(hosts_project(json!({}), json!({})), GsdHosts::new());
    let shell = |command: &str| {
        json!({"event": "pre_commit", "host": "claude-code", "session": "s1",
               "tool": {"name": "Bash", "class": "shell", "input": {"command": command}}})
    };
    let status = run_hook(&hosted, "validate_commit", &shell("git status"));
    assert!(
        status.exports.is_empty(),
        "git status is no commit: {status:?}"
    );
    assert!(status.answer["decision"].is_null());

    let bad = run_hook(
        &hosted,
        "validate_commit",
        &shell("git commit -m \"wip stuff\""),
    );
    assert_eq!(bad.answer["decision"], "deny", "{bad:?}");
    assert_eq!(
        hosted.input("hook__validate_commit")["commit"]["message"],
        "wip stuff"
    );

    let good = run_hook(
        &hosted,
        "validate_commit",
        &shell("git commit -m \"feat(03-01): x\""),
    );
    assert_eq!(good.answer["decision"], "allow", "{good:?}");
    assert_eq!(hosted.input("hook__validate_commit")["shell"][0], "git");

    // A cluster of short options still gives its message; a message read from elsewhere is unknown, not empty.
    run_hook(
        &hosted,
        "validate_commit",
        &shell("git commit -am \"wip stuff\""),
    );
    assert_eq!(
        hosted.input("hook__validate_commit")["commit"]["message"],
        "wip stuff"
    );
    run_hook(&hosted, "validate_commit", &shell("git commit -F msg.txt"));
    assert!(hosted.input("hook__validate_commit")["commit"]["message"].is_null());
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "a hook's plan is applied through the edit plan step, and a refused plan becomes a notice"
)]
fn a_hook_plan_is_applied_through_the_edit_plan_step() {
    let hosted_ok = |plan: serde_json::Value| {
        hosted(
            hosts_project(json!({}), json!({})),
            GsdHosts::new().answering(
                CORE,
                "hook__record_hook",
                WasmCallResult::Ok(json!({"plan": plan}).to_string().into_bytes()),
            ),
        )
    };
    let event = |hosted: &crate::host_support::Hosted| {
        let root = root_of(hosted);
        json!({"event": "post_tool", "host": "claude-code", "session": "s1",
               "tool": {"name": "Write", "class": "write", "input": {"file_path": format!("{root}/m.spec"), "content": "x"}}})
    };
    let uat = hosts_revision("u.spec");
    let written = hosted_ok(
        json!({"ops": [{"op": "set", "entity": "v1.0/03/U1", "field": "result",
                                            "value": "pass", "revision": uat}]}),
    );
    let run = run_hook(&written, "record_hook", &event(&written));
    assert!(written.project.read("u.spec").contains("pass"), "{run:?}");
    assert_eq!(run.changed, ["u.spec"]);

    // A stale revision: K5's apply step refuses the plan, nothing is written, and the answer says so.
    let refused = hosted_ok(
        json!({"ops": [{"op": "set", "entity": "v1.0/03/U1", "field": "phase",
                                            "value": "v1.0/03", "revision": "stale"}]}),
    );
    let run = run_hook(&refused, "record_hook", &event(&refused));
    assert!(run.changed.is_empty(), "{run:?}");
    assert!(
        run.answer["notice"]
            .as_str()
            .unwrap()
            .contains("t.record_hook"),
        "{run:?}"
    );
}

/// The revision a plan names for `path`, as the host computes it.
fn hosts_revision(path: &str) -> String {
    let project = hosts_project(json!({}), json!({}));
    specforge_project::testing::revision_of(project.read(path).as_bytes())
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "a hook whose when setting is off is a no-op that loads nothing further"
)]
fn a_hook_off_by_its_setting_is_a_no_op() {
    // `write_guard` declares `when: workflow.hooks`.
    let hosted = hosted(
        hosts_project(json!({"workflow.hooks": false}), json!({})),
        GsdHosts::new(),
    );
    let root = root_of(&hosted);
    let before = hosted.exports().len();
    let run: HookRun = run_hook(
        &hosted,
        "write_guard",
        &write_event("claude-code", &format!("{root}/m.spec"), "x"),
    );
    assert!(run.answer["decision"].is_null(), "{run:?}");
    assert!(run.exports.is_empty(), "{run:?}");
    assert_eq!(hosted.exports().len(), before, "the guest was not called");
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "the human guard denies a shell call that runs a human-attested command, from the served descriptors"
)]
fn the_human_guard_denies_a_shell_call_running_a_human_command() {
    let hosted = hosted(hosts_project(json!({}), json!({})), GsdHosts::new());
    let shell = |command: &str| {
        json!({"event": "pre_tool", "host": "cursor", "session": "s1",
               "tool": {"name": "Shell", "class": "shell", "input": {"command": command}}})
    };
    for line in [
        "specforge t approve --by me",
        "cd x && /usr/local/bin/specforge --path . t approve",
        "FOO=1 specforge t approve",
        "bash -c 'specforge t approve --by me'",
    ] {
        let run = run_hook(&hosted, "core.human_guard", &shell(line));
        assert_eq!(run.answer["decision"], "deny", "{line}: {run:?}");
        let reason = run.answer["reason"].as_str().unwrap();
        assert!(
            reason.contains("`specforge t approve`") && reason.contains("their own terminal"),
            "{line}: {reason}"
        );
        assert!(run.exports.is_empty(), "no guest is called");
    }
    for line in [
        "specforge t stats",
        "echo specforge t approve",
        "git status && ls",
        "specforge t approve-not-a-command",
    ] {
        let run = run_hook(&hosted, "core.human_guard", &shell(line));
        assert!(run.answer["decision"].is_null(), "{line}: {run:?}");
    }
}
