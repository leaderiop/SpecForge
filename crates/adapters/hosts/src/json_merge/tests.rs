use serde_json::json;
use specforge_ops::host::MergedEntry;

use super::merge;

fn mcp() -> MergedEntry {
    MergedEntry {
        keys: vec!["mcpServers".into()],
        name: Some("specforge".into()),
        value: json!({"command": "specforge", "args": ["mcp", "."]}),
    }
}

fn hook(event: &str, route: &str) -> MergedEntry {
    MergedEntry {
        keys: vec!["hooks".into(), event.into()],
        name: None,
        value: json!({"matcher": "Write", "hooks": [{"type": "command", "command": format!("specforge hook {route}"), "timeout": 10}]}),
    }
}

fn text(existing: Option<&str>, entries: &[MergedEntry]) -> Option<String> {
    merge(existing, entries, &[]).unwrap().text
}

#[test]
fn json_merge_writes_a_new_file_as_pretty_json() {
    let out = text(None, &[mcp()]).unwrap();
    assert!(out.ends_with("}\n"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&out).unwrap(),
        json!({"mcpServers": {"specforge": {"command": "specforge", "args": ["mcp", "."]}}})
    );
    assert!(out.contains("\n  \"mcpServers\": {\n    \"specforge\""));
}

#[test]
fn json_merge_leaves_a_users_file_byte_for_byte_after_a_round_trip() {
    for original in [
        "{\n    \"theme\": \"dark\",\n    \"hooks\": {\n        \"Stop\": [\n            {\"hooks\": [{\"type\": \"command\", \"command\": \"mine\"}]}\n        ]\n    }\n}\n",
        "{\"theme\":\"dark\"}",
        "{\n  \"a\": 1\n}\n",
        "{\n  \"mcpServers\": {\n    \"mine\": {\"command\": \"x\"}\n  }\n}\n",
    ] {
        let entries = [mcp(), hook("PreToolUse", "t.a"), hook("Stop", "t.b")];
        let with = text(Some(original), &entries).unwrap();
        assert_ne!(with, original);
        let without = text(Some(&with), &[]).unwrap();
        assert_eq!(without, original, "after {with}");
        // Merging again changes nothing.
        assert_eq!(text(Some(&with), &entries).unwrap(), with);
    }
}

#[test]
fn json_merge_removes_a_file_that_only_held_our_entries() {
    let with = text(None, &[mcp(), hook("PreToolUse", "t.a")]).unwrap();
    assert_eq!(text(Some(&with), &[]), None);
}

#[test]
fn json_merge_replaces_our_stale_entry_and_keeps_a_users_sibling() {
    let one = text(
        Some("{\"hooks\": {\"PreToolUse\": [{\"hooks\": [{\"command\": \"mine\"}]}]}}"),
        &[hook("PreToolUse", "t.old")],
    )
    .unwrap();
    let two = text(Some(&one), &[hook("PreToolUse", "t.new")]).unwrap();
    assert!(two.contains("t.new") && !two.contains("t.old") && two.contains("mine"));
}

#[test]
fn json_merge_does_not_take_a_name_the_user_set() {
    let merged = merge(
        Some("{\"mcpServers\": {\"specforge\": {\"command\": \"mine\"}}}"),
        &[mcp()],
        &[],
    )
    .unwrap();
    assert_eq!(merged.not_rendered.len(), 1);
    assert!(merged.text.unwrap().contains("\"mine\""));
}

#[test]
fn json_merge_refuses_what_is_no_json_object() {
    assert!(merge(Some("[1]"), &[mcp()], &[]).is_err());
    assert!(merge(Some("{\"a\": "), &[mcp()], &[]).is_err());
}
