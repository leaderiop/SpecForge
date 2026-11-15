//! The absorb table (ADR 0360 D2): one row per field of an old `@specforge/governance` or `@specforge/product` kind,
//! as the old builtins' registry holds it, and one per old kind that takes `verify`. Each row names its homes in
//! `@gsd/core`; every home of a row is applied to the row's value. The table is the measure: a test counts it.

use serde::Serialize;

/// What a Later value waits for: a kind `@gsd/core` does not hold yet, never a date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Pending {
    Release,
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
}

/// An FMEA rating's word scale (ADR 0358 D8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    Severity,
    Occurrence,
    Detection,
}

/// How a value is written to its `To` home.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    /// As written: a text, a date, a number, a list of texts (appended to what the field holds; a scheme
    /// reference in it is ledgered, as its `ref` cannot be written).
    As,
    /// A text joined after what the field holds, a blank line between.
    Text,
    /// References to converted entities, appended to a list; one that leaves the run is ledgered.
    Refs,
    /// Written on each entity the value names (the home's field is of that entity's kind), naming this one.
    InvertOnto,
    /// A decision status as `outcome`: proposed → pending, deprecated → revisit.
    Outcome,
    /// A decision status as `settled`: accepted → true.
    Settled,
    /// Each string an option record `{name}`.
    Options,
    /// A rating word as its number.
    Sod(Scale),
    /// A rating word as its number in the `post` block, unless `post_mitigation` writes that member.
    PostWord(Scale),
    /// `post_mitigation`'s rating words as the `post` block's numbers (its `rpn` is compared).
    Post,
    /// The bound line of a metric (with the `threshold` field, when the metric is one name) as the `metric`
    /// record; the other lines join the entity's `text`.
    Bound,
    /// A threshold: read with the metric's bound; when no bound takes it, it joins the text.
    Threshold,
    /// A quality class: the ten words as they are, interoperability as compatibility.
    Category,
    /// critical, high, medium, low as must, should, could, could.
    Priority,
}

/// How a computed value is compared with the old one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compare {
    /// A decision status: `in_force` is false exactly when the old status is `superseded`.
    Superseded,
    /// The same value.
    Equal,
    /// `post_mitigation`'s `rpn` member.
    PostRpn,
    /// An assessment in words: no number equals it, so it is always ledgered with the new value.
    Assessment,
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
const RISK: &str = "risk";
const INVARIANT: &str = "invariant";

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

/// `"kind": ["field" => [home, …], …] …`: one row per field, in written order.
macro_rules! rows {
    ($($kind:literal: [$($field:literal => [$($home:expr),* $(,)?]),* $(,)?])*) => {
        &[$($(Row { kind: $kind, field: $field, homes: &[$($home),*] },)*)*]
    };
}

use Compare::*;
use How::*;
use Pending::*;
use Scale::*;

pub const TABLE: &[Row] = rows! {
    // ADR 0359's conversion table.
    "decision": [
        "status" => [
            to(DECISION, "outcome", Outcome),
            to(DECISION, "settled", Settled),
            computed(DECISION, "in_force", Superseded),
        ],
        "context" => [to(DECISION, "context", As)],
        "decision" => [to(DECISION, "text", As)],
        "consequences" => [to(DECISION, "consequences", As)],
        "date" => [to(DECISION, "decided", As)],
        "authors" => [to(DECISION, "deciders", As)],
        "deciders" => [to(DECISION, "deciders", As)],
        "reason" => [to(DECISION, "rationale", As)],
        "tags" => [to(DECISION, "tags", As)],
        "superseded_by" => [to(DECISION, "supersedes", InvertOnto)],
        "invariants" => [to(DECISION, "affects", Refs)],
        "affects_features" => [to(DECISION, "affects", Refs)],
        "constraints" => [to(DECISION, "affects", Refs)],
        "alternatives" => [to(DECISION, "alternatives", Options)],
        "refs" => [REFUSED],
        "description" => [REFUSED],
    ]
    // ADR 0359 D1-D6: a constraint is a requirement of area "Constraint".
    "constraint": [
        "description" => [to(REQUIREMENT, "text", Text)],
        "category" => [to(REQUIREMENT, "category", Category)],
        "priority" => [to(REQUIREMENT, "priority", Priority)],
        "metric" => [to(REQUIREMENT, "metric", Bound)],
        "threshold" => [to(REQUIREMENT, "metric", Threshold)],
        "governs_features" => [to(REQUIREMENT, "constrains", Refs)],
        "protects" => [to(REQUIREMENT, "protects", Refs)],
        "refs" => [to(REQUIREMENT, "source", As)],
        "constrains" => [later(Behavior)],
        "enforced_by" => [later(Truth)],
        "scope" => [REFUSED],
        "verify" => [later(Truth)],
    ]
    // ADR 0358 D8: the word scale and field map.
    "failure_mode": [
        "severity" => [to(RISK, "severity", Sod(Severity))],
        "occurrence" => [to(RISK, "occurrence", Sod(Occurrence))],
        "detection" => [to(RISK, "detection", Sod(Detection))],
        "post_mitigation" => [to(RISK, "post", Post), computed(RISK, "post_rpn", PostRpn)],
        "post_severity" => [to(RISK, "post", PostWord(Severity))],
        "post_occurrence" => [to(RISK, "post", PostWord(Occurrence))],
        "post_detection" => [to(RISK, "post", PostWord(Detection))],
        "rpn" => [computed(RISK, "rpn", Equal)],
        "cause" => [to(RISK, "cause", As)],
        "effect" => [to(RISK, "effect", As)],
        "mitigation" => [to(RISK, "mitigation", As)],
        "invariant" => [to(RISK, "mitigated_by", Refs)],
        "threatens_features" => [to(RISK, "affects", Refs)],
        "affected_behaviors" => [to(RISK, "affects", Refs)],
        "description" => [to(RISK, "description", As)],
        "refs" => [to(RISK, "refs", As)],
        "risk" => [computed(RISK, "rpn", Assessment)],
        "verify" => [later(Truth)],
    ]
    // ADR 0358 D9.
    "invariant": [
        "guarantee" => [to(INVARIANT, "statement", As)],
        "risk" => [to(INVARIANT, "criticality", As)],
        "description" => [to(INVARIANT, "description", As)],
        "refs" => [to(INVARIANT, "refs", As)],
        "expression" => [later(Truth)],
    ]
};

/// The `@gsd/core` kind an old kind's entity becomes; none for a kind whose rows create no entity, or that has no
/// rows yet.
pub fn new_kind(old: &str) -> Option<&'static str> {
    match old {
        "decision" => Some(DECISION),
        "constraint" => Some(REQUIREMENT),
        "failure_mode" => Some(RISK),
        "invariant" => Some(INVARIANT),
        _ => None,
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

/// What a reference to an entity of `kind`, a kind no row gives a home, waits for.
pub fn pending_of(kind: &str) -> Pending {
    match kind {
        "behavior" => Behavior,
        "module" => Package,
        "release" => Release,
        _ => Truth,
    }
}
