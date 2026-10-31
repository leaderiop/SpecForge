//! `commit` (`spec/execute.md`): a git commit that names a task or a plan, recorded by the `gsd-git` collector.

use gsd_kit::dsl::*;

/// A commit of the project's repository, named by the first 12 hex digits of its hash and recorded in its plan's own
/// file. Every field is written by the collector (`H`): the host refuses any other writer. The `hash` is not required
/// (the DSL refuses a required harness field), and its name already carries it. `task.commits` and `plan.commits` are
/// the inverses of the two references; GSD has no commit trailer, so a task is named by the subject alone.
// Paths are `strs`, not `files`: a later commit may delete or rename them, and the host's E016 would fire. They are
// repository-relative, as git prints them.
pub const COMMIT: Kind = Kind::new(
    "commit",
    "A git commit that names a task or a plan, recorded from the repository's history",
    &[
        str("hash")
            .h()
            .i()
            .doc("the full 40-hex hash; the name is its first 12 digits"),
        rf("task", "task")
            .h()
            .inverse("commits")
            .doc("the task its subject names, when it does"),
        rf("plan", "plan")
            .h()
            .inverse("commits")
            .doc("`{type}({phase}-{plan}): {task}`: the plan its scope names"),
        str("type").h().doc("`feat`, `fix`, `test`, `docs` ..."),
        str("subject").h().doc("the first line of the message"),
        ts("at").h().doc("the author time, in UTC"),
        strs("files").h().doc("the paths it touched"),
        strs("created")
            .h()
            .doc("the paths it created, from `git log --summary`"),
        int("insertions").h().doc("added lines"),
        int("deletions").h().doc("deleted lines"),
        rfs("descends_from", "commit")
            .h()
            .doc("the recorded commits of its plan it descends from, through merges; absent when never collected"),
    ],
);
