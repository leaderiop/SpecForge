//! `truth`, `artifact`, `key_link` and `prohibition`: a plan's `must_haves:` block, one entity per item so a
//! verifier's verdict can name it (`spec/plan.md`). GSD has no "interfaces" must-have: `<interfaces>` is a block of
//! `<context>`. Paths are `str`, not `files`: an artifact or a link end is a file the plan creates.

use super::dsl::*;

const TRUTH_VERIFICATION: &[&str] = &["explicit", "backstop"];

/// `must_haves.truths`, named `TR1` within its plan.
pub const TRUTH: Kind = Kind::new(
    "truth",
    "A user-observable truth the plan makes true (GSD must_haves.truths)",
    &[
        rf("plan", "plan").r().i().doc("parent"),
        str("text").r().doc("the truth, as a user observes it"),
        en("verification", TRUTH_VERIFICATION)
            .doc("explicit: a named passing test; backstop: judged"),
        int("criterion").doc("the phase success criterion it proves"),
    ],
)
.scoped_by("plan");

/// `must_haves.artifacts`, named `AR1` within its plan.
pub const ARTIFACT: Kind = Kind::new(
    "artifact",
    "A file the plan provides (GSD must_haves.artifacts)",
    &[
        rf("plan", "plan").r().i().doc("parent"),
        str("path")
            .r()
            .doc("`path:` (from the project root; not existence-checked)"),
        str("provides").r().doc("`provides:`"),
        int("min_lines").doc("`min_lines:`"),
        strs("exports").doc("`exports:`"),
        str("contains").doc("`contains:`"),
    ],
)
.scoped_by("plan");

/// `must_haves.key_links`, named `KL1` within its plan.
pub const KEY_LINK: Kind = Kind::new(
    "key_link",
    "A wiring between two files the plan creates (GSD must_haves.key_links)",
    &[
        rf("plan", "plan").r().i().doc("parent"),
        str("from")
            .r()
            .doc("`from:` (a path, not existence-checked)"),
        str("to").r().doc("`to:` (a path, not existence-checked)"),
        str("via").r().doc("`via:`"),
        str("pattern").doc("`pattern:`"),
    ],
)
.scoped_by("plan");

const PROHIBITION_VERIFICATION: &[&str] = &["test", "judgment"];

/// `must_haves.prohibitions`, named `PR1` within its plan.
pub const PROHIBITION: Kind = Kind::new(
    "prohibition",
    "Something the plan must not do (GSD must_haves.prohibitions)",
    &[
        rf("plan", "plan").r().i().doc("parent"),
        str("statement").r().doc("`statement:`"),
        en("verification", PROHIBITION_VERIFICATION)
            .r()
            .doc("`verification:`"),
        str("test").doc("the test it names when `verification test` (gsd/E319)"),
    ],
)
.scoped_by("plan");
