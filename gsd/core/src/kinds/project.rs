//! `project` and `requirement` (`spec/project.md`).

use super::dsl::*;

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

/// Singleton. GSD: `PROJECT.md`. `key_decisions` joins with the `decision` kind (round r36).
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

/// One bullet of `REQUIREMENTS.md`, named `AUTH-01`.
pub const REQUIREMENT: Kind = Kind::new(
    "requirement",
    "A requirement, named by its GSD id such as AUTH-01 (GSD REQUIREMENTS.md)",
    &[
        str("text").r().doc("the description after the ID"),
        str("category").r().doc("### [Category]"),
        en("tier", TIERS).r().doc("v1, v2 or Out of Scope"),
        str("reason").doc("Out of Scope reason: required when out_of_scope (gsd/E105)"),
        date("defined").i().doc("Defined"),
        en("status", REQUIREMENT_STATUS)
            .c()
            .doc("Traceability status, from plans, summaries and verifications"),
    ],
);
