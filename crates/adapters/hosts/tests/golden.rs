#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]
//! The goldens: the tree each adapter writes for the fixture into an empty project (`tests/golden/<host>/`). Bless
//! with `SPECFORGE_BLESS=1` and read the diff.

use crate::support;

use std::collections::BTreeMap;
use std::path::Path;

use specforge_hosts::renderer;
use specforge_ops::host::Host;

fn written(host: Host) -> BTreeMap<String, String> {
    let adapter = renderer(host).expect("an adapter");
    let tree = adapter.render(&support::fixture(host));
    let mut files: BTreeMap<String, String> =
        tree.owned.into_iter().map(|f| (f.path, f.text)).collect();
    for merged in tree.merged {
        let out = adapter
            .merge(&merged.path, None, &merged.entries, &[])
            .expect("merges into nothing");
        files.extend(out.text.map(|t| (merged.path, t)));
    }
    files
}

fn on_disk(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<String, String>) {
        let Ok(entries) = std::fs::read_dir(at) else {
            return;
        };
        for entry in entries {
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
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn check(host: Host) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(host.as_str());
    let files = written(host);
    if std::env::var_os("SPECFORGE_BLESS").is_some() {
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in &files {
            let target = root.join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target, text).unwrap();
        }
    }
    assert_eq!(files, on_disk(&root));
}

#[specforge_test_macros::test(
    behavior = "install_into_a_host",
    verify = "Claude Code gets the agents, skills, slash commands, hooks, statusline and MCP entry of the fixture"
)]
fn claude_code_golden_is_the_fixture_rendered() {
    check(Host::ClaudeCode);
}

#[specforge_test_macros::test(
    behavior = "install_into_a_host",
    verify = "Codex gets agent TOML without an Anthropic model, skills, hooks.json and config.toml"
)]
fn codex_golden_is_the_fixture_rendered() {
    check(Host::Codex);
}

#[specforge_test_macros::test(
    behavior = "install_into_a_host",
    verify = "Cursor gets agents, skills, hooks.json and mcp.json, and reports the statusline and pre_compact as not rendered"
)]
fn cursor_golden_is_the_fixture_rendered() {
    check(Host::Cursor);
}

#[specforge_test_macros::test(
    behavior = "install_into_a_host",
    verify = "a command slash's CLI fallback names the host it was written for"
)]
fn a_command_fallback_names_the_host() {
    for host in Host::ALL {
        let files = written(host);
        let skill = files
            .iter()
            .find(|(path, _)| path.contains("t-stats"))
            .unwrap_or_else(|| panic!("{host:?} has the t-stats skill"));
        let want = format!(
            "run `specforge t stats --host {} <that text>`",
            host.as_str()
        );
        assert!(skill.1.contains(&want), "{host:?}: {}", skill.1);
    }
}
