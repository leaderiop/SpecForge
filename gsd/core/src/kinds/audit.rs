//! The milestone audit (`spec/verify.md`): what `/gsd-audit-milestone` found. Its gaps, counts and status are
//! computed; only a person accepts its tech debt. `integration` is the integration checker's verdict (gsd/E518).
//!
//! gsd-core modules this file ports: audit.cjs.

use super::dsl::*;

const GAP_STATUS: &[&str] = &["unsatisfied", "partial", "orphaned"];
const AUDIT_STATUS: &[&str] = &["passed", "gaps_found", "tech_debt"];

const INTEGRATION: &[&str] = &["passed", "issues"];
const TECH_DEBT: &[Field] = &[rf("phase", "phase"), str("item").r()];
const DEBT_ACCEPTED: &[Field] = &[str("by").r(), date("at").r()];
const GAPS: &[Field] = &[rf("requirement", "requirement"), en("status", GAP_STATUS)];
const NYQUIST: &[Field] = &[
    int("compliant"),
    int("partial"),
    int("not_validated"),
    int("missing"),
];

/// `v<version>-MILESTONE-AUDIT.md`, named `A1` and scoped by its milestone.
pub const MILESTONE_AUDIT: Kind = Kind::new(
    "milestone_audit",
    "What an audit of a milestone found (GSD vX.Y-MILESTONE-AUDIT.md)",
    &[
        rf("milestone", "milestone").r().i().doc("the version"),
        blocks("tech_debt", TECH_DEBT).doc("`tech_debt:`"),
        en("integration", INTEGRATION)
            .v()
            .doc("the integration checker's cross-phase verdict (gsd/E518)"),
        block("debt_accepted", DEBT_ACCEPTED)
            .u()
            .doc("a person accepts the `tech_debt` status; refused in every plan and MCP write"),
        blocks("gaps", GAPS)
            .c()
            .doc("`gaps:`; the milestone's requirements that are not complete"),
        block("nyquist", NYQUIST).c().doc("`nyquist:`"),
        en("status", AUDIT_STATUS).c().doc("`status:`"),
    ],
)
.scoped_by("milestone");
