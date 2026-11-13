//! `retrospective`, `lesson` and `methodology` (`spec/project.md`).
//!
//! Also ports gsd-core bin/lib modules: learnings.cjs.

use gsd_kit::dsl::*;

const COST: &[Field] = &[str("model_mix"), int("sessions"), str("notable")];

/// One `## Milestone:` section of `RETROSPECTIVE.md`, named `v1.0-RETROSPECTIVE`.
pub const RETROSPECTIVE: Kind = Kind::new(
    "retrospective",
    "A milestone's retrospective (GSD RETROSPECTIVE.md)",
    &[
        rf("milestone", "milestone").r().i(),
        strs("built"),
        strs("worked"),
        strs("inefficient"),
        strs("patterns"),
        block("cost", COST),
        int("phases").c(),
        int("plans").c(),
        int("sessions").c(),
    ],
)
.scoped_by("milestone");

const LESSON_KINDS: &[&str] = &["decision", "lesson", "pattern", "surprise"];
const GRADUATION: &[&str] = &["promoted", "deferred", "dismissed"];
const GRADUATED: &[Field] = &[
    en("disposition", GRADUATION).r(),
    rf("target", "decision"),
    rf("until", "phase"),
    ts("at").r(),
];
const LESSON_STATUS: &[&str] = &["open", "deferred", "promoted", "dismissed"];

/// A lesson, named `L1` and scoped by its retrospective; what /gsd-extract-learnings writes has none.
pub const LESSON: Kind = Kind::new(
    "lesson",
    "A lesson learned (GSD RETROSPECTIVE.md, LEARNINGS.md)",
    &[
        rf("retrospective", "retrospective")
            .inverse("lessons")
            .doc("the retrospective that scopes it"),
        str("text").r(),
        rf("milestone", "milestone"),
        bool("top"),
        en("kind", LESSON_KINDS).doc("what /gsd-extract-learnings classified it as"),
        block("graduated", GRADUATED).h().doc(
            "`**Graduated:**` and `graduation_backlog`: promoted into a decision (`target`), deferred until a phase (`until`) or dismissed, written by `graduate_lessons`",
        ),
        en("status", LESSON_STATUS)
            .c()
            .doc("open unless graduated; a deferral is open again once its phase is complete"),
    ],
)
.scoped_by("retrospective");

const METHODOLOGY_STATUS: &[&str] = &["active", "superseded"];

/// `METHODOLOGY.md`.
pub const METHODOLOGY: Kind = Kind::new(
    "methodology",
    "A recorded methodology (GSD METHODOLOGY.md)",
    &[
        str("name").r(),
        str("diagnoses"),
        str("recommends"),
        str("apply_when"),
        en("status", METHODOLOGY_STATUS),
        rf("superseded_by", "methodology"),
    ],
);
