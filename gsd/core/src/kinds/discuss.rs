//! `context`, `decision`, `deferred_idea` and `discussion` (`spec/discuss.md`).

use super::dsl::*;

const CANONICAL_REF: &[Field] = &[str("topic"), files("paths")];
const CONTEXT_STATUS: &[&str] = &["gathering", "ready_for_planning"];

/// `NN-CONTEXT.md`, named `02-CONTEXT` and scoped by its phase.
pub const CONTEXT: Kind = Kind::new(
    "context",
    "A phase's context: its boundary, canonical references and discretion (GSD NN-CONTEXT.md)",
    &[
        rf("phase", "phase").r().i().doc("file location"),
        date("gathered").i().doc("**Gathered:**"),
        str("boundary").r().doc("<domain> Phase Boundary"),
        strs("discretion").doc("Claude's Discretion"),
        str("specifics").doc("<specifics>"),
        // Not required: `gsd/E201` is the one diagnostic of its absence.
        blocks("canonical_refs", CANONICAL_REF).doc("<canonical_refs>"),
        strs("reusable_assets").doc("<code_context>"),
        strs("patterns").doc("<code_context>"),
        strs("integration_points").doc("<code_context>"),
        en("status", CONTEXT_STATUS)
            .c()
            .doc("**Status:** ready once canonical_refs and boundary are present and no interview is open"),
    ],
)
.scoped_by("phase");

const OUTCOME: &[&str] = &["good", "revisit", "pending"];
const REVERSIBILITY: &[&str] = &["reversible", "costly", "one_way"];
const ORIGIN: &[&str] = &["context", "project", "summary", "checkpoint"];

/// `- **D-NN:**` of a CONTEXT, a Key Decisions row; scoped by its phase, or by the project when it has none.
// `from_summary → summary` is declared with `summary` (round r38): a reference to a kind no loaded extension declares
// is W021.
pub const DECISION: Kind = Kind::new(
    "decision",
    "A decision, named D-01 within its phase or the project (GSD CONTEXT.md, PROJECT.md Key Decisions)",
    &[
        rf("phase", "phase")
            .i()
            .doc("owning CONTEXT; absent for a project decision"),
        str("area").doc("### [Area]"),
        str("text").r().doc("the decision"),
        str("rationale").doc("Key Decisions Rationale"),
        en("outcome", OUTCOME).doc("Good, Revisit or Pending"),
        en("reversibility", REVERSIBILITY).doc("planner reversibility rating"),
        bool("settled").doc("locked: a plan contradicting it is refused (judged)"),
        en("origin", ORIGIN).r().i().doc("where it was recorded"),
        rf("superseded_by", "decision").doc("a later decision that reverses it"),
    ],
)
.scoped_by("phase");

/// `<deferred>`: excluded from every coverage rule. Named `DI1`.
pub const DEFERRED_IDEA: Kind = Kind::new(
    "deferred_idea",
    "An idea deferred out of a phase (GSD CONTEXT.md <deferred>)",
    &[str("text").r(), rf("phase", "phase").r(), str("why")],
)
.scoped_by("phase");

const DISCUSSION_MODES: &[&str] = &["interactive", "assumptions", "advisor"];
const ENTRY: &[Field] = &[
    str("area"),
    str("question"),
    strs("options"),
    str("chosen"),
    en("mode", DISCUSSION_MODES),
];

/// `NN-DISCUSSION-LOG.md`, named `02-DISCUSSION-LOG`: an audit record no agent context is built from.
pub const DISCUSSION: Kind = Kind::new(
    "discussion",
    "A phase's discussion log: the options weighed and the choices made (GSD NN-DISCUSSION-LOG.md)",
    &[
        rf("phase", "phase").r().i().doc("file location"),
        blocks("entries", ENTRY)
            .append()
            .doc("one entry per question asked"),
    ],
)
.scoped_by("phase");
