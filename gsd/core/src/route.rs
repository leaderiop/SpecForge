//! The routes `specforge trace` follows. The first, from a requirement (ADR 0330, ADR 0331 D8, ADR 0371 D7), is read
//! from GSD's own fields and collected data alone: the requirement with its computed `proof` and the status of each
//! test whose tag names it; its plans (`requirements`), their tasks, the commits that name those tasks (with how each is linked), the
//! code items each commit changed (ADR 0352 D3), the plans' truths (with their E501 judgement), and the coverage items
//! of its summaries (`deliverable`, with each check's `ref`).
//!
//! A check's computed status is not printed: a route annotation reads a field's written value first, and a written
//! `checks` block holds no `status`, so the requirement's `proof`, which reads the coverage checks, carries it. A test
//! is an annotation of the start, not a step, since every step is required: a requirement no test tags would never
//! read whole. Without `@code/core` the `item` step and the tests' annotation are inert, as every selector into an
//! unloaded optional peer's kind.
//!
//! The second, `milestone-release` (ADR 0390 D10), starts at a milestone: its computed `api_breaks` and `release`
//! (gsd/E533's and gsd/E535's evidence) and the `tag` `gsd-git` recorded, then its phases (a route needs a step).

use specforge_extension_sdk::prelude::*;

/// Declare `requirement-thread` and `milestone-release` (`gsd/<name>` once qualified).
pub fn declare(c: &mut ContributionsBuilder) {
    c.route("requirement-thread", |r| {
        r.start("requirement")
            .description(
                "A requirement's thread: its plans, tasks, their commits and the code items they changed, the plans' \
                 truths, and its summaries' coverage items",
            )
            .field("proof")
            .annotate(|a| {
                a.of("<test.names").field("status");
            })
            .step("plan", "<plan.requirements", |s| {
                s.label("plan");
            })
            .step("task", "<task.plan", |_| {})
            .step("commit", "<commit.task", |s| {
                s.from_step("task").field("linked_by");
            })
            .step("item", ".items_changed", |_| {})
            .step("truth", "<truth.plan", |s| {
                s.from_step("plan").judgement("E501");
            })
            .step("deliverable", "<deliverable.requirement", |s| {
                s.from_start().label("deliverable").field("checks.ref");
            });
    });
    c.route("milestone-release", |r| {
        r.start("milestone")
            .description(
                "A milestone's release evidence: its API breaks, its release (tag, commit, packages) and its \
                 recorded tag, and its phases",
            )
            .field("api_breaks")
            .field("release")
            .field("tag")
            .step("phase", "<phase.milestone", |s| {
                s.label("phase");
            });
    });
}
