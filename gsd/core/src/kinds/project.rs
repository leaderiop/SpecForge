//! `project` and `requirement` (`spec/project.md`).

use gsd_kit::dsl::*;

const CONSTRAINT_KINDS: &[&str] = &[
    "tech_stack",
    "timeline",
    "budget",
    "dependencies",
    "compatibility",
    "performance",
    "security",
    "other",
];
const CONSTRAINT: &[Field] = &[en("kind", CONSTRAINT_KINDS), str("what"), str("why")];
const OUT_OF_SCOPE: &[Field] = &[str("item"), str("why").r()];

/// Singleton. GSD: `PROJECT.md`.
pub const PROJECT: Kind = Kind::new(
    "project",
    "The project: what it is, its core value, context and constraints (GSD PROJECT.md)",
    &[
        str("name").r().doc("# [Project Name]"),
        str("what").r().doc("What This Is"),
        str("core_value").r().doc("Core Value"),
        str("customer").doc("Business Context"),
        str("revenue_model").doc("Business Context"),
        str("success_metric").doc("Business Context"),
        str("context").r().doc("Context"),
        blocks("constraints", CONSTRAINT).r().doc("Constraints"),
        blocks("out_of_scope", OUT_OF_SCOPE).doc("Out of Scope"),
        rfs("key_decisions", "decision").doc("Key Decisions"),
        str("project_code").doc("config project_code: ^[A-Z][A-Z0-9_]*$"),
        rfs("validated", "requirement")
            .c()
            .doc("Validated: from requirement status"),
        rfs("active", "requirement")
            .c()
            .doc("Active: from requirement status"),
        ts("updated")
            .c()
            .doc("Last updated: the latest recorded commit or summary completion"),
        int("windows_open").c().doc("broken windows still open"),
        int("windows_waived").c().doc("broken windows waived"),
        int("windows_fixed").c().doc("broken windows fixed"),
        int("windows_total").c().doc("broken windows recorded"),
    ],
)
.singleton();

const TIERS: &[&str] = &["v1", "v2", "out_of_scope"];
const REQUIREMENT_STATUS: &[&str] = &[
    "pending",
    "in_progress",
    "complete",
    "blocked",
    "gaps_found",
];
/// A requirement's quality class: ISO/IEC 25010:2023's nine product quality characteristics by their short names
/// (interoperability is compatibility), and `regulatory`.
pub const REQUIREMENT_CATEGORY: &[&str] = &[
    "functional",
    "performance",
    "compatibility",
    "usability",
    "reliability",
    "security",
    "maintainability",
    "portability",
    "safety",
    "regulatory",
];
/// The sense a metric's threshold holds in.
pub const DIRECTION: &[&str] = &["at_most", "at_least", "exactly"];
/// A measure: no bound on the threshold (a latency is positive, a drift may be negative: `direction` carries the
/// sense), and no unit for a plain count.
pub const METRIC: &[Field] = &[
    str("name").r().doc("what is measured"),
    num("threshold").r().doc("the value it is held to"),
    str("unit").doc("its unit (ms, s, %); none for a count"),
    en("direction", DIRECTION)
        .r()
        .doc("at_most, at_least or exactly the threshold"),
];
/// How far a requirement's thread reaches (`passes::thread::Proof`, in its order).
pub const PROOF_STATES: &[&str] = &[
    "no_task",
    "no_commit",
    "not_run",
    "no_test",
    "failed",
    "proven",
];

/// One bullet of `REQUIREMENTS.md`, named `AUTH-01`, and the fields of the product feature and the governance
/// constraint it absorbs (quality category, priority, metric, sources, relations, tags).
pub const REQUIREMENT: Kind = Kind::new(
    "requirement",
    "A requirement, named by its GSD id such as AUTH-01 (GSD REQUIREMENTS.md)",
    &[
        str("text").r().doc("the description after the ID"),
        str("area").r().doc("### [Category]"),
        en("tier", TIERS).r().doc("v1, v2 or Out of Scope"),
        str("reason").doc("Out of Scope reason: required when out_of_scope (gsd/E105)"),
        date("defined").i().doc("Defined"),
        en("category", REQUIREMENT_CATEGORY)
            .doc("its quality characteristic (ISO/IEC 25010:2023) or regulatory"),
        en("priority", super::product::PRIORITY)
            .doc("must, should or could (won't is `tier out_of_scope`)"),
        block("metric", METRIC).doc(
            "the measure it is held to: `name`, a `threshold` number, its `unit` when it has one, a `direction`",
        ),
        strs("source").doc(
            "where it came from: a person, quoted, or a declared `ref` (`gh.issue:42`), bare",
        ),
        rfs("personas", "persona")
            .inverse("requirements")
            .doc("the personas it serves; a persona reads them back as `requirements`"),
        rfs("constrains", "requirement")
            .inverse("constrained_by")
            .doc("the requirements it limits; each reads it back as `constrained_by`"),
        rfs("protects", "invariant")
            .inverse("protected_by")
            .doc("the invariants it guards; each reads it back as `protected_by`"),
        strs("tags").doc("words to group it by"),
        en("status", REQUIREMENT_STATUS)
            .c()
            .doc("Traceability status, from plans, summaries and verifications"),
        rfs("satisfied_by", "item").c().doc(
            "the code items the tasks of its plans name, held or not: sorted, each once; absent when none",
        ),
        en("proof", PROOF_STATES).c().doc(
            "how far its thread reaches: a task of its plans, a commit of one, a passing test; `proven` when whole",
        ),
        ens("shapes", super::spec::SHAPES)
            .doc("upstream's authored shapes override; an empty list proposes no edge"),
        str("text_en")
            .doc("an English translation the edge probe classifies instead (response_language)"),
    ],
);
