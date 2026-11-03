//! `commit` (`spec/execute.md`): a git commit that names a task or a plan, recorded by the `gsd-git` collector.

use gsd_kit::dsl::*;

use crate::collectors::LinkedBy;

/// A commit of the project's repository, named by the first 12 hex digits of its hash and recorded in its plan's own
/// file. Every field is written by the collector (`H`): the host refuses any other writer. The `hash` is not required
/// (the DSL refuses a required harness field), and its name already carries it. `task.commits` and `plan.commits` are
/// the inverses of the two references. A `Gsd-Task: <plan>/<task>` trailer names the task first, else the subject's
/// title does (ADR 0331 D3); `linked_by` says which.
// Paths are `strs`, not `files`: a later commit may delete or rename them, and the host's E016 would fire. They are
// repository-relative, as git prints them.
/// `linked_by`'s values, as gsd-git writes them.
const LINKED_BY: &[&str] = &[LinkedBy::Trailer.as_str(), LinkedBy::Title.as_str()];

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
            .doc("the task its `Gsd-Task:` trailer or its subject's title names, when one does"),
        rf("plan", "plan")
            .h()
            .inverse("commits")
            .doc("the plan of the task its trailer names, else the one its scope names (`{type}({phase}-{plan}): {task}`)"),
        en("linked_by", LINKED_BY)
            .h()
            .doc("how `task` was found: the `Gsd-Task:` trailer or the subject's title; written only with `task`"),
        str("type").h().doc("`feat`, `fix`, `test`, `docs` ...; absent for a subject with no `type:` header"),
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
