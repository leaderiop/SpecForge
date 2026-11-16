//! `invariant` (`spec/governance.md`): beyond GSD.

use gsd_kit::dsl::*;

/// How far an invariant reaches: the whole project, or its start phase's milestone.
const INVARIANT_SCOPE: &[&str] = &["project", "milestone"];
/// An invariant's status: `pending` until it is in force, then what its verdict says.
const INVARIANT_STATUS: &[&str] = &[
    "pending",
    "unverified",
    "stale",
    "holds",
    "violated",
    "uncertain",
];
/// The statuses of an invariant in force: every one but `pending`. While pending, its `proof` and `items` are not
/// checked and `gsd/I536` asks no verdict of it (`judged_while: complete`).
pub const INVARIANT_IN_FORCE: &[&str] = &["unverified", "stale", "holds", "violated", "uncertain"];
/// What a violation costs, highest first.
const CRITICALITY: &[&str] = &["critical", "high", "medium", "low"];

/// What must always hold in the code, from a phase on, and the tests that prove it. Unscoped: an invariant outlives
/// the phase it starts at, and its name is unique in the project.
pub const INVARIANT: Kind = Kind::new(
    "invariant",
    "An invariant: what must always hold, from which phase, proven by which tests, over which code items (beyond \
     GSD)",
    &[
        str("statement").prose().r().doc("what must always hold"),
        str("description").prose().doc("a summary beside the statement"),
        en("scope", INVARIANT_SCOPE)
            .doc("how far it reaches: the project (absent), or the milestone of its `holds_from` phase"),
        rf("holds_from", "phase")
            .doc("in force once this phase has been complete (executed or after) or verified, and for good; absent: \
                  from the start"),
        rfs("proof", "test").doc(
            "the tests that prove it: `@code/core` test facts, quoted (`\"js:pkg/a.test.js#adds\"`)",
        ),
        rfs("items", "item").doc(
            "the code items it holds over: `@code/core` item facts, quoted (`\"js:pkg/src/a.js::add\"`)",
        ),
        en("criticality", CRITICALITY).doc("what a violation costs"),
        strs("tags").doc("words to group it"),
        strs("refs").doc("external references: issue or document URIs"),
        en("status", INVARIANT_STATUS).c().doc(
            "pending until its holds_from phase has been complete or verified; then unverified (no usable verdict), \
             stale, or the fresh verdict's outcome",
        ),
        ts("last_verified")
            .c()
            .doc("the time of the fresh verdict that it holds; absent otherwise"),
    ],
)
.complete_when("status", INVARIANT_IN_FORCE);
