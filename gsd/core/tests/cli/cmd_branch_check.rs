//! `specforge gsd branch-check [--agent]`: the branch `HEAD` is on, read through the git port, and the settings.

use serde_json::json;

use super::commands_support::{gsd, json};
use crate::harness::{Project, project};

const AT: &str = "2026-01-01T00:00:00Z";

fn answer(p: &Project, args: &[&str]) -> serde_json::Value {
    let mut all = args.to_vec();
    all.extend(["--format", "json"]);
    json(&gsd(p, "branch-check", &all))
}

/// On a case-insensitive file system `git checkout MAIN` leaves `HEAD` on `refs/heads/MAIN`, which is `main`'s ref.
#[test]
fn a_branch_named_as_main_in_another_case_is_protected() {
    let p = project("execute");
    p.commit();
    p.git(
        &["symbolic-ref", "HEAD", "refs/heads/MAIN"],
        "2026-01-01T00:00:00Z",
    );
    let answer = answer(&p, &[]);
    assert_eq!(answer["branch"], "MAIN", "{answer}");
    assert_eq!(answer["protected"], true, "{answer}");
    assert_eq!(answer["blocks_commit"], true, "{answer}");
}

/// A broken override must not read as no override: `feat` stays unprotected only because the setting cannot say so.
#[test]
fn a_broken_protection_setting_refuses_naming_it() {
    let p = project("execute");
    p.commit();
    p.git(&["checkout", "-q", "-b", "feat"], AT);
    for (settings, name) in [
        (
            json!({"git.protected_branches": "feat"}),
            "git.protected_branches",
        ),
        (
            json!({"git.protected_branches": ["feat", 1]}),
            "git.protected_branches",
        ),
        (json!({"git.base_branch": 1}), "git.base_branch"),
    ] {
        p.settings(settings.clone());
        let run = gsd(&p, "branch-check", &[]);
        assert_eq!(run.code, 1, "{settings}: {}", run.all());
        assert!(run.all().contains(name), "{settings}: {}", run.all());
    }
    p.settings(json!({"git.protected_branches": ["feat"]}));
    assert_eq!(answer(&p, &[])["blocks_commit"], true);
}

/// The branches a milestone's work sits on: the ones its strategy names and git has, and no other.
#[test]
fn milestone_branches_answers_what_exists_and_refuses_a_phase_with_no_name() {
    let p = crate::harness::taskboard();
    p.commit();
    let branches = |p: &Project| json(&gsd(p, "milestone-branches", &["v1.0", "--format", "json"]));
    assert_eq!(branches(&p)["strategy"], "none");
    assert_eq!(branches(&p)["branches"], json!([]));
    p.settings(json!({"git.branching_strategy": "phase"}));
    assert_eq!(branches(&p)["branches"], json!([]), "none exists yet");
    p.git(&["branch", "feature/x"], AT);
    p.git(&["branch", "gsd/phase-01-core"], AT);
    assert_eq!(branches(&p)["branches"], json!(["gsd/phase-01-core"]));
    p.settings(json!({"git.branching_strategy": "milestone"}));
    assert_eq!(branches(&p)["branches"], json!([]));
    p.git(&["branch", "gsd/v1.0-taskboard-1-0"], AT);
    assert_eq!(branches(&p)["branches"], json!(["gsd/v1.0-taskboard-1-0"]));
    p.settings(json!({"git.branching_strategy": "phase", "git.phase_branch_template": "--help"}));
    let refused = gsd(&p, "milestone-branches", &["v1.0"]);
    assert_eq!(refused.code, 1, "{}", refused.all());
    assert!(refused.all().contains("gsd/W409"), "{}", refused.all());
    assert_ne!(gsd(&p, "milestone-branches", &["v9.9"]).code, 0);
    assert_ne!(gsd(&p, "milestone-branches", &[""]).code, 0);
}

#[test]
fn branch_check_reads_head_and_the_settings() {
    let p = project("execute");
    p.commit();
    let main = answer(&p, &[]);
    assert_eq!(main["branch"], "main", "{main}");
    assert_eq!(main["protected"], true, "{main}");
    assert_eq!(main["blocks_commit"], true, "{main}");
    assert!(
        main["why"].as_str().is_some_and(|w| w.contains("'main'")),
        "{main}"
    );
    let human = gsd(&p, "branch-check", &[]);
    assert!(
        human.stdout.contains("protected/default branch"),
        "{}",
        human.all()
    );
    assert_eq!(
        gsd(&p, "branch-check", &["--pick", ".blocks_commit"])
            .stdout
            .trim(),
        "true"
    );

    p.settings(json!({"git.allow_default_branch_commits": true}));
    let allowed = answer(&p, &[]);
    assert_eq!(
        allowed["protected"], true,
        "the warnings still show: {allowed}"
    );
    assert_eq!(allowed["overridden"], true, "{allowed}");
    assert_eq!(allowed["blocks_commit"], false, "{allowed}");
    p.settings(json!({}));

    p.git(&["checkout", "-q", "-b", "trunk"], AT);
    assert_eq!(answer(&p, &[])["protected"], true);

    p.git(&["checkout", "-q", "-b", "agent-1"], AT);
    let agent = answer(&p, &["--agent"]);
    assert_eq!(agent["agent"], true, "{agent}");
    assert_eq!(agent["blocks_commit"], false, "{agent}");
    assert_eq!(gsd(&p, "branch-check", &[]).stdout.trim(), "ok: agent-1");

    p.git(&["checkout", "-q", "-b", "feature/x"], AT);
    assert_eq!(answer(&p, &["--agent"])["blocks_commit"], true);
    assert_eq!(answer(&p, &[])["blocks_commit"], false);

    p.git(&["checkout", "-q", "--detach"], AT);
    let detached = answer(&p, &[]);
    assert_eq!(detached["detached"], true, "{detached}");
    assert_eq!(detached["blocks_commit"], true, "{detached}");
    assert!(
        detached["why"]
            .as_str()
            .is_some_and(|w| w.contains("detached")),
        "{detached}"
    );
}
