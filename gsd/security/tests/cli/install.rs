//! What `specforge install <host>` writes for the files `@gsd/security` owns (plan 16 T23): the auditor on each host
//! (the full body on Claude Code, the compact one on Codex and Cursor) and the `/gsd-secure-phase` skill, pinned by
//! goldens, with the report rows that name them. Core's own files are not compared here: its goldens cover them,
//! unchanged.

use std::path::Path;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::harness::{Project, project};
use super::hooks_support::Home;

const HOSTS: [&str; 3] = ["claude-code", "codex", "cursor"];

/// The files of `@gsd/security` on a host: its agent and its command's skill.
fn owned(host: &str) -> [&'static str; 2] {
    match host {
        "codex" => [
            ".codex/agents/gsd-security-auditor.toml",
            ".codex/skills/gsd-secure-phase/SKILL.md",
        ],
        "cursor" => [
            ".cursor/agents/gsd-security-auditor.md",
            ".cursor/skills/gsd-secure-phase/SKILL.md",
        ],
        _ => [
            ".claude/agents/gsd-security-auditor.md",
            ".claude/skills/gsd-secure-phase/SKILL.md",
        ],
    }
}

/// `text` with the marker's `@gsd/security@<version>` read as `@gsd/security@VERSION`.
fn normalised(text: &str) -> String {
    let key = "@gsd/security@";
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

/// A file as a golden: its first 40 lines, then the count, size and digest of the whole.
fn summary(text: &str) -> String {
    let text = normalised(text);
    let head: Vec<&str> = text.lines().take(40).collect();
    format!(
        "{}\n\n--- lines: {}, bytes: {}, sha256: {:x}\n",
        head.join("\n"),
        text.lines().count(),
        text.len(),
        Sha256::digest(text.as_bytes())
    )
}

fn check_golden(host: &str, name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/install")
        .join(host)
        .join(name);
    if std::env::var_os("GSD_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("the folder");
        std::fs::write(&path, actual).expect("write the golden");
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{} is missing: GSD_BLESS=1 writes it", path.display()));
    assert!(
        expected == actual,
        "{host}/{name} differs from its golden: review the change, then GSD_BLESS=1\n--- expected\n{expected}\n--- actual\n{actual}"
    );
}

fn both() -> Project {
    project("empty").with_extension("@gsd/security", "GSD_SECURITY_WASM")
}

fn install(p: &Project, home: &Home, host: &str) -> Value {
    let h = home.path().display().to_string();
    let run = p.run_with(&["install", host, "--format", "json"], &[("HOME", &h)]);
    assert_eq!(run.code, 0, "install {host}: {}", run.all());
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("{e}: {}", run.all()))
}

/// The report rows that name a file `@gsd/security` owns.
fn rows(report: &Value, host: &str) -> Value {
    let files = report["hosts"][0]["files"].as_array().expect("files");
    Value::Array(
        files
            .iter()
            .filter(|f| owned(host).iter().any(|p| f["path"] == *p))
            .cloned()
            .collect(),
    )
}

#[test]
fn install_writes_the_auditor_and_the_skill_on_three_hosts() {
    for host in HOSTS {
        let (p, home) = (both(), Home::new());
        let report = install(&p, &home, host);
        for file in owned(host) {
            let name = Path::new(file)
                .file_name()
                .expect("a name")
                .to_string_lossy();
            let dir = if file.contains("/agents/") {
                "agents"
            } else {
                "skills"
            };
            check_golden(host, &format!("{dir}-{name}"), &summary(&p.read(file)));
        }
        let rows = rows(&report, host);
        assert_eq!(rows.as_array().map(Vec::len), Some(2), "{host}: {rows}");
        assert!(
            rows.as_array()
                .expect("rows")
                .iter()
                .all(|r| r["action"] == "created")
        );
        check_golden(
            host,
            "report.json",
            &(serde_json::to_string_pretty(&json!({ "rows": rows })).expect("JSON") + "\n"),
        );
    }
}

/// `workflow.compact_content` (a setting of `@gsd/core`) presets the auditor's variant: Codex and Cursor get the compact
/// body, Claude Code keeps the full one (upstream installs no compact agent for it).
#[test]
fn the_compact_body_is_for_codex_and_cursor_the_full_one_for_claude_code() {
    let body = |text: &str| {
        text.split_once('\n')
            .map_or(String::new(), |(_, rest)| rest.to_string())
    };
    for host in HOSTS {
        // what the install renders: the body with its references repointed for the host
        let render = |text: &str| body(&gsd_kit::prompt::repoint::repoint(text, host, &|_| true));
        let full = render(gsd_security::agents::FULL);
        let compact = render(gsd_security::agents::COMPACT);
        let (p, home) = (both(), Home::new());
        p.settings_of("@gsd/core", json!({"workflow.compact_content": true}));
        install(&p, &home, host);
        let agent = p.read(owned(host)[0]);
        if host == "claude-code" {
            assert!(
                agent.contains(full.trim()) && !agent.contains(compact.trim()),
                "{host}"
            );
        } else {
            assert!(
                agent.contains(compact.trim()) && !agent.contains(full.trim()),
                "{host}"
            );
            check_golden(host, "agents-compact", &summary(&agent));
        }
    }
}

#[test]
fn a_second_install_changes_nothing() {
    for host in HOSTS {
        let (p, home) = (both(), Home::new());
        install(&p, &home, host);
        let first: Vec<String> = owned(host).iter().map(|f| p.read(f)).collect();
        let again = install(&p, &home, host);
        let again = rows(&again, host);
        assert!(
            again
                .as_array()
                .expect("rows")
                .iter()
                .all(|r| r["action"] == "unchanged"),
            "{host}: {again}"
        );
        let second: Vec<String> = owned(host).iter().map(|f| p.read(f)).collect();
        assert_eq!(first, second, "{host}");
    }
}

#[test]
fn without_security_nothing_of_it_is_installed() {
    for host in HOSTS {
        let (p, home) = (project("empty"), Home::new());
        install(&p, &home, host);
        for file in owned(host) {
            assert!(!p.exists(file), "{host}: {file}");
        }
    }
}

/// REVIEW-security-part2 M4: the installed auditor reads core's references through `specforge gsd reference`, as core's
/// agents do; `~/.claude/gsd-core/` does not exist in a SpecForge install.
#[test]
fn the_installed_auditor_names_no_upstream_install_path() {
    for host in HOSTS {
        for compact in [false, true] {
            let (p, home) = (both(), Home::new());
            p.settings_of("@gsd/core", json!({ "workflow.compact_content": compact }));
            install(&p, &home, host);
            let agent = p.read(owned(host)[0]);
            assert!(
                !agent.contains("~/.claude/gsd-core/") && !agent.contains(".claude/gsd-core"),
                "{host} compact={compact}: {agent}"
            );
            assert!(agent.contains("specforge gsd reference"), "{host}");
        }
    }
}
