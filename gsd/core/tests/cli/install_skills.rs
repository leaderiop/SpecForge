//! What `specforge install claude-code` writes for the slash commands: a golden per skill file.

use std::path::Path;

use super::harness::project;

const SKILLS: [&str; 50] = [
    "gsd-phase",
    "gsd-workstreams",
    "gsd-cleanup",
    "gsd-help",
    "gsd-health",
    "gsd-stats",
    "gsd-surface",
    "gsd-new-project",
    "gsd-new-milestone",
    "gsd-discuss-phase",
    "gsd-spec-phase",
    "gsd-mvp-phase",
    "gsd-plan-phase",
    "gsd-plan-review-convergence",
    "gsd-ultraplan-phase",
    "gsd-execute-phase",
    "gsd-verify-work",
    "gsd-validate-phase",
    "gsd-add-tests",
    "gsd-audit-uat",
    "gsd-audit-milestone",
    "gsd-complete-milestone",
    "gsd-ship",
    "gsd-next",
    "gsd-progress",
    "gsd-manager",
    "gsd-quick",
    "gsd-quick-batch",
    "gsd-fast",
    "gsd-autonomous",
    "gsd-pause-work",
    "gsd-resume-work",
    "gsd-review",
    "gsd-review-backlog",
    "gsd-settings",
    "gsd-config",
    "gsd-undo",
    "gsd-pr-branch",
    "gsd-milestone-summary",
    "gsd-extract-learnings",
    "gsd-ingest-docs",
    "gsd-onboard",
    "gsd-workspace",
    "gsd-update",
    "gsd-ns-context",
    "gsd-ns-ideate",
    "gsd-ns-manage",
    "gsd-ns-project",
    "gsd-ns-review",
    "gsd-ns-workflow",
];

fn check_golden(id: &str, actual: &str) {
    let actual = super::install_hosts::normalised(actual);
    let actual = actual.as_str();
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/install/claude-code/skills")
        .join(format!("{id}.md"));
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
        "{id} differs from its golden: review the change, then GSD_BLESS=1 cargo test\n--- expected\n{expected}\n--- actual\n{actual}"
    );
}

#[test]
fn install_skills_golden() {
    let p = project("empty");
    let run = p.install("claude-code", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    for skill in SKILLS {
        check_golden(skill, &p.read(&format!(".claude/skills/{skill}/SKILL.md")));
    }
}

#[test]
fn a_second_install_changes_nothing() {
    let p = project("empty");
    assert_eq!(p.install("claude-code", &[]).code, 0);
    let first: Vec<String> = SKILLS
        .iter()
        .map(|s| p.read(&format!(".claude/skills/{s}/SKILL.md")))
        .collect();
    assert_eq!(p.install("claude-code", &[]).code, 0);
    let second: Vec<String> = SKILLS
        .iter()
        .map(|s| p.read(&format!(".claude/skills/{s}/SKILL.md")))
        .collect();
    assert_eq!(first, second);
}

/// `(name, description)` of upstream's 72 commands, from the vendored `commands.json`.
fn upstream_descriptions() -> Vec<(String, String)> {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/upstream/v1.18.0/commands.json"),
    )
    .expect("commands.json");
    let rows: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    rows.as_array()
        .expect("an array")
        .iter()
        .map(|r| {
            (
                r["name"].as_str().expect("a name").to_string(),
                r["description"]
                    .as_str()
                    .expect("a description")
                    .to_string(),
            )
        })
        .collect()
}

/// The `description:` of a rendered skill, unquoted.
fn description_of(skill: &str) -> String {
    let line = skill
        .lines()
        .find_map(|l| l.strip_prefix("description: "))
        .expect("a description");
    line.trim_matches('"').replace("\\\"", "\"")
}

/// The skill directories `gsd-*` a host's skills folder holds.
fn gsd_skills(p: &super::harness::Project, dir: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(p.root().join(dir).join("skills"))
        .expect("a skills folder")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    // a stray `gsd:x` or unprefixed directory is a failure, not something to filter away
    assert!(
        names.iter().all(|n| n.starts_with("gsd-")),
        "{dir}: {names:?}"
    );
    names.sort();
    names
}

/// Every core and router row of the roster is one skill, with upstream's description (pr-branch's and ingest-docs'
/// are port-rewritten), and codex and cursor get the same fifty (probe P7).
#[test]
fn fifty_host_names() {
    let p = project("empty");
    for host in ["claude-code", "codex", "cursor"] {
        let run = p.install(host, &[]);
        assert_eq!(run.code, 0, "{host}: {}", run.all());
    }
    let mut want: Vec<String> = crate::roster::ROSTER
        .iter()
        .filter(|(_, o)| o.is_core())
        .map(|(n, _)| format!("gsd-{n}"))
        .collect();
    want.sort();
    assert_eq!(want.len(), 50);
    for dir in [".claude", ".codex", ".cursor"] {
        assert_eq!(gsd_skills(&p, dir), want, "{dir}");
    }
    let rewritten = ["pr-branch", "ingest-docs"];
    let mut verbatim = 0;
    for (name, description) in upstream_descriptions() {
        let skill = format!("gsd-{name}");
        // the seven command-backed slash commands (r41) describe what the command does here, not upstream's text
        let command_backed = matches!(
            crate::roster::owner(&name),
            Some(crate::roster::Owner::CoreCommand)
        );
        if !want.contains(&skill) || rewritten.contains(&name.as_str()) || command_backed {
            continue;
        }
        for dir in [".claude", ".codex", ".cursor"] {
            let text = p.read(&format!("{dir}/skills/{skill}/SKILL.md"));
            assert_eq!(description_of(&text), description, "{dir}/{skill}");
        }
        verbatim += 1;
    }
    // 50 = 41 with upstream's text + 2 port-rewritten + 7 command-backed (the command's text)
    assert_eq!(verbatim, 41);
}

#[test]
fn install_writes_50_skills_on_claude_code() {
    let p = project("empty");
    assert_eq!(p.install("claude-code", &[]).code, 0);
    assert_eq!(gsd_skills(&p, ".claude").len(), 50);
    assert_eq!(SKILLS.len(), 50);
}
