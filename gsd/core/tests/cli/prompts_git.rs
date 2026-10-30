//! `undo` and `pr_branch` (T7): they run `git` in the person's repository, so the tests render and read the text.

use std::process::Command;

use super::commands_support::twin;
use super::harness::Project;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

#[test]
fn undo_renders() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "undo",
        &["--plan", "v1.0/01/01-01"],
        "claude-code",
    )));
    for word in [
        "--plan v1.0/01/01-01",
        "specforge query \"${TARGET_PLAN_ID}\" --depth 2",
        "specforge check",
        "ask the user before reverting",
        "git revert --no-commit",
    ] {
        assert!(text.contains(word), "no '{word}'");
    }
}

#[test]
fn undo_refuses_a_missing_plan() {
    let p = twin();
    let run = prompt(&p, "undo", &["--plan", "v1.0/01/99-99"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E003"), "{}", run.all());
}

#[test]
fn undo_renders_for_three_hosts() {
    let p = twin();
    render_three_hosts(
        &p,
        "undo",
        &["--last", "2", "--phase", "v1.0/01"],
        "--last 2 --phase v1.0/01",
        "Safe git revert",
    );
}

#[test]
fn pr_branch_renders_for_three_hosts() {
    let p = twin();
    for (host, rendered) in
        render_three_hosts(&p, "pr_branch", &["--target", "main"], "--target", "PR")
    {
        let text = text_of(&rendered);
        for word in [
            "SPEC_ROOT",
            "specforge-harness",
            "git cherry-pick --no-commit",
        ] {
            assert!(text.contains(word), "{host}: no '{word}'");
        }
    }
}

#[test]
fn pr_branch_description_names_no_planning() {
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden/install/claude-code/skills/gsd-pr-branch.md"),
    )
    .expect("the skill golden");
    let description = skill
        .lines()
        .find(|l| l.starts_with("description:"))
        .expect("a description");
    assert!(!description.contains(".planning"), "{description}");
    assert!(description.contains("spec-only"), "{description}");
}

/// The twin with a second milestone `v2.0` that has its own phase `01` (and, when `closed`, is abandoned).
fn two_milestones(closed: bool) -> Project {
    let p = twin();
    let ruling = if closed {
        "  abandoned { reason \"dropped\" ruled_by \"tester\" at 2026-10-01 }\n"
    } else {
        ""
    };
    p.write(
        "spec/v2.spec",
        &format!(
            "milestone v2.0 \"Next\" {{\n  name \"Next\"\n{ruling}}}\n\nphase 01 \"Next one\" {{\n  milestone v2.0\n  name \"Next one\"\n  slug \"next-one\"\n  goal \"g\"\n  success_criteria [\"a\"]\n}}\n"
        ),
    );
    p
}

/// Runs the lines of the rendered undo text that start with `starts` (bash, in the project, `specforge` on the path)
/// after `setup`, and answers what `echo` printed.
fn undo_shell(p: &Project, text: &str, setup: &str, starts: &[&str], echo: &str) -> String {
    let mut script = String::from(setup);
    script.push('\n');
    for line in text.lines() {
        if starts.iter().any(|s| line.starts_with(s)) {
            script.push_str(line);
            script.push('\n');
        }
    }
    script.push_str(echo);
    let bin = std::path::PathBuf::from(std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN"));
    let path = format!(
        "{}:{}",
        bin.parent().expect("a directory").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .args(["-c", &script])
        .env("PATH", path)
        .current_dir(p.root())
        .output()
        .expect("bash");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// REVIEW-r43 M1: `--phase v2.0/01` rendered as `--phase 01`, so the query resolved `v1.0/01` (the open milestone)
/// and offered that phase's commits for `git revert`.
#[test]
fn undo_keeps_the_milestone_of_the_phase_it_was_given() {
    let p = two_milestones(false);
    let text = text_of(&payload(&prompt(
        &p,
        "undo",
        &["--phase", "v2.0/01"],
        "claude-code",
    )));
    assert!(
        text.contains("<arguments>--phase v2.0/01</arguments>"),
        "no full id"
    );
    let got = undo_shell(
        &p,
        &text,
        "MILESTONE=v1.0; TARGET_PHASE=v2.0/01",
        &[
            "UNDO_MILESTONE=$MILESTONE; case \"$TARGET_PHASE\"",
            "PHASE_DIR=$(specforge query \"$UNDO_MILESTONE/${TARGET_PHASE}\"",
        ],
        "echo \"$UNDO_MILESTONE $MILESTONE $TARGET_PHASE $PHASE_DIR\"",
    );
    assert_eq!(got, "v2.0 v1.0 01 v2.0/01");
    let plan = text_of(&payload(&prompt(
        &p,
        "undo",
        &["--plan", "v1.0/01/01-01"],
        "claude-code",
    )));
    let got = undo_shell(
        &p,
        &plan,
        "MILESTONE=v2.0; TARGET_PLAN=v1.0/01/01-01",
        &["UNDO_MILESTONE=$MILESTONE; case \"$TARGET_PLAN\""],
        "echo \"$UNDO_MILESTONE $MILESTONE $TARGET_PLAN $TARGET_PLAN_ID $PLAN_PHASE\"",
    );
    assert_eq!(got, "v1.0 v2.0 01-01 v1.0/01/01-01 01");
}

/// Upstream refuses a phase of an archived milestone; a closed milestone is the archive here.
#[test]
fn undo_refuses_a_phase_of_a_closed_milestone() {
    for (closed, expect) in [(true, "v2.0/01"), (false, "")] {
        let p = two_milestones(closed);
        let text = text_of(&payload(&prompt(
            &p,
            "undo",
            &["--phase", "v2.0/01"],
            "claude-code",
        )));
        let got = undo_shell(
            &p,
            &text,
            // a fresh shell: only the open milestone is bound, so the phase id must name the milestone
            "MILESTONE=v1.0; PHASE_DIR=v2.0/01",
            &[
                "MILESTONE_STATE=$(specforge query \"${UNDO_MILESTONE",
                "PHASE_DIR_ARCHIVED=\"\"; case",
            ],
            "echo \"$PHASE_DIR_ARCHIVED\"",
        );
        assert_eq!(got, expect, "closed={closed}");
    }
}

/// `--from`, `--to` and `--only` are numbers of the open milestone, and say so.
#[test]
fn autonomous_bounds_stay_numbers_of_the_open_milestone() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "autonomous",
        &["--from", "v1.0/01", "--to", "v1.0/02"],
        "claude-code",
    )));
    assert!(text.contains("<arguments>--from 01 --to 02</arguments>"));
    assert!(text.contains("a phase of another milestone is not a bound here"));
}

/// Runs `script` with bash in `dir`, answering stdout.
fn bash_in(dir: &std::path::Path, script: &str) -> String {
    let out = Command::new("bash")
        .args(["-ec", script])
        .current_dir(dir)
        .output()
        .expect("bash");
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// REVIEW-r43 M6: `specforge-harness` is a file at the project root, not a directory, and only the spec root is the
/// spec: a commit of a `.spec` and the harness alone is planning noise, a build's `app.spec` is code.
#[test]
fn pr_branch_filters_the_harness_file_and_the_spec_root() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "pr_branch",
        &["--target", "main"],
        "claude-code",
    )));
    let fences = super::rendered_lines::bash_fences(&text);
    let pick = |needle: &str| {
        fences
            .iter()
            .find(|f| f.contains(needle))
            .unwrap_or_else(|| panic!("no fence with {needle}"))
            .clone()
    };
    let (declare, derive, classify) = (
        pick("TRANSIENT_FILES="),
        pick("FILTER_PATHS=$(printf"),
        pick("NON_PLANNING=$(echo"),
    );
    let repo = tempfile::tempdir().expect("a repo");
    bash_in(
        repo.path(),
        "git init -q -b main . && git config user.email t@t && git config user.name t \
         && echo '{\"spec_root\":\"spec\"}' > specforge.json && mkdir spec src && echo a > src/a.rs \
         && echo s > spec/x.spec && echo h > specforge-harness && echo b > app.spec && git add -A && git commit -qm base \
         && git checkout -qb feat \
         && echo s2 >> spec/x.spec && echo h2 >> specforge-harness && git commit -qam spec-only \
         && echo a2 >> src/a.rs && echo h3 >> specforge-harness && git commit -qam mixed \
         && echo b2 >> app.spec && git commit -qam build-spec",
    );
    let script = format!(
        "PR_STRICT=true\n{declare}\n{derive}\nfor HASH in $(git rev-list --reverse main..feat); do\n{classify}\n\
         echo \"$(git log -1 --format=%s $HASH) $NON_PLANNING $PLANNING_COUNT\"; done\necho \"$FILTER_PATHS\"\n"
    );
    let got = bash_in(repo.path(), &script);
    assert_eq!(
        got, "spec-only 0 2\nmixed 1 1\nbuild-spec 1 0\nspec/\nspecforge-harness",
        "{got}"
    );
}

/// REVIEW-r43 M5: `git.base_branch` is declared, so pr-branch and quick start from it; the fallbacks are the remote's
/// head, then `main` (not `init.defaultBranch`, the user's default for new repositories).
#[test]
fn pr_branch_and_quick_start_from_the_declared_base_branch() {
    for (id, args, marker) in [
        ("pr_branch", vec![], "TARGET=${TARGET:-"),
        ("quick", vec!["--task", "x"], "DEFAULT_BRANCH=$("),
    ] {
        let p = twin();
        let line_of = |p: &Project| {
            let text = text_of(&payload(&prompt(p, id, &args, "claude-code")));
            let at = text
                .find(marker)
                .unwrap_or_else(|| panic!("{id}: no {marker}"));
            let mut end = text[at..].find("\n\n").map_or(text.len(), |n| at + n);
            if id == "pr_branch" {
                end = text[at..].find('\n').map_or(text.len(), |n| at + n);
            } else {
                end = end.min(
                    text[at..]
                        .find("echo main)")
                        .map_or(end, |n| at + n + "echo main)".len()),
                );
            }
            text[at..end].to_string()
        };
        let line = line_of(&p);
        assert!(!line.contains("init.defaultBranch"), "{id}: {line}");
        p.settings(serde_json::json!({ "git.base_branch": "develop" }));
        let set = line_of(&p);
        assert!(set.contains("\"develop\""), "{id}: {set}");
        // run it in a repository with no remote: the setting wins, and without it the answer is main
        let repo = tempfile::tempdir().expect("a repo");
        bash_in(
            repo.path(),
            "git init -q -b main . && git config user.email t@t && git config user.name t && git commit -q --allow-empty -m x",
        );
        let var = if id == "pr_branch" {
            "TARGET"
        } else {
            "DEFAULT_BRANCH"
        };
        let pre = if id == "pr_branch" {
            "ARGUMENTS=''\n"
        } else {
            ""
        };
        let run = |l: &str| bash_in(repo.path(), &format!("{pre}{l}\necho \"${var}\""));
        assert_eq!(run(&set), "develop", "{id}");
        assert_eq!(run(&line), "main", "{id}");
    }
}
