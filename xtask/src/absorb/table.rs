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
    /// The old default, which the new kind implies: nothing is written; any other value stops the run.
    Default(&'static str),
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
    /// One reference to a converted entity.
    Ref,
    /// Ids or words as a list of texts, appended (one word is a one-word list).
    Words,
    /// Each text a step record `{action}`, in order.
    Steps,
    /// A decision `<id>_solution` created beside the entity, with the same title and file, the value as its text,
    /// affecting the entity.
    Solution,
    /// A feature status as a tier: proposed and deferred v2, deprecated out_of_scope (with a reason), else v1.
    FeatureTier,
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
const JOURNEY: &str = "journey";
const PERSONA: &str = "persona";
const TERM: &str = "term";
const MILESTONE: &str = "milestone";

/// A feature status as the computed requirement status it means.
const FEATURE_STATUS: Compare = Compare::Means(&[
    ("done", "complete"),
    ("in_progress", "in_progress"),
    ("proposed", "pending"),
    ("accepted", "pending"),
    ("deferred", "pending"),
]);
/// A milestone status as the computed milestone state; blocked has no equal.
const MILESTONE_STATE: Compare = Compare::Means(&[
    ("planned", "planned"),
    ("in_progress", "in_progress"),
    ("completed", "shipped"),
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
    // ADR 0359 D7-D11: a feature is a requirement of area "Feature".
    "feature": [
        "problem" => [to(REQUIREMENT, "text", Text)],
        "description" => [to(REQUIREMENT, "text", Text)],
        "solution" => [to(DECISION, "text", Solution)],
        "status" => [
            to(REQUIREMENT, "tier", FeatureTier),
            computed(REQUIREMENT, "status", FEATURE_STATUS),
        ],
        "priority" => [to(REQUIREMENT, "priority", Priority)],
        "tags" => [to(REQUIREMENT, "tags", As)],
        "effort" => [to(REQUIREMENT, "effort", As)],
        "owner" => [to(REQUIREMENT, "source", Words)],
        "contributors" => [to(REQUIREMENT, "source", Words)],
        "refs" => [to(REQUIREMENT, "source", Words)],
        "reason" => [to(REQUIREMENT, "reason", As)],
        "acceptance" => [later(Truth)],
        "tests" => [later(Truth)],
        "depends_on" => [REFUSED],
        "features" => [REFUSED],
    ]
    // ADR 0357.
    "journey": [
        "persona" => [to(JOURNEY, "persona", Ref)],
        "flow" => [to(JOURNEY, "steps", Steps)],
        "channels" => [to(JOURNEY, "channels", Words)],
        "priority" => [to(JOURNEY, "priority", Priority)],
        "tags" => [to(JOURNEY, "tags", As)],
        "features" => [to(JOURNEY, "requirements", Refs)],
        "description" => [REFUSED],
    ]
    "deliverable": [
        "description" => [to(MILESTONE, "goal", As)],
        "journeys" => [to(JOURNEY, "milestones", InvertOnto)],
        "artifact_type" => [to(MILESTONE, "artifact_types", Words)],
        "depends_on" => [to(MILESTONE, "depends_on", Refs)],
        "owner" => [to(MILESTONE, "owner", As)],
        "contributors" => [to(MILESTONE, "contributors", As)],
        "tags" => [to(MILESTONE, "tags", As)],
        "status" => [computed(MILESTONE, "state", DELIVERABLE_STATE)],
        "modules" => [later(Package)],
        "version" => [REFUSED],
        "milestones" => [REFUSED],
        "reason" => [REFUSED],
    ]
    "milestone": [
        "description" => [to(MILESTONE, "goal", As)],
        "features" => [to(MILESTONE, "target_features", Words)],
        "exit_criteria" => [to(MILESTONE, "exit_criteria", As)],
        "start_date" => [to(MILESTONE, "starts", As)],
        "target_date" => [to(MILESTONE, "targets", As)],
        "depends_on" => [to(MILESTONE, "depends_on", Refs)],
        "owner" => [to(MILESTONE, "owner", As)],
        "contributors" => [to(MILESTONE, "contributors", As)],
        "tags" => [to(MILESTONE, "tags", As)],
        "status" => [computed(MILESTONE, "state", MILESTONE_STATE)],
        "modules" => [later(Package)],
        "behaviors" => [later(Behavior)],
        "blockers" => [REFUSED],
        "priority" => [REFUSED],
        "reason" => [REFUSED],
        "refs" => [REFUSED],
    ]
    "term": [
        "definition" => [to(TERM, "definition", As)],
        "context" => [to(TERM, "context", As)],
        "aliases" => [to(TERM, "aliases", As)],
        "see_also" => [to(TERM, "related", Refs)],
        "tags" => [to(TERM, "tags", As)],
        "module" => [later(Package)],
    ]
    "persona": [
        "description" => [to(PERSONA, "context", As)],
        "technical_level" => [to(PERSONA, "technical_level", As)],
        "goals" => [to(PERSONA, "goals", As)],
        "pain_points" => [to(PERSONA, "pain_points", As)],
        "tags" => [to(PERSONA, "tags", As)],
        "status" => [to(PERSONA, "status", As)],
        "key_features" => [to(REQUIREMENT, "personas", InvertOnto)],
        "reason" => [REFUSED],
    ]
    // ADR 0357: a channel is a word; its entity is a term of its id.
    "channel": [
        "description" => [to(TERM, "definition", As)],
        "interaction_model" => [to(TERM, "tags", Words)],
        "tags" => [to(TERM, "tags", As)],
        "status" => [Home::Default("active")],
        "url" => [REFUSED],
        "reason" => [REFUSED],
    ]
    // No kind holds a module or a release yet: each value waits, and no entity is created.
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
    "release": [
        "description" => [later(Release)],
        "version" => [later(Release)],
        "status" => [later(Release)],
        "deliverables" => [later(Release)],
        "milestones" => [later(Release)],
        "target_date" => [later(Release)],
        "release_date" => [later(Release)],
        "changelog" => [later(Release)],
        "depends_on" => [later(Release)],
        "owner" => [later(Release)],
        "contributors" => [later(Release)],
        "reason" => [later(Release)],
        "refs" => [later(Release)],
        "tags" => [later(Release)],
    ]
};

/// The `@gsd/core` kind an old kind's entity becomes, keeping its id; none for module and release, whose rows create
/// no entity.
pub fn new_kind(old: &str) -> Option<&'static str> {
    match old {
        "decision" => Some(DECISION),
        "constraint" | "feature" => Some(REQUIREMENT),
        "failure_mode" => Some(RISK),
        "invariant" => Some(INVARIANT),
        "journey" => Some(JOURNEY),
        "persona" => Some(PERSONA),
        "term" | "channel" => Some(TERM),
        "milestone" | "deliverable" => Some(MILESTONE),
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
