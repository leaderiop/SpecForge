use serde_json::json;
use specforge_ops::host::{HOOK_MARK, MergedEntry};

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
        value: json!({"matcher": "Write", "hooks": [{"type": "command", "command": format!("specforge hook {route} {HOOK_MARK}"), "timeout": 10}]}),
    }
}

const NAMED: &[super::Named<'static>] = &[(&["mcpServers"], "specforge"), (&[], "statusLine")];

fn text(existing: Option<&str>, entries: &[MergedEntry]) -> Option<String> {
    merge(existing, entries, &[], NAMED).unwrap().text
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
        NAMED,
    )
    .unwrap();
    assert_eq!(merged.not_rendered.len(), 1);
    assert!(merged.text.unwrap().contains("\"mine\""));
}

#[test]
fn json_merge_refuses_what_is_no_json_object() {
    assert!(merge(Some("[1]"), &[mcp()], &[], NAMED).is_err());
    assert!(merge(Some("{\"a\": "), &[mcp()], &[], NAMED).is_err());
}

#[test]
fn json_merge_keeps_a_scaffold_key_only_while_something_else_stands() {
    let version = MergedEntry {
        keys: vec![],
        name: Some("version".into()),
        value: json!(1),
    };
    let entries = [version, hook("preToolUse", "t.a")];
    let with = merge(None, &entries, &[("version", json!(1))], &[])
        .unwrap()
        .text
        .unwrap();
    assert!(with.contains("\"version\": 1"));
    assert_eq!(
        merge(Some(&with), &[], &[("version", json!(1))], &[])
            .unwrap()
            .text,
        None
    );
    let mine = "{\n  \"version\": 2\n}\n";
    let kept = merge(Some(mine), &entries, &[("version", json!(1))], &[])
        .unwrap()
        .text
        .unwrap();
    assert!(kept.contains("\"version\": 2"));
    assert_eq!(
        merge(Some(&kept), &[], &[("version", json!(1))], &[])
            .unwrap()
            .text
            .unwrap(),
        mine
    );
}

#[test]
fn a_users_hook_beside_ours_in_one_group_survives_update_and_uninstall() {
    let user = format!(
        "{{\"hooks\": {{\"PreToolUse\": [{{\"matcher\": \"Write\", \"hooks\": [{{\"type\": \"command\", \"command\": \"specforge hook t.a {HOOK_MARK}\"}}, {{\"type\": \"command\", \"command\": \"./my-user-hook.sh\"}}]}}]}}}}"
    );
    let updated = text(Some(&user), &[hook("PreToolUse", "t.b")]).unwrap();
    assert!(
        updated.contains("my-user-hook.sh") && updated.contains("t.b") && !updated.contains("t.a")
    );
    let gone = text(Some(&updated), &[]).unwrap();
    assert!(
        gone.contains("my-user-hook.sh") && !gone.contains("specforge hook"),
        "{gone}"
    );
}

#[test]
fn a_users_own_specforge_command_is_not_ours_unless_marked_or_named() {
    let mine = "{\n  \"mcpServers\": {\n    \"mine\": {\"command\": \"specforge\", \"args\": [\"mcp\", \"/other\"]}\n  },\n  \"hooks\": {\"Stop\": [{\"hooks\": [{\"command\": \"specforge hook mine.x --host claude-code\"}]}]}\n}\n";
    let with = text(Some(mine), &[mcp(), hook("PreToolUse", "t.a")]).unwrap();
    assert!(with.contains("/other") && with.contains("mine.x"));
    let without = text(Some(&with), &[]).unwrap();
    assert_eq!(without, mine);
}

#[test]
fn a_scalar_where_install_needs_an_object_is_refused_not_shadowed() {
    let err = merge(
        Some("{\"hooks\": null, \"a\": 1}"),
        &[hook("Stop", "t.a")],
        &[],
        NAMED,
    );
    assert!(err.is_err());
}
