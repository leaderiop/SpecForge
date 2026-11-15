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
    /// No tracked project writes it and nothing holds it: a value stops the run (ADR 0357's refused class).
    Refused,
}

/// How a value is written to its `To` home.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    /// As written: a text, a date, a list of texts (appended to what the field holds).
    As,
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
}

/// How a computed value is compared with the old one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compare {
    /// A decision status: `in_force` is false exactly when the old status is `superseded`.
    Superseded,
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

const fn to(kind: &'static str, field: &'static str, how: How) -> Home {
    Home::To { kind, field, how }
}

const fn row(kind: &'static str, field: &'static str, homes: &'static [Home]) -> Row {
    Row { kind, field, homes }
}

pub const TABLE: &[Row] = &[
    // decision → decision (ADR 0359's conversion table)
    row(
        "decision",
        "status",
        &[
            to("decision", "outcome", How::Outcome),
            to("decision", "settled", How::Settled),
            Home::Computed {
                kind: "decision",
                field: "in_force",
                compare: Compare::Superseded,
            },
        ],
    ),
    row("decision", "context", &[to("decision", "context", How::As)]),
    row("decision", "decision", &[to("decision", "text", How::As)]),
    row(
        "decision",
        "consequences",
        &[to("decision", "consequences", How::As)],
    ),
    row("decision", "date", &[to("decision", "decided", How::As)]),
    row(
        "decision",
        "authors",
        &[to("decision", "deciders", How::As)],
    ),
    row(
        "decision",
        "deciders",
        &[to("decision", "deciders", How::As)],
    ),
    row(
        "decision",
        "reason",
        &[to("decision", "rationale", How::As)],
    ),
    row("decision", "tags", &[to("decision", "tags", How::As)]),
    row(
        "decision",
        "superseded_by",
        &[to("decision", "supersedes", How::InvertOnto)],
    ),
    row(
        "decision",
        "invariants",
        &[to("decision", "affects", How::Refs)],
    ),
    row(
        "decision",
        "affects_features",
        &[to("decision", "affects", How::Refs)],
    ),
    row(
        "decision",
        "constraints",
        &[to("decision", "affects", How::Refs)],
    ),
    row(
        "decision",
        "alternatives",
        &[to("decision", "alternatives", How::Options)],
    ),
    row("decision", "refs", &[Home::Refused]),
    row("decision", "description", &[Home::Refused]),
];

/// The `@gsd/core` kind an old kind's entity becomes; none for a kind whose rows create no entity, or that has no
/// rows yet.
pub fn new_kind(old: &str) -> Option<&'static str> {
    match old {
        "decision" => Some("decision"),
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
        "behavior" => Pending::Behavior,
        "module" => Pending::Package,
        "release" => Pending::Release,
        _ => Pending::Truth,
    }
}
