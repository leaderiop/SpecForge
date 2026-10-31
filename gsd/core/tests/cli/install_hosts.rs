//! What `specforge install` writes on each of the three hosts (D-r44-18): the tree, the config files and the report are
//! pinned by goldens, and install's ownership is proved (idempotence, uninstall, an edited file, a foreign file, a
//! setting that makes the install stale, and a `HOME` that holds nothing but the wasmtime cache).
//!
//! The claude-code agents and skills have their own goldens (`install_agents.rs`, `install_skills.rs`). A marker's
//! `@gsd/core@<version>` is read as `@gsd/core@VERSION`; its `sha256:` digest is kept, as it is the digest of the file
//! without the marker line and does not move with the version.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::harness::{Project, Run, project};
use super::hooks_support::{HOSTS, Home, all_commands};

/// The directories and files a host's install owns at the project root.
fn owned(host: &str) -> &'static [&'static str] {
    match host {
        "claude-code" => &[".claude", ".mcp.json"],
        "codex" => &[".codex"],
        "cursor" => &[".cursor"],
        other => panic!("no host {other}"),
    }
}

/// Each config file of a host and the name of its golden.
fn configs(host: &str) -> [(&'static str, &'static str); 2] {
    match host {
        "claude-code" => [
            (".claude/settings.json", "settings.json"),
            (".mcp.json", "mcp.json"),
        ],
        "codex" => [
            (".codex/hooks.json", "hooks.json"),
            (".codex/config.toml", "config.toml"),
        ],
        "cursor" => [
            (".cursor/hooks.json", "hooks.json"),
            (".cursor/mcp.json", "mcp.json"),
        ],
        other => panic!("no host {other}"),
    }
}

/// A host's hooks file and a hook entry of the user's own, in that file's shape.
fn user_hook(host: &str) -> (&'static str, Value) {
    match host {
        "claude-code" => (
            ".claude/settings.json",
            json!({"matcher": "Bash", "hooks": [{"type": "command", "command": "echo mine"}]}),
        ),
        "codex" => (
            ".codex/hooks.json",
            json!({"matcher": "Bash", "hooks": [{"type": "command", "command": "echo mine"}]}),
        ),
        "cursor" => (".cursor/hooks.json", json!({"command": "echo mine"})),
        other => panic!("no host {other}"),
    }
}

/// A hand-written agent of the user's, which install did not write.
fn user_agent(host: &str) -> &'static str {
    match host {
        "codex" => ".codex/agents/mine.toml",
        "cursor" => ".cursor/agents/mine.md",
        _ => ".claude/agents/mine.md",
    }
}

/// An agent file install writes on each host.
fn an_agent(host: &str) -> &'static str {
    match host {
        "codex" => ".codex/agents/gsd-planner.toml",
        "cursor" => ".cursor/agents/gsd-planner.md",
        _ => ".claude/agents/gsd-planner.md",
    }
}

/// A skill file install writes on each host.
fn a_skill(host: &str) -> &'static str {
    match host {
        "codex" => ".codex/skills/gsd-fast/SKILL.md",
        "cursor" => ".cursor/skills/gsd-fast/SKILL.md",
        _ => ".claude/skills/gsd-fast/SKILL.md",
    }
}

/// `text` with every marker's `@gsd/core@<version>` read as `@gsd/core@VERSION`.
pub(super) fn normalised(text: &str) -> String {
    let key = "@gsd/core@";
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(key) {
        let (head, tail) = rest.split_at(at + key.len());
        out.push_str(head);
        let end = tail
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-')))
            .unwrap_or(tail.len());
        out.push_str("VERSION");
        rest = &tail[end..];
    }
    out + rest
}

fn golden_path(host: &str, name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/install")
        .join(host)
        .join(name)
}

/// `actual` against the golden `host/name`; `GSD_BLESS=1` writes it.
fn check_golden(host: &str, name: &str, actual: &str) {
    let path = golden_path(host, name);
    if std::env::var_os("GSD_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("the folder");
        std::fs::write(&path, actual).expect("write the golden");
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "{} is missing: GSD_BLESS=1 cargo test writes it",
            path.display()
        )
    });
    assert!(
        expected == actual,
        "{host}/{name} differs from its golden: review the change, then GSD_BLESS=1 cargo test\n--- expected\n{expected}\n--- actual\n{actual}"
    );
}

/// The paths of the project that a host's install may touch, with their bytes: everything but git's, the extension's
/// state, the specs and the manifest.
fn tree(p: &Project) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .expect("under root")
                .display()
                .to_string();
            if matches!(
                rel.as_str(),
                ".git" | ".specforge" | "spec" | "specforge.json"
            ) {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(rel, std::fs::read(&path).expect("read"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(p.root(), p.root(), &mut out);
    out
}

/// Every directory of the project, outside git, state and the specs, that holds nothing.
fn empty_dirs(p: &Project) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let entries: Vec<_> = std::fs::read_dir(dir)
            .expect("read dir")
            .flatten()
            .collect();
        if entries.is_empty() && dir != root {
            out.push(
                dir.strip_prefix(root)
                    .expect("under root")
                    .display()
                    .to_string(),
            );
        }
        for entry in entries {
            let rel = entry
                .path()
                .strip_prefix(root)
                .expect("under root")
                .display()
                .to_string();
            if entry.path().is_dir() && !matches!(rel.as_str(), ".git" | ".specforge" | "spec") {
                walk(root, &entry.path(), out);
            }
        }
    }
    let mut out = Vec::new();
    walk(p.root(), p.root(), &mut out);
    out.sort();
    out
}

fn tree_golden(p: &Project) -> String {
    let files: BTreeMap<String, Value> = tree(p)
        .into_iter()
        .map(|(path, bytes)| {
            let text = normalised(&String::from_utf8_lossy(&bytes));
            let digest = Sha256::digest(text.as_bytes());
            (
                path,
                json!({"bytes": text.len(), "sha256": format!("{digest:x}")}),
            )
        })
        .collect();
    serde_json::to_string_pretty(&files).expect("JSON") + "\n"
}

fn read(p: &Project, relative: &str) -> String {
    p.read(relative)
}

fn install(p: &Project, home: &Home, args: &[&str]) -> Run {
    let h = home.path().display().to_string();
    p.run_with(args, &[("HOME", &h)])
}

/// `specforge install <host> --format json` in a fresh project, exit 0 asserted.
fn first_install(host: &str) -> (Project, Home, Run) {
    let p = project("empty");
    let home = Home::new();
    let run = install(&p, &home, &["install", host, "--format", "json"]);
    assert_eq!(run.code, 0, "install {host}: {}", run.all());
    (p, home, run)
}

fn report(run: &Run) -> Value {
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("{e}: {}", run.all()))
}

/// What the host cannot take of the hooks, by hook id.
fn not_rendered(run: &Run) -> Vec<String> {
    report(run)["hosts"][0]["not_rendered"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|n| n["what"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn gsd_hooks(p: &Project, host: &str) -> usize {
    all_commands(p, host)
        .iter()
        .filter(|c| c.contains("specforge hook gsd."))
        .count()
}

/// The tree, the config files and the report of a first install on `host`; the hook lines and what the host could
/// not take are asserted beside the goldens.
fn golden_install(host: &str, hooks: usize, missing: &[&str]) {
    let (p, _home, run) = first_install(host);
    let files = tree(&p);
    assert_eq!(
        files.len(),
        67,
        "{host}: 15 agents, 50 skills and 2 config files"
    );
    check_golden(host, "tree.json", &tree_golden(&p));
    for (file, name) in configs(host) {
        check_golden(host, name, &normalised(&read(&p, file)));
    }
    check_golden(host, "report.json", &normalised(&run.stdout));
    let r = report(&run);
    assert!(
        r["hosts"][0]["files"]
            .as_array()
            .is_some_and(|f| f.len() == 67 && f.iter().all(|x| x["action"] == "created")),
        "{host}: every action is created"
    );
    assert_eq!(r["refused"], json!([]));
    assert_eq!(r["dry_run"], false);
    assert_eq!(gsd_hooks(&p, host), hooks, "{host}");
    let mut not = not_rendered(&run);
    not.sort();
    let mut want: Vec<String> = missing.iter().map(|m| format!("hook gsd.{m}")).collect();
    want.sort();
    assert_eq!(not, want, "{host}");
}

/// P6: the install writes 67 paths and the gsd hook lines on each host.
#[test]
fn install_tree_golden_claude_code() {
    golden_install("claude-code", 10, &[]);
    let (p, _home, _) = first_install("claude-code");
    let settings: Value = serde_json::from_str(&read(&p, ".claude/settings.json")).expect("JSON");
    assert!(settings["statusLine"].is_object(), "{settings}");
    assert!(
        all_commands(&p, "claude-code")
            .iter()
            .any(|c| c.contains("specforge hook core.spec_guard ")),
        "core.spec_guard"
    );
}

#[test]
fn install_tree_golden_codex() {
    golden_install(
        "codex",
        8,
        &["config_reload", "injection_scanner", "statusline"],
    );
}

#[test]
fn install_tree_golden_cursor() {
    // The prompt marker is rendered since ADR 0322 (Cursor's beforeSubmitPrompt): 8 lines, 2 not taken.
    golden_install(
        "cursor",
        8,
        &["config_reload", "context_compacted", "statusline"],
    );
}

#[test]
fn a_second_install_writes_nothing_on_three_hosts() {
    for host in HOSTS {
        let (p, home, _) = first_install(host);
        let before = tree(&p);
        let again = install(&p, &home, &["install", host, "--format", "json"]);
        assert_eq!(again.code, 0, "{host}: {}", again.all());
        let files = report(&again)["hosts"][0]["files"].clone();
        let files = files.as_array().expect("files");
        assert_eq!(files.len(), 67, "{host}");
        assert!(
            files.iter().all(|f| f["action"] == "unchanged"),
            "{host}: {files:?}"
        );
        assert_eq!(tree(&p), before, "{host}");
        let check = install(&p, &home, &["install", host, "--check"]);
        assert_eq!(check.code, 0, "{host}: {}", check.all());
    }
}

#[test]
fn uninstall_leaves_only_what_the_user_wrote() {
    for host in HOSTS {
        let (p, home, _) = first_install(host);
        let marked: Vec<String> = tree(&p).into_keys().collect();
        // the user's own hook entry, key and agent
        let (file, entry) = user_hook(host);
        let mut config: Value = serde_json::from_str(&read(&p, file)).expect("JSON");
        let first_event = config["hooks"]
            .as_object()
            .and_then(|h| h.keys().next().cloned())
            .expect("an event");
        config["hooks"][&first_event]
            .as_array_mut()
            .expect("entries")
            .push(entry);
        config["user_key"] = json!(1);
        p.write(file, &serde_json::to_string_pretty(&config).expect("JSON"));
        p.write(user_agent(host), "mine\n");
        let run = install(&p, &home, &["uninstall", host, "--format", "json"]);
        assert_eq!(run.code, 0, "{host}: {}", run.all());
        let left = tree(&p);
        let kept: Vec<&String> = left.keys().collect();
        assert_eq!(
            kept,
            [&file.to_string(), &user_agent(host).to_string()]
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>(),
            "{host}"
        );
        let after = read(&p, file);
        assert!(
            after.contains("echo mine") && after.contains("user_key"),
            "{host}: {after}"
        );
        assert!(
            !after.contains("specforge-install") && !after.contains("specforge hook"),
            "{host}: {after}"
        );
        assert!(marked.len() > left.len(), "{host}");
        assert_eq!(empty_dirs(&p), Vec::<String>::new(), "{host}");
        // with nothing of the user's, uninstall removes every path it wrote
        let (q, qhome, _) = first_install(host);
        let run = install(&q, &qhome, &["uninstall", host, "--format", "json"]);
        assert_eq!(run.code, 0, "{host}: {}", run.all());
        assert_eq!(tree(&q).len(), 0, "{host}: {:?}", tree(&q).keys());
        assert_eq!(empty_dirs(&q), Vec::<String>::new(), "{host}");
    }
}

#[test]
fn an_edited_agent_is_refused_and_forced() {
    for host in HOSTS {
        let (p, home, _) = first_install(host);
        let file = an_agent(host);
        let edited = format!("{}\n# the user's line\n", read(&p, file));
        p.write(file, &edited);
        let run = install(&p, &home, &["install", host, "--format", "json"]);
        assert_eq!(run.code, 1, "{host}: {}", run.all());
        let r = report(&run);
        let refused = r["refused"].as_array().expect("refused");
        assert_eq!(refused.len(), 1, "{host}: {refused:?}");
        assert_eq!(refused[0]["code"], "edited_file", "{host}");
        assert_eq!(refused[0]["path"], file, "{host}");
        assert_eq!(read(&p, file), edited, "{host}: the edit is kept");
        let forced = install(&p, &home, &["install", host, "--force", "--format", "json"]);
        assert_eq!(forced.code, 0, "{host}: {}", forced.all());
        assert!(!read(&p, file).contains("the user's line"), "{host}");
        let updated: Vec<Value> = report(&forced)["hosts"][0]["files"]
            .as_array()
            .expect("files")
            .iter()
            .filter(|f| f["action"] != "unchanged")
            .cloned()
            .collect();
        assert_eq!(
            updated,
            [json!({"path": file, "action": "updated"})],
            "{host}"
        );
    }
}

#[test]
fn an_unmarked_skill_is_foreign() {
    for host in HOSTS {
        let p = project("empty");
        let home = Home::new();
        let file = a_skill(host);
        p.write(file, "a skill the user wrote\n");
        let run = install(&p, &home, &["install", host, "--format", "json"]);
        assert_eq!(run.code, 1, "{host}: {}", run.all());
        let r = report(&run);
        let refused = r["refused"].as_array().expect("refused");
        assert_eq!(refused.len(), 1, "{host}: {refused:?}");
        assert_eq!(refused[0]["code"], "foreign_file", "{host}");
        assert_eq!(refused[0]["path"], file, "{host}");
        assert_eq!(read(&p, file), "a skill the user wrote\n", "{host}");
    }
}

/// REVIEW-r44 post-fix N5: the Cursor rendering of the session start is a `sessionStart` entry of the installed
/// `hooks.json`, present once `hooks.community` is on and absent by default (the roster's `gsd-cursor-session-start.js`).
#[test]
fn cursor_renders_the_session_start_of_the_community_hook() {
    let (p, home, _) = first_install("cursor");
    assert!(!read(&p, ".cursor/hooks.json").contains("sessionStart"));
    p.settings(json!({"hooks.community": true}));
    let run = install(&p, &home, &["install", "cursor", "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let hooks: Value = serde_json::from_str(&read(&p, ".cursor/hooks.json")).expect("json");
    let entries = hooks["hooks"]["sessionStart"]
        .as_array()
        .expect("a sessionStart list");
    assert!(
        entries
            .iter()
            .any(|e| e.to_string().contains("session_state")),
        "{entries:?}"
    );
}

#[test]
fn a_setting_flip_makes_the_install_stale() {
    for host in HOSTS {
        let (p, home, _) = first_install(host);
        let before = all_commands(&p, host);
        p.settings(json!({"hooks.community": true}));
        let check = install(&p, &home, &["install", host, "--check"]);
        assert_eq!(check.code, 1, "{host}: {}", check.all());
        let doctor = install(&p, &home, &["doctor", "--format", "json"]);
        let d: Value = serde_json::from_str(&doctor.stdout)
            .unwrap_or_else(|e| panic!("{e}: {}", doctor.all()));
        let stale: Vec<&Value> = d["findings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|f| {
                f["code"] == "host_install_stale"
                    && f["check"].as_str().is_some_and(|c| c.contains(host))
            })
            .collect();
        assert_eq!(stale.len(), 1, "{host}: {}", doctor.stdout);
        let run = install(&p, &home, &["install", host, "--format", "json"]);
        assert_eq!(run.code, 0, "{host}: {}", run.all());
        let after = all_commands(&p, host);
        let mut added: Vec<&String> = after.iter().filter(|c| !before.contains(c)).collect();
        added.sort();
        assert_eq!(added.len(), 3, "{host}: {added:?}");
        for (i, hook) in ["phase_boundary", "session_state", "validate_commit"]
            .iter()
            .enumerate()
        {
            assert!(
                added[i].starts_with(&format!("specforge hook gsd.{hook} ")),
                "{host}: {added:?}"
            );
        }
        assert_eq!(after.len(), before.len() + 3, "{host}");
        let check = install(&p, &home, &["install", host, "--check"]);
        assert_eq!(check.code, 0, "{host}: {}", check.all());
    }
}

/// Every path that exists under `dir`, relative to it.
fn names(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
            out.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .expect("under")
                    .display()
                    .to_string(),
            );
            if entry.path().is_dir() {
                walk(root, &entry.path(), out);
            }
        }
    }
    walk(dir, dir, &mut out);
    out.sort();
    out
}

#[test]
fn install_never_leaves_the_project() {
    for host in HOSTS {
        let (p, home, run) = first_install(host);
        install(&p, &home, &["install", host, "--check"]);
        install(&p, &home, &["uninstall", host]);
        // the wasmtime module cache is all the home holds
        let held = names(home.path());
        assert!(
            held.iter().all(|n| n == ".cache"
                || n == "bin"
                || n.starts_with(".cache/specforge")
                || n.starts_with("bin/")),
            "{host}: {held:?}"
        );
        // every path the install reported is relative and under the root
        for f in report(&run)["hosts"][0]["files"].as_array().expect("files") {
            let path = f["path"].as_str().unwrap_or_default();
            assert!(
                !path.starts_with('/') && !path.contains(".."),
                "{host}: {path}"
            );
            assert!(
                owned(host)
                    .iter()
                    .any(|o| path == *o || path.starts_with(&format!("{o}/"))),
                "{host}: {path}"
            );
        }
    }
}

/// D-r44-23: no golden holds the extension's version, so the next bump moves nothing but `Cargo.toml` and the lock.
#[test]
fn the_marker_version_is_normalised() {
    fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("a golden folder") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                files(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let mut all = Vec::new();
    files(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/install"),
        &mut all,
    );
    let marked: Vec<_> = all
        .iter()
        .filter(|f| std::fs::read_to_string(f).is_ok_and(|t| t.contains("@gsd/core@")))
        .collect();
    assert_eq!(
        marked.len(),
        65,
        "15 agent and 50 skill goldens hold the marker"
    );
    for file in marked {
        let text = std::fs::read_to_string(file).expect("a golden");
        assert_eq!(
            normalised(&text),
            text,
            "{} holds a version, not VERSION",
            file.display()
        );
    }
}

#[test]
fn a_broken_specforge_json_refuses_install_and_keeps_every_guard() {
    for host in HOSTS {
        let (p, home, _) = first_install(host);
        p.write("specforge.json", "{bad");
        let before = tree(&p);
        let run = install(&p, &home, &["install", host, "--format", "json"]);
        assert_eq!(run.code, 1, "{host}: {}", run.all());
        assert_eq!(report(&run)["code"], "E069", "{host}: {}", run.all());
        assert_eq!(tree(&p), before, "{host}: install wrote");
    }
}
