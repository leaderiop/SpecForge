//! `phase_spec`, `locked_req`, `interview`, `edge` and `spec_prohibition` (`spec/discuss.md`).

use gsd_kit::dsl::*;

use crate::probe::edge::EDGE_VALIDATORS;
use crate::probe::{PROHIBITION_VALIDATORS, VALID_STATUS};

/// The `spec_phase` prompt's verdict, in percent: `scores` is a verdict field (`V`) bound by the judged rule `gsd/E202`.
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
        context_record("context_pack")
            .doc("the gsd/phase-context pack it was written against (gsd/W406)"),
        date("created").i().doc("**Created:**"),
        str("goal").r().doc("X changes from A to B"),
        str("background"),
        strs("in_scope"),
        strs("out_of_scope"),
        strs("constraints"),
        strs("acceptance"),
        block("scores", SCORES).v().doc("the spec_phase prompt's verdict, in percent (goal >= 75, boundary >= 70, constraint >= 65, acceptance >= 70, ambiguity <= 20)"),
    ],
)
.scoped_by("phase");

/// The shapes an edge probe classifies a requirement into: upstream's names with `_` for `-`.
pub const SHAPES: &[&str] = &["numeric_range", "collection", "text", "stateful", "io"];

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
        ens("shapes", SHAPES)
            .doc("upstream's authored shapes override; an empty list proposes no edge"),
        str("text_en")
            .doc("an English translation the edge probe classifies instead (response_language)"),
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

/// One `## Edge Coverage` row, named `EC1`: a requirement's edge category and its resolution, in the probe engine's
/// vocabularies. gsd/E523 holds the lifecycle; `specforge gsd resolve-edge` writes it in one edit.
pub const EDGE: Kind = Kind::new(
    "edge",
    "One edge-coverage row of a phase spec (GSD NN-SPEC.md ## Edge Coverage)",
    &[
        rf("spec", "phase_spec").r().i(),
        rf("requirement", "locked_req").r(),
        en("category", EDGE_VALIDATORS.categories).r(),
        en("status", &VALID_STATUS).r(),
        en("verification", EDGE_VALIDATORS.verification),
        str("resolution").doc("the acceptance criterion or backstop test note"),
        str("reason").doc("why it is dismissed"),
    ],
)
.scoped_by("spec");

/// One `## Prohibitions` row, named `SP1`: a must-NOT statement of a requirement and its resolution, with the check
/// descriptor a resolved `test` row may carry. gsd/E523 holds the lifecycle; `specforge gsd resolve-prohibition` writes
/// it in one edit.
pub const SPEC_PROHIBITION: Kind = Kind::new(
    "spec_prohibition",
    "One must-NOT row of a phase spec (GSD NN-SPEC.md ## Prohibitions)",
    &[
        rf("spec", "phase_spec").r().i(),
        rf("requirement", "locked_req").r(),
        str("statement").r(),
        en("status", &VALID_STATUS).r(),
        en("verification", PROHIBITION_VALIDATORS.verification),
        str("reason"),
        en("check_kind", &["node_test", "lint_rule"]),
        str("check_target"),
        str("check_rule"),
        str("check_violation_fixture"),
        str("check_clean_fixture"),
    ],
)
.scoped_by("spec");
