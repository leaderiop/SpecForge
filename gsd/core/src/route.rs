//! The route `specforge trace` follows from a requirement (ADR 0330, ADR 0331 D8): its plans, their tasks, the
//! commits that name those, the tests its truths cite as `proof` (with their status) and its truths (with their E501
//! judgement). Without `@code/core` the `test` step is inert, as every selector into an unloaded optional peer's kind.

use specforge_extension_sdk::prelude::*;

/// Declare `requirement-thread` (`gsd/requirement-thread` once qualified).
pub fn declare(c: &mut ContributionsBuilder) {
    c.route("requirement-thread", |r| {
        r.start("requirement")
            .description(
                "A requirement's thread: its plans, tasks, commits, proving tests and truths",
            )
            .step("plan", "<plan.requirements", |s| {
                s.label("plan");
            })
            .step("task", "<task.plan", |_| {})
            .step("commit", "<commit.task", |s| {
                s.field("linked_by");
            })
            .step("test", "<truth.requirement.proof", |s| {
                s.from_start().annotate(|a| {
                    a.field("status").absent("not run");
                });
            })
            .step("truth", "<truth.requirement", |s| {
                s.from_start().judgement("E501");
            });
    });
}
