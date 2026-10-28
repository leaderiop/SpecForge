//! `specforge install` and `uninstall`: idempotent, all or nothing, and owning exactly what they marked.

use serde_json::json;
use specforge_test_macros::test as specforge_test;

use super::{UI_EXTENSION, install, project, tree, uninstall};

const HOSTS: [&str; 3] = ["claude-code", "codex", "cursor"];

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "install writes the tree and a second install writes nothing"
)]
fn install_writes_the_tree_and_a_second_install_writes_nothing() {
    let project = project(json!({}), json!({}));
    let before = tree(&project);
    let first = install(&project, &HOSTS, &[]);
    assert_eq!(first.code, 0, "{}", first.stderr);
    let written = tree(&project);
    assert!(written.len() > before.len() + 10, "{:?}", written.keys());
    assert_eq!(
        first.action(".claude/agents/t-planner.md").as_deref(),
        Some("created")
    );

    let second = install(&project, &HOSTS, &[]);
    assert_eq!(second.code, 0, "{}", second.stderr);
    assert_eq!(tree(&project), written, "a second install writes nothing");
    let actions: Vec<String> = second.json()["hosts"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|h| h["files"].as_array().unwrap().clone())
        .map(|f| f["action"].as_str().unwrap().to_string())
        .collect();
    assert!(
        !actions.is_empty() && actions.iter().all(|a| a == "unchanged"),
        "{actions:?}"
    );
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "install refuses a file it did not write and writes nothing"
)]
fn install_refuses_a_foreign_file_and_writes_nothing() {
    let project = project(json!({}), json!({}));
    project.write(
        ".claude/agents/t-planner.md",
        "---\nname: t-planner\n---\nmine\n",
    );
    let before = tree(&project);
    let out = install(&project, &["claude-code", "codex"], &[]);
    assert_eq!(out.code, 1);
    let refused = out.json()["refused"].clone();
    assert_eq!(refused[0]["code"], "foreign_file", "{refused}");
    assert_eq!(refused[0]["path"], ".claude/agents/t-planner.md");
    assert_eq!(
        tree(&project),
        before,
        "nothing was written, on either host"
    );
    // --force never overwrites a file without a marker.
    let forced = install(&project, &["claude-code"], &["--force"]);
    assert_eq!(forced.code, 1);
    assert_eq!(tree(&project), before);
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "install refuses a marked file edited since unless forced"
)]
fn install_refuses_an_edited_file_unless_forced() {
    let project = project(json!({}), json!({}));
    assert_eq!(install(&project, &["claude-code"], &[]).code, 0);
    let path = ".claude/skills/t-ns-context/SKILL.md";
    let original = project.read(path);
    project.write(path, format!("{original}\nmy own line\n"));

    let refused = install(&project, &["claude-code"], &[]);
    assert_eq!(refused.code, 1);
    assert_eq!(refused.json()["refused"][0]["code"], "edited_file");
    assert!(
        project.read(path).contains("my own line"),
        "the edit is kept"
    );

    let forced = install(&project, &["claude-code"], &["--force"]);
    assert_eq!(forced.code, 0, "{}", forced.stderr);
    assert_eq!(project.read(path), original);
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "install removes what no extension contributes any more and keeps a foreign sibling"
)]
fn install_removes_what_is_no_longer_contributed() {
    let project = project(json!({}), json!({"workflow.ui_phase": true}));
    assert_eq!(install(&project, &["claude-code"], &[]).code, 0);
    assert!(
        project
            .root()
            .join(".claude/agents/t-ui-checker.md")
            .exists()
    );
    project.write(
        ".claude/agents/mine.md",
        "---\nname: mine\n---\nhand written\n",
    );

    project.config_edit(|c| c["settings"][UI_EXTENSION]["workflow.ui_phase"] = json!(false));
    let out = install(&project, &["claude-code"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.action(".claude/agents/t-ui-checker.md").as_deref(),
        Some("removed")
    );
    assert!(
        !project
            .root()
            .join(".claude/agents/t-ui-checker.md")
            .exists()
    );
    assert!(
        project.root().join(".claude/agents/mine.md").exists(),
        "a foreign sibling is kept"
    );
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "install --check exits 1 on drift and names what would change, per host"
)]
fn install_check_exits_1_on_drift() {
    let project = project(json!({}), json!({}));
    assert_eq!(install(&project, &HOSTS, &[]).code, 0);
    assert_eq!(
        install(&project, &HOSTS, &["--check"]).code,
        0,
        "nothing drifted"
    );

    // Claude Code writes the model alias into the agent; Codex ignores an Anthropic alias, Cursor has no model.
    project.config_edit(|c| c["settings"]["@t/core"]["models.planner"] = json!("heavy"));
    let claude = install(&project, &["claude-code"], &["--check"]);
    assert_eq!(claude.code, 1);
    assert_eq!(
        claude.action(".claude/agents/t-planner.md").as_deref(),
        Some("updated"),
        "{}",
        claude.stdout
    );
    assert_eq!(install(&project, &["codex"], &["--check"]).code, 0);
    assert_eq!(install(&project, &["cursor"], &["--check"]).code, 0);
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "uninstall removes exactly what install wrote"
)]
fn uninstall_removes_exactly_what_install_wrote() {
    let project = project(json!({}), json!({}));
    project.write(
        ".claude/agents/mine.md",
        "---\nname: mine\n---\nhand written\n",
    );
    project.write(
        ".mcp.json",
        "{\n  \"mcpServers\": {\n    \"other\": {\"command\": \"other\"}\n  }\n}\n",
    );
    project.write(".claude/settings.json", "{\"hooks\": {\"PreToolUse\": [{\"matcher\": \"Bash\", \"hooks\": [{\"type\": \"command\", \"command\": \"mine.sh\"}]}]}}\n");
    let before = tree(&project);

    assert_eq!(install(&project, &HOSTS, &[]).code, 0);
    assert_ne!(tree(&project), before);
    let out = uninstall(&project, &HOSTS, &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        tree(&project),
        before,
        "the tree is what it was, byte for byte"
    );
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "install merges its entries without touching a user's hooks or servers"
)]
fn install_merges_without_touching_foreign_entries() {
    let project = project(json!({}), json!({}));
    project.write(
        ".mcp.json",
        "{\"mcpServers\": {\"other\": {\"command\": \"other\"}}}\n",
    );
    project.write(
        ".claude/settings.json",
        "{\"permissions\": {\"allow\": [\"Bash(ls)\"]}, \"hooks\": {\"PreToolUse\": [{\"matcher\": \"Bash\", \"hooks\": [{\"type\": \"command\", \"command\": \"mine.sh\"}]}]}}\n",
    );
    assert_eq!(install(&project, &["claude-code"], &[]).code, 0);
    let mcp: serde_json::Value = serde_json::from_str(&project.read(".mcp.json")).unwrap();
    assert_eq!(mcp["mcpServers"]["other"]["command"], "other");
    assert_eq!(mcp["mcpServers"]["specforge"]["command"], "specforge");
    let settings: serde_json::Value =
        serde_json::from_str(&project.read(".claude/settings.json")).unwrap();
    assert_eq!(settings["permissions"]["allow"][0], "Bash(ls)");
    let pre = settings["hooks"]["PreToolUse"].to_string();
    assert!(
        pre.contains("mine.sh") && pre.contains("specforge hook t.write_guard"),
        "{pre}"
    );

    assert_eq!(uninstall(&project, &["claude-code"], &[]).code, 0);
    let settings: serde_json::Value =
        serde_json::from_str(&project.read(".claude/settings.json")).unwrap();
    assert!(
        !settings.to_string().contains("specforge hook"),
        "{settings}"
    );
    assert!(settings.to_string().contains("mine.sh"));
}

#[specforge_test(
    behavior = "roll_back_interrupted_write",
    verify = "the next install rolls back an install killed between two writes before it plans"
)]
fn an_install_killed_between_two_writes_is_rolled_back_by_the_next_writer() {
    let project = project(json!({}), json!({}));
    assert_eq!(install(&project, &["claude-code"], &[]).code, 0);
    let installed = tree(&project);
    let path = ".claude/agents/t-planner.md";
    let written = project.read(path);
    // The kill: the journal says two files were written; the first is on disk, the second never got there. A host file
    // is not restored from the journal (it holds code the project directory could have planted): the next install
    // plans over the disk and finishes the job.
    let second = ".claude/agents/t-plan-checker.md";
    let _ = std::fs::remove_file(project.root().join(second));
    let journal = json!({
        "format": 1, "doing": "cannot write the host files",
        "files": [
            {"path": path, "before": "", "after": specforge_project::journal::digest(written.as_bytes())},
            {"path": second, "before": "", "after": specforge_project::journal::digest(b"never written")},
        ],
    });
    project.write(".specforge/write-journal.json", journal.to_string());

    let out = install(&project, &["claude-code"], &[]);
    assert_eq!(out.code, 0, "{}{}", out.stderr, out.stdout);
    assert_eq!(tree(&project), installed);
    assert!(
        !project
            .root()
            .join(".specforge/write-journal.json")
            .exists()
    );
}
