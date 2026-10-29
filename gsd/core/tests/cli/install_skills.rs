//! What `specforge install claude-code` writes for the slash commands: a golden per skill file.

use std::path::Path;

use super::harness::project;

const SKILLS: [&str; 32] = [
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
];

fn check_golden(id: &str, actual: &str) {
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
