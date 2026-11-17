//! The absorb table (ADR 0360 D2, ADR 0372 D1 and D2): one fate per old kind, and one row per field of an old kind
//! that converts, as the old builtins' registry holds it, and one per converting kind that takes `verify`. Each row
//! names its homes in `@gsd/core`; every home of a row is applied to the row's value. The table is the measure: a test
//! counts it.
//!
//! Six old kinds convert and eleven are dropped whole, each with its reason ([`FATES`]). A home is one of three
//! classes: To (an authored field, through a named way of writing), Computed (the new kind computes it, and the old
//! value is compared after the write) or Dropped (no `@gsd/core` field takes the value, and the ledger says why).
//! No value waits and none is refused for having no home: what stops a run before any write is the refused class
//! (an entity of a kind with no fate, a field of a converting kind with no row, two entities given one new id, an id
//! the target refuses, a required value no rule fills).
//!
//! A release becomes a version milestone and each milestone it lists a phase under it (ADR 0372 D4). No home writes
//! a reference except the phase's three: its `milestone`, its `requirements` and its `depends_on`, each through the
//! id map ([`super::ids`]).

/// What an old kind becomes (ADR 0372 D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fate {
    /// Its entities convert to entities of this `@gsd/core` kind, through its rows. One may still be dropped whole: a
    /// decision that no longer holds, a milestone no converted release lists.
    To(&'static str),
    /// Its entities are dropped whole: each one's title and every value ledgered `dropped` with this reason. Its kind
    /// has no row.
    Dropped(&'static str),
}

impl Fate {
    /// The `@gsd/core` kind it converts to; none when it is dropped.
    pub fn new_kind(self) -> Option<&'static str> {
        match self {
            Fate::To(kind) => Some(kind),
            Fate::Dropped(_) => None,
        }
    }
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
    /// A bound: the value appended to the last line of the text the field holds, after one space (ADR 0368 D2), so a
    /// threshold reads on its metric's line (`throughput >= 200`); the value alone when the field holds none.
    Bound,
    /// A constraint's own category word as its requirement's category, by the one rule that also gives its id's
    /// prefix ([`super::ids::category`], ADR 0372 D3): the quality class is the heading (ADR 0370 D5). A blank word
    /// writes nothing, and the fill writes the kind's name.
    Category,
    /// A feature status as a tier: proposed and deferred v2, deprecated out_of_scope (with a reason), else v1.
    FeatureTier,
    /// A decision status as `settled`: accepted → true. Only a decision in force converts (ADR 0372 D5), so its status
    /// is always `accepted` here.
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

const DECISION: &str = "decision";
const REQUIREMENT: &str = "requirement";
const MILESTONE: &str = "milestone";
const PHASE: &str = "phase";

/// Why a deliverable is dropped whole (ADR 0372 D1).
const DELIVERABLE: &str = "GSD's deliverable is a coverage item of a plan's summary: the old spec holds no plan or \
                           summary, and none is invented (ADR 0372 D1)";
/// Why a failure mode is dropped whole (ADR 0370 D2, ADR 0372 D1).
const FAILURE_MODE: &str = "gsd-core has no risk kind (ADR 0370 D2), and a threat belongs to @gsd/security's \
                            register, which needs a plan: an old failure mode becomes no requirement and no threat \
                            (ADR 0372 D1)";
/// Why a module is dropped whole, and why a milestone's modules are dropped (ADR 0365).
const MODULE: &str = "a module is dropped and no @gsd/core kind holds it (ADR 0365)";

/// `no_kind!("journey")`: gsd-core has no kind for it (ADR 0370 D2).
macro_rules! no_kind {
    ($kind:literal) => {
        concat!("gsd-core has no ", $kind, " kind (ADR 0370 D2)")
    };
}

/// `software!("behavior")`: the software kinds are dropped (ADR 0364 D1 and D2).
macro_rules! software {
    ($kind:literal) => {
        concat!(
            "gsd-core has no ",
            $kind,
            " kind: the software kinds are dropped (ADR 0364 D1, D2)"
        )
    };
}

/// Every old kind and its fate (ADR 0372 D1): governance's four, product's nine, software's four. One lookup
/// ([`fate`]) answers what a kind becomes.
pub const FATES: [(&str, Fate); 17] = [
    // A decision in force becomes a decision (ADR 0372 D5).
    ("decision", Fate::To(DECISION)),
    // A constraint becomes a requirement whose text holds its bound (ADR 0372 D6).
    ("constraint", Fate::To(REQUIREMENT)),
    // A failure mode becomes no requirement and no threat.
    ("failure_mode", Fate::Dropped(FAILURE_MODE)),
    // An invariant becomes a requirement whose text is its guarantee (ADR 0372 D6).
    ("invariant", Fate::To(REQUIREMENT)),
    // A feature becomes a requirement (ADR 0359 D7-D11).
    ("feature", Fate::To(REQUIREMENT)),
    // A journey has no GSD kind.
    ("journey", Fate::Dropped(no_kind!("journey"))),
    // A deliverable is a plan summary's coverage item, and the old spec holds no plan.
    ("deliverable", Fate::Dropped(DELIVERABLE)),
    // A milestone a converted release lists becomes a phase under its version milestone (ADR 0372 D4).
    ("milestone", Fate::To(PHASE)),
    // A module has no GSD kind (ADR 0365).
    ("module", Fate::Dropped(MODULE)),
    // A term has no GSD kind.
    ("term", Fate::Dropped(no_kind!("term"))),
    // A persona has no GSD kind.
    ("persona", Fate::Dropped(no_kind!("persona"))),
    // A channel has no GSD kind.
    ("channel", Fate::Dropped(no_kind!("channel"))),
    // A release becomes the version milestone its version names (ADR 0372 D4).
    ("release", Fate::To(MILESTONE)),
    // The software kinds have no GSD kind (ADR 0364).
    ("behavior", Fate::Dropped(software!("behavior"))),
    ("event", Fate::Dropped(software!("event"))),
    ("type", Fate::Dropped(software!("type"))),
    ("port", Fate::Dropped(software!("port"))),
];

/// The fate of the old kind `kind`; none for a kind the table does not know, which stops a run (ADR 0372 D2).
pub fn fate(kind: &str) -> Option<Fate> {
    FATES
        .iter()
        .find(|(old, _)| *old == kind)
        .map(|(_, fate)| *fate)
}

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

/// Why an old milestone's modules are dropped: a module is dropped and no kind holds it (ADR 0365).
const NO_MODULE: Home = Home::Dropped(MODULE);
/// Why an old milestone's behaviors are dropped: the software kinds are dropped (ADR 0364 D2).
const NO_BEHAVIOR: Home = Home::Dropped(
    "a behavior is dropped with the software kinds (ADR 0364 D2): no phase lists one",
);
/// Why a constraint's `constrains` and `enforced_by` are dropped: the behaviors they name are dropped.
const NAMES_BEHAVIORS: Home = Home::Dropped(
    "it names behaviors, which are dropped with the software kinds (ADR 0364 D2): no requirement field names one \
     (ADR 0372 D6)",
);
/// Why a constraint's and an invariant's `verify` lines are dropped.
const NO_OBLIGATIONS: Home = Home::Dropped(
    "a requirement holds no list of obligations (ADR 0368 D1): a verify line is dropped (ADR 0372 D6)",
);
/// Why an invariant's formal expression is dropped.
const NO_EXPRESSION: Home = Home::Dropped(
    "a requirement holds no formal expression, and the prove pass does not change (ADR 0368 D3): it is dropped (ADR \
     0372 D6)",
);
/// Why a feature's acceptance and tests are dropped.
const PLAN_TRUTHS: Home = Home::Dropped(
    "GSD writes acceptance as a plan's truths, and the conversion creates no plan (ADR 0372 D6)",
);
/// Why a release's completion date is dropped: a milestone's `shipped` day is computed from its completion.
const SHIPPED_IS_COMPUTED: Home = Home::Dropped(
    "milestone.shipped is computed from the milestone's completion, which absorb does not write (ADR 0370 D3)",
);

/// Why an old milestone that no converted release lists converts to nothing (ADR 0372 D4): a phase sits under a
/// version milestone, and none is made for it.
pub const NO_RELEASE: &str = "no converted release lists this milestone: it becomes no phase, and no version \
                              milestone is made for it (ADR 0372 D4)";

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

pub const TABLE: &[Row] = rows! {
    // ADR 0359's conversion table; only a decision in force converts (ADR 0372 D5).
    "decision": [
        "status" => [to(DECISION, "settled", Settled)],
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
        "refs" => [keeps!("decision", "references")],
        "description" => [keeps!("decision", "description")],
    ]
    // ADR 0359 D1-D6: a constraint is a requirement whose category is its own quality class, else "Constraint" (ADR
    // 0372 D3); its rows in text order: the description, then the metric, the threshold on the metric's last line (ADR
    // 0372 D6).
    "constraint": [
        "description" => [to(REQUIREMENT, "text", Text)],
        "metric" => [to(REQUIREMENT, "text", Text)],
        "threshold" => [to(REQUIREMENT, "text", Bound)],
        "category" => [to(REQUIREMENT, "category", Category)],
        "priority" => [leaves!("requirement.priority")],
        "governs_features" => [leaves!("requirement.constrains")],
        "protects" => [leaves!("requirement.protects")],
        "refs" => [leaves!("requirement.source")],
        "constrains" => [NAMES_BEHAVIORS],
        "enforced_by" => [NAMES_BEHAVIORS],
        "scope" => [keeps!("requirement", "scope")],
        "verify" => [NO_OBLIGATIONS],
    ]
    // ADR 0372 D6: an invariant is a requirement of category "Invariant", its text its guarantee then its
    // description.
    "invariant": [
        "guarantee" => [to(REQUIREMENT, "text", Text)],
        "description" => [to(REQUIREMENT, "text", Text)],
        "risk" => [keeps!("requirement", "risk (ADR 0372 D6)")],
        "refs" => [keeps!("requirement", "references (ADR 0372 D6)")],
        "expression" => [NO_EXPRESSION],
        "verify" => [NO_OBLIGATIONS],
    ]
    // ADR 0359 D7-D11: a feature is a requirement of category "Feature", its text its problem, description and
    // solution (ADR 0372 D6).
    "feature": [
        "problem" => [to(REQUIREMENT, "text", Text)],
        "description" => [to(REQUIREMENT, "text", Text)],
        "solution" => [to(REQUIREMENT, "text", Text)],
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
        "acceptance" => [PLAN_TRUTHS],
        "tests" => [PLAN_TRUTHS],
        "depends_on" => [keeps!("requirement", "dependencies")],
        "features" => [keeps!("requirement", "features")],
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
