//! new-project's sub-repo step, run as the asset holds it under bash with only `specforge` and the shell's tools.

use super::{Shell, asset, fence_after};

/// The fence of new-project's "5.1. Sub-Repo Detection" step, then `show`, run in a project holding a directory repo (`backend`), a
/// linked worktree (`linked-wt`, whose `.git` is a file) and a plain directory (`docs`), then what `setup` adds:
/// `(stdout, stderr, exit code)`.
fn sub_repo_step(
    sh: &Shell,
    show: &str,
    setup: impl Fn(&std::path::Path),
) -> (String, String, i32) {
    let code = fence_after(
        &asset("gsd-core/workflows/new-project.md"),
        "## 5.1. Sub-Repo Detection",
    );
    let p = crate::harness::project("empty");
    p.commit();
    p.git(&["init", "-q", "backend"], "2026-01-01T00:00:00Z");
    p.git(
        &["worktree", "add", "-q", "linked-wt"],
        "2026-01-01T00:00:00Z",
    );
    p.write("docs/README.md", "docs\n");
    setup(p.root());
    let run = sh.run_in(p.root(), &format!("{code}\n{show}"));
    #[cfg(unix)]
    {
        // a setup that made the root unreadable leaves it removable
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(p.root(), std::fs::Permissions::from_mode(0o755)).expect("a mode");
    }
    run
}

/// The step lists each child directory that is a repository, a directory repo and a linked worktree alike, by its
/// bare name, and not a plain directory.
#[test]
fn new_project_lists_the_sub_repos() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let (out, err, _) = sub_repo_step(&sh, "printf '%s\\n' \"$SUB_REPOS\"", |_| {});
    assert_eq!(out, "backend\nlinked-wt\n", "{err}");
}

/// A listing that fails (the workspace cannot be read) stops the step: it is not "no sub-repos".
#[cfg(unix)]
#[test]
fn a_failed_sub_repo_listing_stops_the_step() {
    use std::os::unix::fs::PermissionsExt;
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let (out, err, code) = sub_repo_step(&sh, "echo listed", |root| {
        std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o300)).expect("a mode");
    });
    assert!(!out.contains("listed") && code != 0, "{out}{err}");
}

/// A directory name holding a newline is refused whole, and the step stops, never reading it as two names.
#[cfg(unix)]
#[test]
fn a_sub_repo_name_holding_a_newline_stops_the_step() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let (out, err, code) = sub_repo_step(&sh, "printf '%s\\n' \"$SUB_REPOS\"", |root| {
        let odd = root.join("we\nird");
        std::fs::create_dir_all(odd.join(".git")).expect("a dir");
    });
    assert!(out.is_empty() && code != 0, "{out}{err}");
    assert!(err.contains("a newline"), "{err}");
}
