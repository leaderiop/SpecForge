//! new-project's sub-repo step, run as the asset holds it under bash with only `specforge` and the shell's tools, and
//! the project and requirement fences its steps 4 and 7 run, as the templates hold them.

use super::{Shell, asset, fence_after, typed};
use crate::harness::taskboard;

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

/// The project template's fence, filled for project `notes` with one constraint, one exclusion and decision `D-03`
/// (the taskboard's project decisions hold `D-01` and `D-02`).
fn project_fence() -> String {
    typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/templates/project.md"),
            "## The Project's Typed Edits",
        ),
        &[
            ("SLUG", "notes"),
            ("NAME", "Notes"),
            ("WHAT", "A note keeper for the terminal"),
            ("CORE_VALUE", "A note is never lost"),
            ("CONTEXT", "Built for one person's own notes"),
            (
                "CONSTRAINTS",
                r#"[{"kind": "tech_stack", "what": "Node 20", "why": "it ships as one script"}]"#,
            ),
            (
                "OUT_OF_SCOPE",
                r#"[{"item": "Sync", "why": "one machine is enough for now"}]"#,
            ),
            ("DECISION_ID", "D-03"),
            ("DECISION", "Notes are plain text files"),
            ("RATIONALE", "Any editor opens them"),
        ],
    )
}

/// The requirements template's fence, filled for one requirement of `tier`; the `--reason` flag left out unless the
/// requirement is out of scope, as the template says.
fn requirement_fence(id: &str, text: &str, tier: &str) -> String {
    let fence = typed::typed_fence(
        &asset("gsd-core/templates/requirements.md"),
        "## The Requirements' Typed Edits",
    );
    let fence = if tier == "out_of_scope" {
        fence
    } else {
        fence.replace(r#" --reason "$REASON""#, "")
    };
    typed::filled(
        &fence,
        &[
            ("REQ_ID", id),
            ("TEXT", text),
            ("CATEGORY", "Notes"),
            ("TIER", tier),
            ("REASON", "One machine is enough for now"),
        ],
    )
}

/// On a taskboard copy without its project, the project fence records the project with its decision, and the
/// requirement fence records a v1, a v2 and an out-of-scope requirement. The v1 requirement is listed in phase
/// `v1.0/03` first, as the roadmapper maps it: `gsd/E101` binds once phases exist. The check reports nothing new.
#[test]
fn the_project_fence_records_the_project_and_its_requirements() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    std::fs::remove_file(p.root().join("spec/project.spec")).expect("the project file");
    let roadmap = p.read("spec/v1.0.spec");
    let listed = "requirements     [TASK-03, LIST-01]";
    assert!(roadmap.contains(listed), "{roadmap}");
    p.write(
        "spec/v1.0.spec",
        &roadmap.replace(listed, "requirements     [TASK-03, LIST-01, NOTE-01]"),
    );
    let mut added = Vec::new();
    for (name, script) in [
        ("project", project_fence()),
        (
            "v1",
            requirement_fence("NOTE-01", "User can add a note", "v1"),
        ),
        (
            "v2",
            requirement_fence("NOTE-02", "User can tag a note", "v2"),
        ),
        (
            "out of scope",
            requirement_fence("NOTE-03", "Notes sync across machines", "out_of_scope"),
        ),
    ] {
        let run = typed::apply(&sh, &p, &script);
        assert_eq!(run.code, 0, "{name}: {}{}", run.out, run.err);
        added.extend(run.added);
    }
    assert!(added.is_empty(), "{added:?}");
    let project = p.fields_of("notes");
    assert_eq!(
        project["key_decisions"],
        serde_json::json!(["D-03"]),
        "{project}"
    );
    assert_eq!(project["core_value"], "A note is never lost", "{project}");
    assert_eq!(p.fields_of("D-03")["origin"], "project");
    for (id, tier) in [
        ("NOTE-01", "v1"),
        ("NOTE-02", "v2"),
        ("NOTE-03", "out_of_scope"),
    ] {
        assert_eq!(p.fields_of(id)["tier"], tier, "{id}");
    }
    assert_eq!(
        p.fields_of("NOTE-03")["reason"],
        "One machine is enough for now"
    );
}
