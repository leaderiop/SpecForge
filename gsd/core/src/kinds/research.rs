//! `research` and its eight item kinds (`spec/discuss.md`). Each item is scoped by its research and named `F1`, `SC1`,
//! `P1`, `PF1`, `DH1`, `OQ1`, `SRC1`, `PA1`.
//!
//! Also ports gsd-core bin/lib modules: research-store.cjs.

use gsd_kit::dsl::*;

const CONFIDENCE: &[&str] = &["high", "medium", "low"];
const RESPONSIBILITY: &[Field] = &[str("capability"), str("tier")];

/// `NN-RESEARCH.md`, named `02-RESEARCH`; the project's research (`research/*.md`) is named `RESEARCH` and has no phase;
/// a quick plan's (`<quick_id>-RESEARCH.md`) names its quick plan in `plan` instead.
pub const RESEARCH: Kind = Kind::new(
    "research",
    "Research for a phase, a quick plan or the project (GSD NN-RESEARCH.md, <quick_id>-RESEARCH.md, research/*.md)",
    &[
        rf("phase", "phase")
            .i()
            .doc("absent for the project's research and a quick plan's"),
        rf("plan", "plan")
            .i()
            .doc("the quick plan of a quick task's research (/gsd-quick --research)"),
        str("context_digest")
            .h()
            .doc("the digest of the phase's (or quick plan's) context when the research was recorded (gsd/W406)"),
        str("domain"),
        date("researched").i(),
        en("confidence", CONFIDENCE).r(),
        str("summary").r(),
        rfs("user_constraints", "decision"),
        blocks("responsibility_map", RESPONSIBILITY),
    ],
)
.scoped_by("phase");

const FINDING_TOPICS: &[&str] = &["stack", "features", "architecture", "pitfalls", "other"];

pub const FINDING: Kind = Kind::new(
    "finding",
    "A finding of a research (GSD ## Summary, project research files)",
    &[
        rf("research", "research").r(),
        en("topic", FINDING_TOPICS),
        str("text").r(),
        en("confidence", CONFIDENCE).r(),
        rfs("sources", "source"),
    ],
)
.scoped_by("research");

const TIERS: &[&str] = &["core", "supporting", "alternative"];

pub const STACK_CHOICE: Kind = Kind::new(
    "stack_choice",
    "A technology a research recommends (GSD ## Standard Stack)",
    &[
        rf("research", "research").r(),
        str("name").r(),
        str("version"),
        str("purpose").r(),
        en("tier", TIERS),
        str("rejected_because"),
    ],
)
.scoped_by("research");

pub const PATTERN: Kind = Kind::new(
    "pattern",
    "An architecture pattern, or an anti-pattern, of a research (GSD ## Architecture Patterns)",
    &[
        rf("research", "research").r(),
        str("name").r(),
        str("what").r(),
        str("when"),
        str("example"),
        bool("anti").doc("### Anti-Patterns"),
    ],
)
.scoped_by("research");

pub const PITFALL: Kind = Kind::new(
    "pitfall",
    "A common pitfall of a research (GSD ## Common Pitfalls)",
    &[
        rf("research", "research").r(),
        str("name").r(),
        str("goes_wrong").r(),
        str("why").r(),
        str("avoid").r(),
        str("warning_signs"),
    ],
)
.scoped_by("research");

pub const DONT_HAND_ROLL: Kind = Kind::new(
    "dont_hand_roll",
    "A problem a research says to solve with an existing library (GSD ## Don't Hand-Roll)",
    &[
        rf("research", "research").r(),
        str("problem").r(),
        str("instead").r(),
        str("why").r(),
    ],
)
.scoped_by("research");

/// Without `resolved_by` a phase research's question is `gsd/W204`.
pub const OPEN_QUESTION: Kind = Kind::new(
    "open_question",
    "An open question of a research (GSD ## Open Questions)",
    &[
        rf("research", "research").r(),
        str("question").r(),
        str("known"),
        str("unclear").r(),
        str("recommendation"),
        rf("resolved_by", "decision"),
    ],
)
.scoped_by("research");

pub const SOURCE: Kind = Kind::new(
    "source",
    "A source a research cites (GSD ## Sources)",
    &[
        rf("research", "research").r(),
        str("url").r(),
        en("tier", &["primary", "secondary", "tertiary"]).r(),
        str("checked"),
    ],
)
.scoped_by("research");

const LEGITIMACY: &[&str] = &["legitimate", "suspicious", "unknown"];

/// `legitimacy` is a verdict (`V`): the phase researcher's outcome for `gsd/W205`, never written by hand (E082).
pub const PACKAGE_AUDIT: Kind = Kind::new(
    "package_audit",
    "A package a research audited for legitimacy (GSD package-legitimacy table)",
    &[
        rf("research", "research").r(),
        str("package").r(),
        str("registry").r(),
        en("legitimacy", LEGITIMACY)
            .v()
            .doc("the phase researcher's verdict (gsd/W205)"),
    ],
)
.scoped_by("research");
