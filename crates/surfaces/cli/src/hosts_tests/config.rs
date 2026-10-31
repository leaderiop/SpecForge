//! ADR 0306: install, uninstall and `--check` refuse a `specforge.json` the compile cannot use and a host config of
//! the wrong shape, name the key, and write nothing; uninstall removes only what install created.

use std::collections::BTreeSet;

use serde_json::json;
use specforge_project::testing::TestProject;
use specforge_test_macros::test as specforge_test;

use super::{install, project, tree, uninstall};

const HOSTS: [&str; 3] = ["claude-code", "codex", "cursor"];

/// Every directory of the project outside `.git` and `.specforge`, by relative path.
fn dirs(project: &TestProject) -> BTreeSet<String> {
    fn walk(root: &std::path::Path, at: &std::path::Path, out: &mut BTreeSet<String>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if relative.starts_with(".specforge") || relative.starts_with(".git") {
                continue;
            }
            if path.is_dir() {
                out.insert(relative);
                walk(root, &path, out);
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(project.root(), project.root(), &mut out);
    out
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "install, uninstall and install --check refuse a specforge.json the compile reports as E069 and write nothing"
)]
fn install_refuses_an_unusable_specforge_json_and_writes_nothing() {
    let p = project(json!({}), json!({}));
    assert_eq!(install(&p, &HOSTS, &[]).code, 0);
    for bad in [
        "{bad",
        "[1,2]",
        r#"{"extensions": "@t/core"}"#,
        r#"{"settings": []}"#,
    ] {
        p.write("specforge.json", bad);
        let before = tree(&p);
        for (name, out) in [
            ("install", install(&p, &HOSTS, &[])),
            ("install --check", install(&p, &HOSTS, &["--check"])),
            ("install --dry-run", install(&p, &HOSTS, &["--dry-run"])),
            ("uninstall", uninstall(&p, &HOSTS, &[])),
        ] {
            assert_eq!(out.code, 1, "{bad}: {name}: {}", out.stdout);
            let body = out.json();
            assert_eq!(body["code"], "E069", "{bad}: {name}: {body}");
            assert_eq!(
                body["suggestion"],
                "fix specforge.json; `specforge explain E069` says what it must be",
                "{bad}: {name}"
            );
            if bad == "{bad" {
                let error = body["error"].as_str().unwrap();
                assert!(
                    error.starts_with("specforge.json can't be used: "),
                    "{error}"
                );
                assert!(error.ends_with("; no extension is loaded"), "{error}");
            }
            assert_eq!(tree(&p), before, "{bad}: {name} wrote");
        }
    }
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "install refuses a host config of the wrong shape, naming the key, and writes nothing on any host"
)]
fn install_refuses_a_wrong_shape_config_by_name_and_writes_nothing() {
    let rows = [
        (
            "claude-code",
            ".mcp.json",
            "{\"mcpServers\": []}",
            "'mcpServers' is an array, and install needs an object there",
        ),
        (
            "claude-code",
            ".claude/settings.json",
            "{\"hooks\": []}",
            "'hooks' is an array, and install needs an object there",
        ),
        (
            "claude-code",
            ".claude/settings.json",
            "{\"hooks\": {\"PreToolUse\": {}}}",
            "'hooks.PreToolUse' is an object, and install needs an array there",
        ),
        (
            "codex",
            ".codex/config.toml",
            "mcp_servers = 1\n",
            "'mcp_servers' holds a value where install needs a table",
        ),
    ];
    for (host, file, text, why) in rows {
        let p = project(json!({}), json!({}));
        p.write(file, text);
        let before = tree(&p);
        let out = install(&p, &HOSTS, &[]);
        assert_eq!(out.code, 1, "{file}: {}", out.stdout);
        let refused = out.json()["refused"].clone();
        let message = format!("{file} is not a document install can merge into: {why}");
        assert_eq!(
            refused[0],
            json!({"code": "unreadable_config", "host": host, "path": file, "message": message}),
            "{file}: {refused}"
        );
        assert!(!message.contains("byte"));
        assert_eq!(tree(&p), before, "{file}: install wrote");
    }
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "uninstall refuses a marked file edited since and names uninstall --force"
)]
fn uninstall_of_an_edited_file_names_uninstall_force() {
    let p = project(json!({}), json!({}));
    assert_eq!(install(&p, &["claude-code"], &[]).code, 0);
    let skill = ".claude/skills/t-ns-context/SKILL.md";
    p.write(skill, format!("{}\nedited\n", p.read(skill)));
    let out = uninstall(&p, &["claude-code"], &[]);
    assert_eq!(out.code, 1, "{}", out.stdout);
    let refused = out.json()["refused"][0].clone();
    assert_eq!(refused["code"], "edited_file", "{refused}");
    assert_eq!(
        refused["message"],
        format!(
            "{skill} was edited since install wrote it: `specforge uninstall claude-code --force` removes it anyway"
        )
    );
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "uninstall removes only what install created: a config file, a key and a directory the user had stay"
)]
#[ignore = "pins K20a; T12 un-ignores"]
fn uninstall_keeps_what_the_user_had_before_install() {
    let p = project(json!({}), json!({}));
    p.write(".claude/settings.json", "{}\n");
    p.write(".mcp.json", "{\"mcpServers\": {}}\n");
    p.write(".cursor/mcp.json", "{\"mcpServers\": {}}\n");
    for dir in [".claude/agents", ".claude/skills", ".codex"] {
        std::fs::create_dir_all(p.root().join(dir)).unwrap();
    }
    let (files, directories) = (tree(&p), dirs(&p));
    assert_eq!(install(&p, &HOSTS, &[]).code, 0);
    assert_eq!(uninstall(&p, &HOSTS, &[]).code, 0);
    assert_eq!(tree(&p), files);
    assert_eq!(dirs(&p), directories);
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "uninstall removes the empty host directories install created, the host's root included"
)]
#[ignore = "pins K20a; T12 un-ignores"]
fn uninstall_removes_the_host_folder_it_created() {
    let p = project(json!({}), json!({}));
    assert_eq!(install(&p, &HOSTS, &[]).code, 0);
    assert_eq!(uninstall(&p, &HOSTS, &[]).code, 0);
    for root in [".claude", ".codex", ".cursor"] {
        assert!(!p.root().join(root).exists(), "{root} was left behind");
    }
    let record = p.root().join(".specforge/install");
    let left = std::fs::read_dir(&record).map_or(0, |d| d.count());
    assert_eq!(left, 0, "a record is left under .specforge/install");
}
