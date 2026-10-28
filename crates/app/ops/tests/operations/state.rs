//! An extension's hooks and commands keep small JSON state the host writes (ADR 0292): per project and per session,
//! read when declared, patched by what they answer, capped at 64 KiB.

use serde_json::json;
use specforge_project::testing::{CORE, GsdHosts};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::WasmCallResult;

use crate::host_support::{hosted, hosts_project, run_hook, state_of};

fn event(class: &str, name: &str) -> serde_json::Value {
    json!({"event": "post_tool", "host": "claude-code", "session": "s1",
           "tool": {"name": name, "class": class, "input": {}}})
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "state a hook patches is what another hook reads, per session"
)]
fn state_round_trips_between_hooks() {
    let hosted = hosted(hosts_project(json!({}), json!({})), GsdHosts::new());
    // The statusline reads the graph and patches the session; the context monitor reads the session.
    let status =
        json!({"event": "statusline", "host": "claude-code", "session": "s1", "status": {}});
    run_hook(&hosted, "statusline", &status);
    assert_eq!(
        state_of(&hosted, CORE, "sessions/s1").unwrap()["entities"]
            .as_u64()
            .unwrap()
            > 0,
        true
    );
    let monitored = run_hook(&hosted, "context_monitor", &event("read", "Read"));
    assert!(
        monitored.answer["context"]
            .as_str()
            .unwrap()
            .starts_with("entities="),
        "{monitored:?}"
    );
    // Another session has none of it.
    let other = json!({"event": "post_tool", "host": "claude-code", "session": "s2",
                       "tool": {"name": "Read", "class": "read", "input": {}}});
    let blank = run_hook(&hosted, "context_monitor", &other);
    assert!(blank.answer["context"].is_null(), "{blank:?}");
    // A session hook patches the project's state.
    run_hook(
        &hosted,
        "session_cursor",
        &json!({"event": "session_start", "host": "claude-code", "session": "s1"}),
    );
    assert_eq!(state_of(&hosted, CORE, "project").unwrap()["cursor"], "03");
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "a state patch that would exceed 64 KiB is refused and reported, and the state is unchanged"
)]
fn a_state_patch_over_the_cap_is_refused() {
    let big = "x".repeat(64 * 1024 + 1);
    let hosted = hosted(
        hosts_project(json!({}), json!({})),
        GsdHosts::new().answering(
            CORE,
            "hook__session_cursor",
            WasmCallResult::Ok(
                json!({"state": {"project": {"blob": big}}})
                    .to_string()
                    .into_bytes(),
            ),
        ),
    );
    let run = run_hook(
        &hosted,
        "session_cursor",
        &json!({"event": "session_start", "host": "claude-code", "session": "s1"}),
    );
    assert!(
        run.answer["notice"].as_str().unwrap().contains("64 KiB"),
        "{run:?}"
    );
    assert!(
        state_of(&hosted, CORE, "project").is_none(),
        "nothing was written"
    );
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "a command reads its state when it declares it and patches it with what it answers"
)]
fn a_command_patches_its_state() {
    let hosted = hosted(
        hosts_project(json!({}), json!({})),
        GsdHosts::new().answering(
            CORE,
            "cmd__stats",
            WasmCallResult::Ok(
                json!({"exit_code": 0, "stdout": "", "stderr": "", "state": {"project": {"runs": 1}}})
                    .to_string()
                    .into_bytes(),
            ),
        ),
    );
    let view = specforge_ops::view::ProjectView::of(&hosted.compiled);
    let commands = specforge_ops::extension::command::ExtensionCommands::build(view.registries());
    let stats = commands.of("t").find(|c| c.id() == "stats").unwrap();
    let args = serde_json::Map::new();
    specforge_ops::extension::command::run(
        &view,
        stats,
        &args,
        specforge_ops::extension::command::CommandFormat::Json,
    )
    .unwrap();
    assert_eq!(
        hosted.input("cmd__stats")["state"]["project"],
        json!({}),
        "it reads the state it declares"
    );
    assert_eq!(state_of(&hosted, CORE, "project").unwrap()["runs"], 1);
}
