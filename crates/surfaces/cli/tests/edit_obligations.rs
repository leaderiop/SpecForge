//! `--verify`'s `kind:text` on `link`, `unlink` and `new` (ADR 0237): the prefix is a kind only when it is one of
//! the entity's kind's verify kinds, and a text names the obligation the entity writes in either reading.

use crate::support::{Cli, Run, TestProject};

const TASKS: &str = "spec/behaviors/task.spec";

fn todo() -> TestProject {
    TestProject::copy_of(specforge_repo_root::repo_root().join("examples/todo-app"))
}

fn sf(project: &TestProject, args: &[&str]) -> Run {
    Cli::on(project).run(args)
}

#[ignore = "flipped by T3"]
#[specforge_test_macros::test(
    behavior = "edit_entity_obligation",
    verify = "--verify reads a prefix as a kind only when it is one of the entity's kind's verify kinds"
)]
fn a_verify_prefix_is_a_kind_only_when_the_entitys_kind_accepts_it() {
    let project = todo();
    for (given, written) in [
        (
            "contract: the list is sorted",
            r#"verify contract "the list is sorted""#,
        ),
        (
            "Given: a done task, it is not listed",
            r#"verify "Given: a done task, it is not listed""#,
        ),
        (
            "http://example.com/tasks answers",
            r#"verify "http://example.com/tasks answers""#,
        ),
    ] {
        let run = sf(&project, &["link", "list_tasks", "--verify", given]).exits(0);
        let out = format!("{}{}", run.stdout, run.stderr);
        assert!(!out.contains("W009"), "{given}: {out}");
        assert!(
            project.read(TASKS).contains(written),
            "{given}\n{}",
            project.read(TASKS)
        );
    }
    // `new` reads the new kind's verify kinds as `link` reads the entity's.
    let run = sf(
        &project,
        &[
            "new",
            "behavior",
            "archive_task",
            "Archive a Task",
            "--in",
            "behaviors/task.spec",
            "--category",
            "command",
            "--types",
            "Task,TaskNotFoundError",
            "--ports",
            "TaskRepository",
            "--features",
            "task_management",
            "--contract",
            "The system MUST archive a done task.",
            "--verify",
            "note: archiving is undoable",
            "--verify",
            "unit:archiving twice is one archive",
            "--dry-run",
        ],
    )
    .exits(0);
    assert!(
        run.stdout
            .contains(r#"+  verify "note: archiving is undoable""#),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout
            .contains(r#"+  verify unit "archiving twice is one archive""#),
        "{}",
        run.stdout
    );
}

#[ignore = "flipped by T3"]
#[specforge_test_macros::test(
    behavior = "edit_entity_obligation",
    verify = "--verify names the obligation the entity writes in either reading of its prefix"
)]
fn unlink_names_the_obligation_written_in_either_reading_of_its_prefix() {
    let project = todo();
    let before = project.read(TASKS);
    let line = "  verify unit \"listing open tasks excludes done tasks\"\n";
    // Kindless with an accepted kind's prefix (as MCP writes it), and a kind `behavior` does not accept (as `link`
    // wrote `Given:` before ADR 0237).
    project.write(
        TASKS,
        before.replace(
            line,
            &format!(
                "{line}  verify \"unit: listing is stable\"\n  verify Given \"a done task, it is not listed\"\n"
            ),
        ),
    );
    let again = sf(
        &project,
        &["link", "list_tasks", "--verify", "unit: listing is stable"],
    )
    .exits(0);
    assert!(
        again.stdout.contains("nothing to write"),
        "{}",
        again.stdout
    );
    sf(
        &project,
        &[
            "unlink",
            "list_tasks",
            "--verify",
            "unit: listing is stable",
        ],
    )
    .exits(0);
    sf(
        &project,
        &[
            "unlink",
            "list_tasks",
            "--verify",
            "Given: a done task, it is not listed",
        ],
    )
    .exits(0);
    assert_eq!(project.read(TASKS), before);
}
