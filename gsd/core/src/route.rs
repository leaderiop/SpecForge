//! The route `specforge trace` follows from a requirement (ADR 0330, ADR 0331 D8, ADR 0371 D7), read from GSD's own
//! fields and collected data alone: the requirement with its computed `proof` and the status of each test whose tag
//! names it; its plans (`requirements`), their tasks, the commits that name those tasks (with how each is linked), the
//! code items each commit changed (ADR 0352 D3), the plans' truths (with their E501 judgement), and the coverage items
//! of its summaries (`deliverable`, with each check's `ref`).
//!
//! A check's computed status is not printed: a route annotation reads a field's written value first, and a written
//! `checks` block holds no `status`, so the requirement's `proof`, which reads the coverage checks, carries it. A test
//! is an annotation of the start, not a step, since every step is required: a requirement no test tags would never
//! read whole. Without `@code/core` the `item` step and the tests' annotation are inert, as every selector into an
//! unloaded optional peer's kind.

use specforge_extension_sdk::prelude::*;

/// Declare `requirement-thread` (`gsd/requirement-thread` once qualified).
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
}
