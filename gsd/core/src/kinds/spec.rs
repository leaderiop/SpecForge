//! `phase_spec`, `locked_req` and `interview` (`spec/discuss.md`).

use super::dsl::*;

/// The `spec_phase` prompt's verdict, in percent. Round r40 makes `scores` a verdict field (`V`) when it declares the
/// judged rule `gsd/E202` that binds it: a verdict field no judged rule lists is the kernel's W021 on every check, so
/// until then it is authored.
const SCORES: &[Field] = &[
    int("goal").range(0, 100),
    int("boundary").range(0, 100),
    int("constraint").range(0, 100),
    int("acceptance").range(0, 100),
    int("ambiguity").range(0, 100),
];

/// `NN-SPEC.md`, written by `/gsd-spec-phase`; named `02-SPEC`.
pub const PHASE_SPEC: Kind = Kind::new(
    "phase_spec",
    "A phase's spec: what it delivers, falsifiably (GSD NN-SPEC.md)",
    &[
        rf("phase", "phase").r().i().doc("file location"),
        date("created").i().doc("**Created:**"),
        str("goal").r().doc("X changes from A to B"),
        str("background"),
        strs("in_scope"),
        strs("out_of_scope"),
        strs("constraints"),
        strs("acceptance"),
        block("scores", SCORES).doc("the spec_phase prompt's verdict, in percent (goal >= 75, boundary >= 70, constraint >= 65, acceptance >= 70, ambiguity <= 20)"),
    ],
)
.scoped_by("phase");

/// A numbered requirement of a SPEC, named `LR1`.
pub const LOCKED_REQ: Kind = Kind::new(
    "locked_req",
    "A locked requirement of a phase spec (GSD NN-SPEC.md)",
    &[
        rf("spec", "phase_spec").r(),
        str("title").r(),
        str("current").r(),
        str("target").r(),
        str("acceptance").r().doc("falsifiable"),
        rfs("traces", "requirement"),
    ],
)
.scoped_by("spec");

const PERSPECTIVES: &[&str] = &[
    "researcher",
    "simplifier",
    "boundary_keeper",
    "failure_analyst",
];

/// One `## Interview Log` row, named `IV1`. A log: removing a row is `E085`, rewriting a field `E084`; `answer` and
/// `locked` may be set once.
pub const INTERVIEW: Kind = Kind::new(
    "interview",
    "One round of a phase spec's interview (GSD NN-SPEC.md Interview Log)",
    &[
        rf("spec", "phase_spec").r().i(),
        int("round").r().i(),
        en("perspective", PERSPECTIVES).r().i(),
        str("question").r().i(),
        str("answer").i(),
        str("locked").i(),
    ],
)
.scoped_by("spec")
.append_only();
