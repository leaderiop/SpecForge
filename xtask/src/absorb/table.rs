//! The absorb table (ADR 0360 D2): one row per field of an old `@specforge/governance` or `@specforge/product` kind,
//! as the old builtins' registry holds it, and one per old kind that takes `verify`. Each row names its homes in
//! `@gsd/core`; every home of a row is applied to the row's value. The table is the measure: a test counts it.
//!
//! `@gsd/core` holds exactly gsd-core's kinds and fields (ADR 0370): a value whose home left it, or whose old entity
//! converts to nothing (an invariant, a failure mode, a journey, a persona, a term, a channel, a decision that no
//! longer holds, a milestone no converted release lists), is `dropped` with its reason. A value with a home in GSD's
//! kinds that absorb does not write yet waits for the conversion change.
//!
//! A release becomes a version milestone and each milestone it lists a phase under it (ADR 0372 D4). No home writes
//! a reference except the phase's three: its `milestone`, its `requirements` and its `depends_on`, each through the
//! id map ([`super::ids`]).

use serde::Serialize;

/// What a Later value waits for: a kind `@gsd/core` does not hold yet, never a date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Pending {
    Package,
    Truth,
    Behavior,
}

/// Where an old value goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Home {
    /// Written to the authored field `field` of a `kind` entity, through `how`.
    To {
        kind: &'static str,
        field: &'static str,
        how: How,
    },
    /// Computed by the new kind (`field` is `writer: computed`): compared with the old value after the write.
    Computed {
        kind: &'static str,
        field: &'static str,
        compare: Compare,
    },
    /// Waits for a kind not absorbed yet: ledgered.
    Later(Pending),
    /// No tracked project writes it and nothing holds it: a value stops the run (ADR 0357's refused class).
    Refused,
    /// No `@gsd/core` field takes the value: it is ledgered `dropped` with this reason and nothing is written.
    Dropped(&'static str),
}

/// How a value is written to its `To` home. Only the phase's three ways write a reference (ADR 0372 D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    /// As written: a text, a date, a number, a list of texts (appended to what the field holds; a scheme
    /// reference in it is ledgered, as its `ref` cannot be written).
    As,
    /// A text joined after what the field holds, a blank line between.
    Text,
    /// The entity's id, which the id map assigns from the value (a release's version): it writes no field, and its
    /// field is [`ID`].
    Id,
    /// Each listed milestone's phase placed under this entity's milestone: its scope field set to this new id. A
    /// listed milestone the run writes no phase for is ledgered `dropped`.
    Parent,
    /// Each listed old id as the requirement it becomes, through the id map; one that becomes none is ledgered
    /// `dropped` with why.
    Requirements,
    /// Each listed old milestone as the path of its phase, kept only when that phase is an earlier one of the same
    /// version milestone (GSD resolves a phase's `depends_on` within its milestone); any other is ledgered `dropped`.
    Earlier,
    /// A decision `<id>_solution` of the project created beside the entity, with the same title and file, the value
    /// as its text.
    Solution,
    /// A feature status as a tier: proposed and deferred v2, deprecated out_of_scope (with a reason), else v1.
    FeatureTier,
    /// A decision status as `outcome`: proposed → pending, deprecated → revisit. A superseded decision never reaches
    /// it: it converts to nothing ([`converts_to_nothing`]).
    Outcome,
    /// A decision status as `settled`: accepted → true.
    Settled,
}

/// How a computed value is compared with the old one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compare {
    /// An old word and the computed word it means; a word not listed has no equal, so it is always ledgered.
    Means(&'static [(&'static str, &'static str)]),
}

/// One old field and its homes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub kind: &'static str,
    pub field: &'static str,
    pub homes: &'static [Home],
}

/// The field name of an obligation row.
pub const VERIFY: &str = "verify";

/// The field of a [`How::Id`] home: the entity's id, which no field holds.
pub const ID: &str = "(id)";

/// The old kinds the table absorbs: governance's four, then product's nine.
pub const OLD_KINDS: [&str; 13] = [
    "decision",
    "constraint",
    "failure_mode",
    "invariant",
    "feature",
    "journey",
    "deliverable",
    "milestone",
    "module",
    "term",
    "persona",
    "channel",
    "release",
];

const DECISION: &str = "decision";
const REQUIREMENT: &str = "requirement";
const MILESTONE: &str = "milestone";
const PHASE: &str = "phase";

/// A feature status as the computed requirement status it means.
const FEATURE_STATUS: Compare = Compare::Means(&[
    ("done", "complete"),
    ("in_progress", "in_progress"),
    ("proposed", "pending"),
    ("accepted", "pending"),
    ("deferred", "pending"),
]);
/// A release status as the computed milestone state; recalled has no equal.
const RELEASE_STATE: Compare = Compare::Means(&[
    ("planned", "planned"),
    ("in_progress", "in_progress"),
    ("released", "shipped"),
]);
/// An old milestone status as the computed phase status; blocked has no equal.
const PHASE_STATUS: Compare = Compare::Means(&[
    ("planned", "not_started"),
    ("in_progress", "in_progress"),
    ("completed", "complete"),
]);
/// A deliverable status as the computed milestone state.
const DELIVERABLE_STATE: Compare = Compare::Means(&[
    ("draft", "planned"),
    ("in_progress", "in_progress"),
    ("shipped", "shipped"),
    ("deprecated", "abandoned"),
]);

const fn to(kind: &'static str, field: &'static str, how: How) -> Home {
    Home::To { kind, field, how }
}

const fn computed(kind: &'static str, field: &'static str, compare: Compare) -> Home {
    Home::Computed {
        kind,
        field,
        compare,
    }
}

const fn later(pending: Pending) -> Home {
    Home::Later(pending)
}

const REFUSED: Home = Home::Refused;

/// Why an old failure mode's values are dropped (ADR 0370 D2 and D8): gsd-core has no risk kind, so an old failure
/// mode converts to nothing. A threat belongs to `@gsd/security`'s register, and needs a plan to name it.
const NO_RISK: Home = Home::Dropped(
    "gsd-core has no risk kind (ADR 0370): an old failure mode converts to nothing; a threat belongs to \
     @gsd/security's register and needs a plan",
);
/// Why an old invariant's values are dropped (ADR 0370 D2 and D8): the conversion change writes one as a requirement.
const NO_INVARIANT: Home = Home::Dropped(
    "gsd-core has no invariant kind (ADR 0370): the conversion change writes an old invariant as a requirement",
);
/// Why an old journey's values are dropped (ADR 0370 D2 and D8).
const NO_JOURNEY: Home =
    Home::Dropped("gsd-core has no journey kind (ADR 0370): the value is dropped");
/// Why an old persona's values are dropped (ADR 0370 D2 and D8).
const NO_PERSONA: Home =
    Home::Dropped("gsd-core has no persona kind (ADR 0370): the value is dropped");
/// Why an old term's values are dropped (ADR 0370 D2 and D8).
const NO_TERM: Home = Home::Dropped("gsd-core has no term kind (ADR 0370): the value is dropped");
/// Why an old channel's values are dropped (ADR 0370 D2 and D8): it was a term of its id, and the term leaves.
const NO_CHANNEL: Home =
    Home::Dropped("gsd-core has no channel or term kind (ADR 0370): the value is dropped");
/// Why an old constraint's metric and threshold are dropped (ADR 0370 D3): `requirement.metric` leaves, and a bound
/// is the requirement's text (ADR 0368 D2), which the conversion change writes.
const NO_METRIC: Home = Home::Dropped(
    "requirement.metric leaves @gsd/core (ADR 0370): a bound is the requirement's text (ADR 0368 D2), which the \
     conversion change writes",
);

/// Why an old constraint's `category` is dropped (ADR 0370 D3): it named a quality class (performance, security, …),
/// which no `@gsd/core` field holds. `requirement.category` is the requirement's heading, which the fill writes.
const NO_QUALITY_CLASS: Home =
    Home::Dropped("the constraint's quality class has no home in @gsd/core (ADR 0370)");

/// Why an old milestone's modules are dropped: a module is dropped and no kind holds it (ADR 0365).
const NO_MODULE: Home =
    Home::Dropped("a module is dropped and no @gsd/core kind holds it (ADR 0365)");
/// Why an old milestone's behaviors are dropped: the software kinds are dropped (ADR 0364 D2).
const NO_BEHAVIOR: Home = Home::Dropped(
    "a behavior is dropped with the software kinds (ADR 0364 D2): no phase lists one",
);
/// Why a release's completion date is dropped: a milestone's `shipped` day is computed from its completion.
const SHIPPED_IS_COMPUTED: Home = Home::Dropped(
    "milestone.shipped is computed from the milestone's completion, which absorb does not write (ADR 0370 D3)",
);

/// Why an old milestone that no converted release lists converts to nothing (ADR 0372 D4): a phase sits under a
/// version milestone, and none is made for it.
pub const NO_RELEASE: &str = "no converted release lists this milestone: it becomes no phase, and no version \
                              milestone is made for it (ADR 0372 D4)";

/// Why an old decision whose status is `superseded` converts to nothing (ADR 0370 D4 and D8): it no longer holds.
pub const NO_SUPERSESSION: &str =
    "gsd-core has no supersession (ADR 0370): a decision that no longer holds converts to nothing";

/// `leaves!("kind.field")`: the value's home field left `@gsd/core` (ADR 0370 D3 and D4), so it is dropped, naming
/// the field.
macro_rules! leaves {
    ($field:literal) => {
        Home::Dropped(concat!($field, " leaves @gsd/core (ADR 0370)"))
    };
}

/// `keeps!("kind", "field")`: the new kind keeps gsd-core's fields alone (ADR 0370 D3) and none holds the value.
macro_rules! keeps {
    ($kind:literal, $field:literal) => {
        Home::Dropped(concat!(
            "a ",
            $kind,
            " keeps gsd-core's fields alone (ADR 0370 D3): none holds its ",
            $field
        ))
    };
}

/// `"kind": ["field" => [home, …], …] …`: one row per field, in written order.
macro_rules! rows {
    ($($kind:literal: [$($field:literal => [$($home:expr),* $(,)?]),* $(,)?])*) => {
        &[$($(Row { kind: $kind, field: $field, homes: &[$($home),*] },)*)*]
    };
}

use How::*;
use Pending::*;

pub const TABLE: &[Row] = rows! {
    // ADR 0359's conversion table.
    "decision": [
        "status" => [
            to(DECISION, "outcome", Outcome),
            to(DECISION, "settled", Settled),
        ],
        "context" => [leaves!("decision.context")],
        "decision" => [to(DECISION, "text", As)],
        "consequences" => [leaves!("decision.consequences")],
        "date" => [leaves!("decision.decided")],
        "authors" => [leaves!("decision.deciders")],
        "deciders" => [leaves!("decision.deciders")],
        "reason" => [to(DECISION, "rationale", As)],
        "tags" => [leaves!("decision.tags")],
        "superseded_by" => [leaves!("decision.supersedes")],
        "invariants" => [leaves!("decision.affects")],
        "affects_features" => [leaves!("decision.affects")],
        "constraints" => [leaves!("decision.affects")],
        "alternatives" => [leaves!("decision.alternatives")],
        "refs" => [REFUSED],
        "description" => [REFUSED],
    ]
    // ADR 0359 D1-D6: a constraint is a requirement of category "Constraint".
    "constraint": [
        "description" => [to(REQUIREMENT, "text", Text)],
        "category" => [NO_QUALITY_CLASS],
        "priority" => [leaves!("requirement.priority")],
        "metric" => [NO_METRIC],
        "threshold" => [NO_METRIC],
        "governs_features" => [leaves!("requirement.constrains")],
        "protects" => [leaves!("requirement.protects")],
        "refs" => [leaves!("requirement.source")],
        "constrains" => [later(Behavior)],
        "enforced_by" => [later(Truth)],
        "scope" => [REFUSED],
        "verify" => [later(Truth)],
    ]
    // ADR 0370 D8: an old failure mode converts to nothing.
    "failure_mode": [
        "severity" => [NO_RISK],
        "occurrence" => [NO_RISK],
        "detection" => [NO_RISK],
        "post_mitigation" => [NO_RISK],
        "post_severity" => [NO_RISK],
        "post_occurrence" => [NO_RISK],
        "post_detection" => [NO_RISK],
        "rpn" => [NO_RISK],
        "cause" => [NO_RISK],
        "effect" => [NO_RISK],
        "mitigation" => [NO_RISK],
        "invariant" => [NO_RISK],
        "threatens_features" => [NO_RISK],
        "affected_behaviors" => [NO_RISK],
        "description" => [NO_RISK],
        "refs" => [NO_RISK],
        "risk" => [NO_RISK],
        "verify" => [NO_RISK],
    ]
    // ADR 0370 D8: an old invariant converts to nothing until the conversion change writes it as a requirement.
    "invariant": [
        "guarantee" => [NO_INVARIANT],
        "risk" => [NO_INVARIANT],
        "description" => [NO_INVARIANT],
        "refs" => [NO_INVARIANT],
        "expression" => [NO_INVARIANT],
        "verify" => [NO_INVARIANT],
    ]
    // ADR 0359 D7-D11: a feature is a requirement of category "Feature".
    "feature": [
        "problem" => [to(REQUIREMENT, "text", Text)],
        "description" => [to(REQUIREMENT, "text", Text)],
        "solution" => [to(DECISION, "text", Solution)],
        "status" => [
            to(REQUIREMENT, "tier", FeatureTier),
            computed(REQUIREMENT, "status", FEATURE_STATUS),
        ],
        "priority" => [leaves!("requirement.priority")],
        "tags" => [leaves!("requirement.tags")],
        "effort" => [leaves!("requirement.effort")],
        "owner" => [leaves!("requirement.source")],
        "contributors" => [leaves!("requirement.source")],
        "refs" => [leaves!("requirement.source")],
        "reason" => [to(REQUIREMENT, "reason", As)],
        "acceptance" => [later(Truth)],
        "tests" => [later(Truth)],
        "depends_on" => [REFUSED],
        "features" => [REFUSED],
    ]
    // ADR 0370 D8: an old journey converts to nothing.
    "journey": [
        "persona" => [NO_JOURNEY],
        "flow" => [NO_JOURNEY],
        "channels" => [NO_JOURNEY],
        "priority" => [NO_JOURNEY],
        "tags" => [NO_JOURNEY],
        "features" => [NO_JOURNEY],
        "description" => [NO_JOURNEY],
    ]
    // A milestone keeps gsd-core's fields alone (ADR 0370 D3).
    "deliverable": [
        "description" => [to(MILESTONE, "goal", As)],
        "journeys" => [NO_JOURNEY],
        "artifact_type" => [leaves!("milestone.artifact_types")],
        "depends_on" => [leaves!("milestone.depends_on")],
        "owner" => [leaves!("milestone.owner")],
        "contributors" => [leaves!("milestone.contributors")],
        "tags" => [leaves!("milestone.tags")],
        "status" => [computed(MILESTONE, "state", DELIVERABLE_STATE)],
        "modules" => [later(Package)],
        "version" => [REFUSED],
        "milestones" => [REFUSED],
        "reason" => [REFUSED],
    ]
    // ADR 0372 D4: a milestone a converted release lists becomes a phase under its version milestone.
    "milestone": [
        "description" => [to(PHASE, "goal", As)],
        "features" => [to(PHASE, "requirements", Requirements)],
        "exit_criteria" => [to(PHASE, "success_criteria", As)],
        "start_date" => [keeps!("phase", "start date")],
        "target_date" => [keeps!("phase", "target date")],
        "depends_on" => [to(PHASE, "depends_on", Earlier)],
        "owner" => [keeps!("phase", "owner")],
        "contributors" => [keeps!("phase", "contributors")],
        "tags" => [keeps!("phase", "tags")],
        "status" => [computed(PHASE, "status", PHASE_STATUS)],
        "modules" => [NO_MODULE],
        "behaviors" => [NO_BEHAVIOR],
        "blockers" => [keeps!("phase", "blockers")],
        "priority" => [keeps!("phase", "priority")],
        "reason" => [keeps!("phase", "reason")],
        "refs" => [keeps!("phase", "references")],
    ]
    // ADR 0370 D8: an old term, persona and channel convert to nothing.
    "term": [
        "definition" => [NO_TERM],
        "context" => [NO_TERM],
        "aliases" => [NO_TERM],
        "see_also" => [NO_TERM],
        "tags" => [NO_TERM],
        "module" => [NO_TERM],
    ]
    "persona": [
        "description" => [NO_PERSONA],
        "technical_level" => [NO_PERSONA],
        "goals" => [NO_PERSONA],
        "pain_points" => [NO_PERSONA],
        "tags" => [NO_PERSONA],
        "status" => [NO_PERSONA],
        "key_features" => [NO_PERSONA],
        "reason" => [NO_PERSONA],
    ]
    "channel": [
        "description" => [NO_CHANNEL],
        "interaction_model" => [NO_CHANNEL],
        "tags" => [NO_CHANNEL],
        "status" => [NO_CHANNEL],
        "url" => [NO_CHANNEL],
        "reason" => [NO_CHANNEL],
    ]
    // No kind holds a module yet: each value waits, and no entity is created.
    "module": [
        "family" => [later(Package)],
        "description" => [later(Package)],
        "features" => [later(Package)],
        "depends_on" => [later(Package)],
        "reason" => [later(Package)],
        "tags" => [later(Package)],
        "ports" => [later(Package)],
        "ports_defined" => [later(Package)],
    ]
    // ADR 0372 D4: a release becomes the version milestone its version names.
    "release": [
        "description" => [to(MILESTONE, "goal", As)],
        "version" => [to(MILESTONE, ID, Id)],
        "status" => [computed(MILESTONE, "state", RELEASE_STATE)],
        "deliverables" => [keeps!("milestone", "deliverables")],
        "milestones" => [to(PHASE, "milestone", Parent)],
        "target_date" => [leaves!("milestone.targets")],
        "release_date" => [SHIPPED_IS_COMPUTED],
        "changelog" => [to(MILESTONE, "delivered", As)],
        "depends_on" => [leaves!("milestone.depends_on")],
        "owner" => [leaves!("milestone.owner")],
        "contributors" => [leaves!("milestone.contributors")],
        "reason" => [keeps!("milestone", "reason")],
        "refs" => [keeps!("milestone", "references")],
        "tags" => [leaves!("milestone.tags")],
    ]
};

/// The `@gsd/core` kind an old kind's entity becomes: a release a version milestone, an old milestone a phase (ADR
/// 0372 D4); none for a module, whose rows create no entity, and for the kinds whose every value is dropped (ADR 0370
/// D8): failure_mode, invariant, journey, persona, term and channel.
pub fn new_kind(old: &str) -> Option<&'static str> {
    match old {
        "decision" => Some(DECISION),
        "constraint" | "feature" => Some(REQUIREMENT),
        "release" | "deliverable" => Some(MILESTONE),
        "milestone" => Some(PHASE),
        _ => None,
    }
}

/// Why an old kind's entities convert to nothing: the one reason every row of it drops its value with. None for a
/// kind with no row, or one a row of which writes, waits or refuses.
pub fn dropped(old: &str) -> Option<&'static str> {
    let mut reasons = rows_of(old).map(|row| match row.homes {
        [Home::Dropped(reason)] => Some(*reason),
        _ => None,
    });
    let reason = reasons.next()??;
    reasons.all(|other| other == Some(reason)).then_some(reason)
}

/// Why an old entity of `kind` whose status is `status` converts to nothing, each of its values ledgered `dropped`
/// (ADR 0370 D8): every row of its kind drops its value ([`dropped`]), or it is a decision that no longer holds.
pub fn converts_to_nothing(kind: &str, status: Option<&str>) -> Option<&'static str> {
    match (kind, status) {
        (DECISION, Some("superseded")) => Some(NO_SUPERSESSION),
        _ => dropped(kind),
    }
}

/// The rows of an old kind, in table order.
pub fn rows_of(kind: &str) -> impl Iterator<Item = &'static Row> + '_ {
    TABLE.iter().filter(move |row| row.kind == kind)
}

/// The row of an old (kind, field).
pub fn row_of(kind: &str, field: &str) -> Option<&'static Row> {
    TABLE
        .iter()
        .find(|row| row.kind == kind && row.field == field)
}
