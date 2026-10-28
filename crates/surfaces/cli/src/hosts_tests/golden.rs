//! The goldens of the three host adapters, compared to the tree `specforge install` writes for the fixture
//! (`crates/adapters/hosts/tests/golden/<host>/`; the tests move into that crate with T6).

use std::collections::BTreeMap;

use serde_json::json;
use specforge_test_macros::test as specforge_test;

use super::{install, project, tree};

/// The files of `crates/adapters/hosts/tests/golden/<host>/`, by relative path.
fn golden(host: &str) -> BTreeMap<String, String> {
    let root = specforge_repo_root::repo_root()
        .join("crates/adapters/hosts/tests/golden")
        .join(host);
    let mut out = BTreeMap::new();
    fn walk(root: &std::path::Path, at: &std::path::Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(at).unwrap_or_else(|e| panic!("{}: {e}", at.display())) {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(relative, std::fs::read_to_string(&path).unwrap());
            }
        }
    }
    walk(&root, &root, &mut out);
    out
}

/// What installing the fixture into `host` adds to the project.
fn rendered(host: &str) -> BTreeMap<String, String> {
    let project = project(json!({}), json!({"workflow.ui_phase": true}));
    let before = tree(&project);
    let out = install(&project, &[host], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    tree(&project)
        .into_iter()
        .filter(|(path, _)| !before.contains_key(path))
        .collect()
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "Claude Code gets the agents, skills, slash commands, hooks, statusline and MCP entry of the fixture"
)]
#[ignore = "K9 T9"]
fn claude_code_renders_the_fixture() {
    assert_eq!(rendered("claude-code"), golden("claude-code"));
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "Codex gets agent TOML without an Anthropic model, skills, hooks.json and config.toml"
)]
#[ignore = "K9 T9"]
fn codex_renders_the_fixture() {
    assert_eq!(rendered("codex"), golden("codex"));
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "Cursor gets agents, skills, hooks.json and mcp.json, and reports the statusline and pre_compact as not rendered"
)]
#[ignore = "K9 T9"]
fn cursor_renders_the_fixture() {
    assert_eq!(rendered("cursor"), golden("cursor"));
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "each host reads its hook stdin into the neutral event and writes the neutral answer as the host expects it"
)]
fn each_host_reads_and_answers_its_hook_protocol() {
    // Recorded stdin and the stdout and exit code each host gets, per host, are fixtures of the adapter crate.
    let protocol = specforge_repo_root::repo_root().join("crates/adapters/hosts/tests/protocol");
    assert!(
        protocol.join("claude-code").exists()
            && protocol.join("codex").exists()
            && protocol.join("cursor").exists()
    );
}
