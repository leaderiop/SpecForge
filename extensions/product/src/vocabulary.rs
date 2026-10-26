//! The closed value sets product's fields take (`types.spec`'s enum types),
//! each declared once (ADR 0071), and the suggested words of the open fields
//! (ADR 0279). The declaration declares each field with
//! `FieldBuilder::vocabulary`, so the host refuses another value (E061) and
//! completes, documents and publishes the values; the list commands filter
//! and sort by them, and the lifecycle pass and the rollups read a missing
//! status as its absent value, all through [`of`] and [`lifecycle`].

use specforge_extension_sdk::prelude::Vocabulary;

/// `Priority`, most important first.
pub const PRIORITY: Vocabulary = Vocabulary::new(&["critical", "high", "medium", "low"]);
/// `FeatureStatus`; a feature without one is proposed.
pub const FEATURE_STATUS: Vocabulary = Vocabulary::new(&[
    "proposed",
    "accepted",
    "in_progress",
    "done",
    "deferred",
    "deprecated",
])
.absent_as("proposed");
/// `DeliverableStatus`; a deliverable without one is a draft.
pub const DELIVERABLE_STATUS: Vocabulary =
    Vocabulary::new(&["draft", "in_progress", "shipped", "deprecated"]).absent_as("draft");
/// `MilestoneStatus`; a milestone without one is planned.
pub const MILESTONE_STATUS: Vocabulary =
    Vocabulary::new(&["planned", "in_progress", "completed", "blocked"]).absent_as("planned");
/// `ReleaseStatus`; a release without one is planned.
pub const RELEASE_STATUS: Vocabulary =
    Vocabulary::new(&["planned", "in_progress", "released", "recalled"]).absent_as("planned");
/// `PersonaStatus`; a persona without one is active.
pub const PERSONA_STATUS: Vocabulary =
    Vocabulary::new(&["active", "deprecated"]).absent_as("active");
/// `ChannelStatus`; a channel without one is active.
pub const CHANNEL_STATUS: Vocabulary =
    Vocabulary::new(&["active", "deprecated"]).absent_as("active");
/// `Effort`, smallest first; its weights are `queries::EFFORT_WEIGHTS`.
pub const EFFORT: Vocabulary = Vocabulary::new(&["xs", "s", "m", "l", "xl"]);
/// `TechnicalLevel`, most technical first.
pub const TECHNICAL_LEVEL: Vocabulary = Vocabulary::new(&[
    "expert",
    "advanced",
    "intermediate",
    "beginner",
    "non_technical",
]);
// ── The suggested words of the open fields ──
// A field that classifies a domain's things is an identifier: any one word
// is accepted (ADR 0279). These are the words its description suggests;
// nothing checks a value against them.

/// A deliverable's `artifact_type`.
pub const ARTIFACT_TYPES: &[&str] = &[
    "cli",
    "service",
    "library",
    "web_app",
    "mobile_app",
    "api",
    "extension",
    "documentation",
    "package",
    "physical_product",
    "media",
    "event",
    "training",
];
/// A channel's `interaction_model`.
pub const INTERACTION_MODELS: &[&str] = &[
    "request_response",
    "event_driven",
    "batch",
    "streaming",
    "bidirectional",
    "manual",
    "in_person",
    "self_service",
    "broadcast",
];
/// A module's `family`.
pub const MODULE_FAMILIES: &[&str] = &[
    "core",
    "platform",
    "extension",
    "integration",
    "advisory",
    "workstream",
];

// The members the queries, the lifecycle pass and the conditional rules
// name, each checked against its vocabulary when the crate compiles.
pub const FEATURE_IN_PROGRESS: &str = FEATURE_STATUS.member("in_progress");
pub const FEATURE_DONE: &str = FEATURE_STATUS.member("done");
pub const FEATURE_DEFERRED: &str = FEATURE_STATUS.member("deferred");
pub const FEATURE_DEPRECATED: &str = FEATURE_STATUS.member("deprecated");
pub const DELIVERABLE_SHIPPED: &str = DELIVERABLE_STATUS.member("shipped");
pub const DELIVERABLE_DEPRECATED: &str = DELIVERABLE_STATUS.member("deprecated");
pub const MILESTONE_IN_PROGRESS: &str = MILESTONE_STATUS.member("in_progress");
pub const MILESTONE_COMPLETED: &str = MILESTONE_STATUS.member("completed");
pub const MILESTONE_BLOCKED: &str = MILESTONE_STATUS.member("blocked");
pub const RELEASE_RECALLED: &str = RELEASE_STATUS.member("recalled");
pub const PERSONA_DEPRECATED: &str = PERSONA_STATUS.member("deprecated");
pub const CHANNEL_DEPRECATED: &str = CHANNEL_STATUS.member("deprecated");

/// Each lifecycle kind's `status` vocabulary, in the order `bulk-status`
/// reports the kinds.
pub const LIFECYCLES: &[(&str, &Vocabulary)] = &[
    ("feature", &FEATURE_STATUS),
    ("milestone", &MILESTONE_STATUS),
    ("deliverable", &DELIVERABLE_STATUS),
    ("persona", &PERSONA_STATUS),
    ("channel", &CHANNEL_STATUS),
    ("release", &RELEASE_STATUS),
];

/// Every other field with a vocabulary, by (kind, field).
const OTHERS: &[(&str, &str, &Vocabulary)] = &[
    ("feature", "priority", &PRIORITY),
    ("feature", "effort", &EFFORT),
    ("journey", "priority", &PRIORITY),
    ("milestone", "priority", &PRIORITY),
    ("persona", "technical_level", &TECHNICAL_LEVEL),
];

/// The `status` vocabulary of `kind`, when it is a lifecycle kind.
pub fn lifecycle(kind: &str) -> Option<&'static Vocabulary> {
    LIFECYCLES
        .iter()
        .find(|(lifecycle_kind, _)| *lifecycle_kind == kind)
        .map(|(_, vocabulary)| *vocabulary)
}

/// The vocabulary of `kind`'s `field`, when it has one.
pub fn of(kind: &str, field: &str) -> Option<&'static Vocabulary> {
    if field == "status" {
        return lifecycle(kind);
    }
    OTHERS
        .iter()
        .find(|(other_kind, other_field, _)| *other_kind == kind && *other_field == field)
        .map(|(_, _, vocabulary)| *vocabulary)
}

/// Every (kind, field, vocabulary): the lifecycles' `status`, then the others.
#[cfg(test)]
pub fn fields() -> impl Iterator<Item = (&'static str, &'static str, &'static Vocabulary)> {
    LIFECYCLES
        .iter()
        .map(|(kind, vocabulary)| (*kind, "status", *vocabulary))
        .chain(OTHERS.iter().copied())
}
