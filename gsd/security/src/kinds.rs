//! The kinds of `@gsd/security` (`spec/README.md`): a plan's trust boundaries and threats, a phase's security audit
//! with its accepted risks and runs, and the two fields it adds to core's `validation_row`. `threats_open`, a
//! threat's `blocking` and the audit's `status` are computed (`passes.rs`).

use gsd_kit::dsl::{self, *};
use specforge_extension_sdk::prelude::*;

pub const CATEGORY: &[&str] = &[
    "spoofing",
    "tampering",
    "repudiation",
    "information_disclosure",
    "denial_of_service",
    "elevation_of_privilege",
];
pub const SEVERITY: &[&str] = &["critical", "high", "medium", "low"];
pub const DISPOSITION: &[&str] = &["mitigate", "accept", "transfer"];
const THREAT_STATUS: &[&str] = &["open", "closed"];
const AUDIT_STATUS: &[&str] = &["draft", "verified"];

/// A row of the planner's `<threat_model>` Trust Boundaries table, named `TB1` and scoped by its plan.
pub const TRUST_BOUNDARY: Kind = Kind::new(
    "trust_boundary",
    "Where untrusted data crosses into the system (GSD <threat_model> Trust Boundaries)",
    &[
        rf("plan", "plan")
            .r()
            .i()
            .doc("the plan whose threat model it belongs to"),
        str("boundary").r().doc("`Boundary`"),
        str("description").r().doc("`Description`"),
        str("data_crossing").doc("`Data Crossing`"),
    ],
)
.scoped_by("plan");

/// A row of the STRIDE register, named `T-03-01` (`T-03-SC` for the supply-chain row every plan keeps) and scoped by
/// its plan. The cross-plan uniqueness of the ID is `gsd.security/E101`.
pub const THREAT: Kind = Kind::new(
    "threat",
    "A threat to a plan's component, with the disposition planned for it (GSD STRIDE register row)",
    &[
        rf("plan", "plan")
            .r()
            .i()
            .doc("the plan whose register it is in"),
        en("category", CATEGORY)
            .r()
            .doc("`Category`: the STRIDE letter spelled out"),
        str("component").r().doc("`Component`"),
        en("severity", SEVERITY).r().doc("`Severity`"),
        en("disposition", DISPOSITION).r().doc("`Disposition`"),
        str("mitigation")
            .r()
            .doc("`Mitigation` (the plan's `Mitigation Plan`)"),
        strs("files").doc("the files the auditor checks for the mitigation"),
        en("status", THREAT_STATUS)
            .h()
            .doc("`Status`: the auditor's verdict, written by record_security_audit"),
        str("evidence")
            .h()
            .doc("where the auditor found the mitigation"),
        bool("retroactive")
            .h()
            .doc("recorded by the audit of a phase planned without a register"),
        bool("blocking")
            .c()
            .doc("open, not accepted, and at or above workflow.security_block_on"),
    ],
)
.scoped_by("plan");

/// `NN-SECURITY.md`, named `03-SECURITY` and scoped by its phase.
pub const SECURITY_AUDIT: Kind = Kind::new(
    "security_audit",
    "A phase's security audit (GSD NN-SECURITY.md)",
    &[
        rf("phase", "phase").r().i().doc("`phase`"),
        int("asvs_level")
            .h()
            .range(1, 3)
            .doc("`asvs_level`: the level it ran at"),
        ts("created").h().doc("`created`"),
        int("threats_open")
            .c()
            .doc("the blocking threats of the phase's plans"),
        en("status", AUDIT_STATUS)
            .c()
            .doc("verified once it has a run and no threat blocks"),
        bool("register_planned")
            .c()
            .doc("a threat of the phase's plans was authored at plan time"),
    ],
)
.scoped_by("phase");

/// A line of the Accepted Risks Log, named `AR1` and scoped by its audit. A person's record: `accept_risk` is the
/// human-attested command that writes it. The kind is not `human` (plan 16 said it would be): the host refuses a human
/// command's plan to create a human-only entity ("write it in your editor"), so the command could not write it.
/// Instead `at` is a harness field only `accept_risk` writes, and only a risk with an `at` counts (`passes.rs`), so an
/// agent's `specforge new accepted_risk` is no acceptance. A kernel round could let a human command write a human kind.
pub const ACCEPTED_RISK: Kind = Kind::new(
    "accepted_risk",
    "A threat a person accepted (GSD Accepted Risks Log)",
    &[
        rf("audit", "security_audit").r().i().doc("parent"),
        rf("threat", "threat").r().i().doc("`Threat Ref`"),
        str("rationale").r().i().doc("`Rationale`"),
        str("accepted_by").r().i().doc("`Accepted By`"),
        date("at").h().i().doc(
            "`Date`: set by `accept_risk` with its ledger line; a risk without it is no acceptance",
        ),
    ],
)
.scoped_by("audit")
.append_only();

/// A line of the Security Audit Trail, named `AU1` and scoped by its audit.
pub const AUDIT_RUN: Kind = Kind::new(
    "audit_run",
    "One run of the security audit (GSD Security Audit Trail)",
    &[
        rf("audit", "security_audit").r().i().doc("parent"),
        ts("at").h().i().doc("`Audit Date`"),
        int("total").h().i().doc("`Threats Total`"),
        int("closed").h().i().doc("`Closed`"),
        int("open").h().i().doc("`Open`"),
        str("run_by").h().i().doc("`Run By`"),
    ],
)
.scoped_by("audit")
.append_only();

/// Every kind of the extension, in the order of the model.
pub const KINDS: &[&Kind] = &[
    &TRUST_BOUNDARY,
    &THREAT,
    &SECURITY_AUDIT,
    &ACCEPTED_RISK,
    &AUDIT_RUN,
];

/// `kind.field` of every computed field: what the compute pass answers.
pub fn computed() -> Vec<String> {
    dsl::computed(KINDS)
}

pub fn declare(c: &mut ContributionsBuilder) {
    dsl::declare(c, KINDS);
    // VALIDATION.md's Threat Ref and Secure Behavior columns are core's `validation_row`, enhanced here.
    c.edge("ValidationRowThreat", |e| {
        e.source_kind("validation_row").target_kind("threat");
    });
    c.enhance("validation_row", |e| {
        e.field("threat", |f| {
            f.field_type(FieldType::Reference)
                .target_kind("threat")
                .edge("ValidationRowThreat")
                .description("`Threat Ref`: the threat the row's test secures");
        })
        .field("secure_behavior", |f| {
            f.field_type(FieldType::String)
                .description("`Secure Behavior`: what the test shows");
        });
    });
}
