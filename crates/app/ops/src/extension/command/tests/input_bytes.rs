//! Pins the command input: 1.10's keys and `now` (ADR 0294), and no `files` for a command that reads none (ADR 0293).

use super::*;

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "the CommandInput carries the format the caller asked for and the host's date"
)]
fn command_input_differs_from_1_10_only_by_now() {
    let runtime = std::sync::Arc::new(fake());
    let fixture = fixture(&runtime);
    run_on(
        &fixture.view(),
        &listing(),
        json!({"status": "done"}).as_object().unwrap(),
        CommandFormat::Json,
        None,
        at("2026-10-03"),
    )
    .unwrap();

    let mut input = last_input(&runtime);
    let object = input.as_object_mut().unwrap();
    let keys: Vec<&str> = object.keys().map(String::as_str).collect();
    assert_eq!(keys, ["args", "cwd", "format", "graph", "now", "today"]);
    assert!(object["graph"].is_object());
    object.remove("graph");
    assert_eq!(
        input,
        json!({
            "args": {"status": "done", "all": false},
            "cwd": canonical_root(&fixture),
            "format": "json",
            "now": "2026-10-03T12:34:56Z",
            "today": "2026-10-03",
        })
    );
}
