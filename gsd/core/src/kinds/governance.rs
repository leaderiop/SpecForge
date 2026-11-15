//! `risk` and `invariant` (`spec/governance.md`): beyond GSD.

use gsd_kit::dsl::*;

/// The ratings once every mitigation is done; a member left out keeps its pre rating.
const POST: &[Field] = &[
    int("severity").range(1, 10).doc("severity once mitigated"),
    int("occurrence")
        .range(1, 10)
        .doc("occurrence once mitigated"),
    int("detection")
        .range(1, 10)
        .doc("detection once mitigated"),
];

/// What can control a risk: a task, done; a truth, verified.
const MITIGATIONS: &[&str] = &["task", "truth"];
const RISK_STATUS: &[&str] = &["open", "mitigating", "mitigated"];

/// A risk of the project. Unscoped: a risk outlives a phase, and moving it to another milestone does not rename it.
pub const RISK: Kind = Kind::new(
    "risk",
    "A risk: what can go wrong, rated 1 to 10 for severity, occurrence and detection (FMEA), and what controls it \
     (beyond GSD)",
    &[
        str("cause").r().doc("what makes it happen"),
        str("effect").r().doc("what breaks when it does"),
        int("severity")
            .r()
            .range(1, 10)
            .doc("1 no effect … 10 hazardous, without warning"),
        int("occurrence")
            .r()
            .range(1, 10)
            .doc("1 remote … 10 almost certain"),
        int("detection")
            .r()
            .range(1, 10)
            .doc("1 almost certainly caught before harm … 10 cannot be caught"),
        block("post", POST).doc(
            "the ratings once every mitigation is done; a member left out keeps its pre rating",
        ),
        rfs_any("mitigated_by", MITIGATIONS)
            .doc("what controls it: a task done, a truth verified"),
        str("mitigation").doc("the planned control as prose; no evidence"),
        rfs("affects", "requirement")
            .inverse("risks")
            .doc("the requirements it threatens; each reads it back as `risks`"),
        rf("milestone", "milestone").inverse("risks").doc(
            "where it must be settled when no phase's plan mitigates it; the milestone reads it back as `risks`",
        ),
        strs("tags").doc("words to group it"),
        str("description").doc("free prose"),
        strs("refs").doc("external references"),
        int("rpn")
            .c()
            .doc("severity × occurrence × detection (1 to 1000)"),
        int("post_rpn")
            .c()
            .doc("the post ratings' product; the rpn without a post block"),
        en("status", RISK_STATUS).c().doc(
            "open while it names no mitigation, mitigated once every one it names is done, else mitigating",
        ),
        bool("blocking").c().doc(
            "its current rpn (post_rpn once mitigated, else rpn) is at or above specforge.risk_block_on; without a \
             post block post_rpn is the rpn, so a mitigated risk stops blocking once post ratings bring it under",
        ),
    ],
);

/// How far an invariant reaches: the whole project, or its start phase's milestone.
const INVARIANT_SCOPE: &[&str] = &["project", "milestone"];
/// What a violation costs, highest first.
const CRITICALITY: &[&str] = &["critical", "high", "medium", "low"];

/// What must always hold in the code, from a phase on, and the tests that prove it. Unscoped: an invariant outlives
/// the phase it starts at, and its name is unique in the project.
pub const INVARIANT: Kind = Kind::new(
    "invariant",
    "An invariant: what must always hold, from which phase, proven by which tests, over which code items (beyond \
     GSD)",
    &[
        str("statement").r().doc("what must always hold"),
        str("description").doc("a summary beside the statement"),
        en("scope", INVARIANT_SCOPE)
            .doc("how far it reaches: the project (absent), or the milestone of its `holds_from` phase"),
        rf("holds_from", "phase").doc("the phase it holds from; absent: from the start"),
        rfs("proof", "test").doc(
            "the tests that prove it: `@code/core` test facts, quoted (`\"js:pkg/a.test.js#adds\"`)",
        ),
        rfs("items", "item").doc(
            "the code items it holds over: `@code/core` item facts, quoted (`\"js:pkg/src/a.js::add\"`)",
        ),
        en("criticality", CRITICALITY).doc("what a violation costs"),
        strs("tags").doc("words to group it"),
        strs("refs").doc("external references: issue or document URIs"),
    ],
);
