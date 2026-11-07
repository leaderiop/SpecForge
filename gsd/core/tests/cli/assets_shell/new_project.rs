//! new-project's sub-repo step, run as the asset holds it under bash with only `specforge` and the shell's tools.

use super::{Shell, asset, block};

/// The fence of new-project's "5.1. Sub-Repo Detection" step, run in a project holding a directory repo (`backend`), a
/// linked worktree (`linked-wt`, whose `.git` is a file) and a plain directory (`docs`): `(stdout, stderr)`.
fn sub_repo_step(sh: &Shell) -> (String, String) {
    let text = asset("gsd-core/workflows/new-project.md");
    let step = block(
        &text,
        "## 5.1. Sub-Repo Detection",
        "**If sub-repos found:**",
    );
    let code: Vec<&str> = step
        .lines()
        .skip_while(|l| *l != "```bash")
        .skip(1)
        .take_while(|l| *l != "```")
        .collect();
    assert!(!code.is_empty(), "no bash fence in the step");
    let p = crate::harness::project("empty");
    p.commit();
    p.git(&["init", "-q", "backend"], "2026-01-01T00:00:00Z");
    p.git(
        &["worktree", "add", "-q", "linked-wt"],
        "2026-01-01T00:00:00Z",
    );
    p.write("docs/README.md", "docs\n");
    let (out, err, _) = sh.run_in(p.root(), &code.join("\n"));
    (out, err)
}

/// Today's defect: the step runs `specforge check --format json` and reads `sub_repos_detected`, which nothing prints,
/// so a workspace's sub-repos are never found.
#[test]
fn new_project_detects_no_sub_repo_today() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let (out, err) = sub_repo_step(&sh);
    assert!(!out.contains("sub_repos_detected"), "{out}{err}");
    assert!(
        !out.contains("backend") && !out.contains("linked-wt"),
        "{out}{err}"
    );
}
