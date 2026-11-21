//! The commits the assets make, run as the assets write them: the executor's task commit is linked to its task, and
//! plan-phase's plan commit commits the spec with no setting deciding it.

use super::{Shell, asset, fence_after};
use crate::harness::Project;

const AT: &str = "2026-10-29T09:00:00Z";

/// The single-repo commit block of `gsd-executor.md`, with its placeholders filled, run under bash with nothing but
/// `specforge` on `PATH` (ADR 0343); the gsd-git collector then records the commit against `01-01`'s task `T1`.
#[test]
fn the_executors_task_commit_links_its_task_by_trailer() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = crate::harness::project("execute");
    committed(&p);
    p.write("src/tasks.js", "export const tasks = [];\n");
    let (_, err, code) = sh.run_in(p.root(), "specforge git add src/tasks.js");
    assert_eq!(code, 0, "{err}");

    let block = fence_after(
        &asset("agents/gsd-executor.md"),
        "**Otherwise (standard single-repo):**",
    );
    let script = [
        ("{type}", "feat"),
        ("{phase}-{plan}", "01-01"),
        ("{concise task description}", "Store module"),
        ("{task}", "T1"),
        ("{key change 1}", "the store module"),
        ("{key change 2}", "its export"),
    ]
    .iter()
    .fold(block, |text, (from, to)| text.replace(from, to));
    let (out, err, code) = sh.run_in(p.root(), &script);
    assert_eq!(code, 0, "{script}\n{out}{err}");

    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let head = p.git(&["rev-parse", "HEAD"], AT);
    let fields = p.fields_of(&head.trim()[..12]);
    assert_eq!(fields["task"], "v1.0/01/01-01/T1", "{fields}");
    assert_eq!(fields["linked_by"], "Gsd-Task", "{fields}");
}

/// Step 13d of `plan-phase.md` commits the plans with no setting deciding it (the spec is committed like code), and
/// run again with nothing staged it changes nothing and answers "nothing to commit", which the step tells the agent
/// means an earlier commit of the run holds the plans.
#[test]
fn the_plan_commit_step_commits_the_spec_and_an_empty_one_says_nothing_to_commit() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = crate::harness::taskboard();
    committed(&p);
    let set = p.run(&[
        "set",
        "v1.0/03",
        "goal",
        "A user lists and filters the open tasks",
    ]);
    assert_eq!(set.code, 0, "{}", set.all());

    let workflow = asset("gsd-core/workflows/plan-phase.md");
    assert!(
        workflow.lines().any(|line| line == "## 13d. Commit Plans"),
        "step 13d names no condition"
    );
    let script = format!(
        "PADDED_PHASE=03\n{}",
        fence_after(&workflow, "## 13d. Commit Plans")
    );
    let (out, err, code) = sh.run_in(p.root(), &script);
    assert_eq!(code, 0, "{script}\n{out}{err}");
    let subject = p.git(&["log", "-1", "--format=%s"], AT);
    assert_eq!(subject.trim(), "docs(03): create phase plan");

    let head = p.head();
    let (out, err, code) = sh.run_in(p.root(), &script);
    assert_ne!(code, 0, "{script}\n{out}{err}");
    assert!(
        format!("{out}{err}").contains("nothing to commit"),
        "{out}{err}"
    );
    assert_eq!(p.head(), head, "an empty repeat made a commit");
}

/// Commits the project's files as its baseline and gives the repository its own identity: the developer's
/// configuration is never read.
fn committed(p: &Project) {
    p.commit();
    for (key, value) in [
        ("user.name", "gsd"),
        ("user.email", "gsd@example.com"),
        ("commit.gpgsign", "false"),
        ("core.hooksPath", "/dev/null"),
    ] {
        p.git(&["config", key, value], AT);
    }
}
