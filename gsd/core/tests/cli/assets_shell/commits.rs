//! The executor's task commit, run as the asset writes it: the commit it makes is linked to its task.

use super::{Shell, asset, fence_after};

const AT: &str = "2026-10-29T09:00:00Z";

/// The single-repo commit block of `gsd-executor.md`, with its placeholders filled, run under bash with nothing but
/// `specforge` on `PATH` (ADR 0343); the gsd-git collector then records the commit against `01-01`'s task `T1`.
#[test]
fn the_executors_task_commit_links_its_task_by_trailer() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = crate::harness::project("execute");
    p.commit();
    // The repository's own identity: the developer's configuration is never read.
    for (key, value) in [
        ("user.name", "gsd"),
        ("user.email", "gsd@example.com"),
        ("commit.gpgsign", "false"),
        ("core.hooksPath", "/dev/null"),
    ] {
        p.git(&["config", key, value], AT);
    }
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
