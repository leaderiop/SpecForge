//! Pins the command input as 1.10 sent it: no `files`, no `now`. Both are later wire fields (K11, K14); a command
//! that asks for nothing is sent what it was sent. The ticket that adds `now` re-blesses this test with that one key.

use super::*;

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "the CommandInput carries the format the caller asked for and the host's date"
)]
fn command_input_is_byte_equal_without_k14() {
    let runtime = std::sync::Arc::new(fake());
    let fixture = fixture(&runtime);
    run_on(
        &fixture.view(),
        &listing(),
        json!({"status": "done"}).as_object().unwrap(),
        CommandFormat::Json,
        "2026-10-03",
    )
    .unwrap();

    let mut input = last_input(&runtime);
    let object = input.as_object_mut().unwrap();
    let keys: Vec<&str> = object.keys().map(String::as_str).collect();
    assert_eq!(keys, ["args", "cwd", "format", "graph", "today"]);
    assert!(object["graph"].is_object());
    object.remove("graph");
    assert_eq!(
        input,
        json!({
            "args": {"status": "done", "all": false},
            "cwd": canonical_root(&fixture),
            "format": "json",
            "today": "2026-10-03",
        })
    );
}
