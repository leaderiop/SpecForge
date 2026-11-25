//! `@gsd/core`'s kind names, which the re-import refusal looks for (ADR 0391 D10). A test holds the list equal to what
//! `@gsd/core` declares; the crate does not depend on `gsd-core` (D12).

/// Every kind `@gsd/core` declares, sorted.
pub const CORE_KINDS: &[&str] = &[
    "artifact",
    "blocker",
    "broken_window",
    "checkpoint",
    "commit",
    "context",
    "decision",
    "deferred_idea",
    "deliverable",
    "deviation",
    "discussion",
    "disposition",
    "dont_hand_roll",
    "edge",
    "finding",
    "handoff",
    "human_item",
    "interview",
    "key_link",
    "lesson",
    "locked_req",
    "manual_check",
    "methodology",
    "milestone",
    "milestone_audit",
    "open_question",
    "override",
    "package_audit",
    "pattern",
    "pause",
    "phase",
    "phase_spec",
    "pitfall",
    "plan",
    "plan_start",
    "prohibition",
    "project",
    "quick_batch",
    "refactor_proposal",
    "requirement",
    "research",
    "retrospective",
    "review",
    "review_concern",
    "revision_conflict",
    "source",
    "spec_prohibition",
    "stack_choice",
    "summary",
    "task",
    "truth",
    "uat_gap",
    "uat_session",
    "uat_test",
    "user_setup",
    "validation",
    "validation_row",
    "verification",
    "verify_gap",
    "wave0_item",
    "workstream",
];

/// The order the plan creates kinds in, so an entity is created before one that names it; a kind not listed comes
/// after these, by name.
pub const CREATE_ORDER: &[&str] = &[
    "milestone",
    "requirement",
    "project",
    "decision",
    "phase",
    "plan",
    "task",
    "truth",
    "artifact",
    "key_link",
    "summary",
    "deliverable",
    "context",
    "discussion",
    "deferred_idea",
    "research",
    "source",
    "finding",
    "stack_choice",
    "pattern",
    "pitfall",
    "dont_hand_roll",
    "open_question",
    "uat_session",
    "uat_test",
    "uat_gap",
    "verification",
    "verify_gap",
    "human_item",
    "security_audit",
    "trust_boundary",
    "threat",
    "accepted_risk",
    "audit_run",
];

/// Each `@gsd/core` and `@gsd/security` kind's required fields that an author writes (neither computed nor harness),
/// in declaration order; a kind with none is not listed. A test holds it equal to both declarations. A record missing
/// one is dropped before the plan is built (ADR 0391 D9): the host would refuse the whole plan for it.
#[rustfmt::skip]
pub const REQUIRED: &[(&str, &[&str])] = &[
    ("project", &["name", "what", "core_value", "context", "constraints"]),
    ("requirement", &["text", "category", "tier"]),
    ("phase", &["milestone", "slug", "goal", "success_criteria"]),
    ("workstream", &["purpose"]),
    ("retrospective", &["milestone"]),
    ("lesson", &["text"]),
    ("methodology", &["name"]),
    ("context", &["boundary"]),
    ("decision", &["text", "origin"]),
    ("deferred_idea", &["text", "phase", "origin"]),
    ("phase_spec", &["phase", "goal"]),
    ("locked_req", &["spec", "title", "current", "target", "acceptance"]),
    ("interview", &["spec", "round", "perspective", "question"]),
    ("edge", &["spec", "requirement", "category", "status"]),
    ("spec_prohibition", &["spec", "requirement", "statement", "status"]),
    ("discussion", &["phase"]),
    ("research", &["confidence", "summary"]),
    ("finding", &["research", "text", "confidence"]),
    ("stack_choice", &["research", "name", "purpose"]),
    ("pattern", &["research", "name", "what"]),
    ("pitfall", &["research", "name", "goes_wrong", "why", "avoid"]),
    ("dont_hand_roll", &["research", "problem", "instead", "why"]),
    ("open_question", &["research", "question", "unclear"]),
    ("source", &["research", "url", "tier"]),
    ("package_audit", &["research", "package", "registry"]),
    ("plan", &["type", "files_modified", "objective"]),
    ("task", &["plan", "type", "action", "done"]),
    ("checkpoint", &["plan", "kind", "resume_signal"]),
    ("truth", &["plan", "text"]),
    ("artifact", &["plan", "path", "provides"]),
    ("key_link", &["plan", "from", "to", "via"]),
    ("prohibition", &["plan", "statement", "verification"]),
    ("disposition", &["plan", "round", "feedback", "outcome"]),
    ("review", &["phase", "plans"]),
    ("review_concern", &["review", "severity", "text"]),
    ("revision_conflict", &["phase", "plan", "dimension", "property", "conflicts_with"]),
    ("user_setup", &["plan", "service", "why"]),
    ("plan_start", &["plan"]),
    ("summary", &["plan", "status", "one_liner", "subsystem", "requirements_completed"]),
    ("deliverable", &["summary", "description", "human_judgment"]),
    ("deviation", &["summary", "kind", "text"]),
    ("pause", &["reason"]),
    ("handoff", &["state", "current_state", "next_action"]),
    ("blocker", &["text"]),
    ("quick_batch", &["items"]),
    ("verification", &["phase"]),
    ("verify_gap", &["verification", "truth", "reason"]),
    ("override", &["verification", "truth", "reason"]),
    ("human_item", &["verification", "test", "expected", "why_human"]),
    ("uat_session", &["phase", "source"]),
    ("uat_test", &["session", "expected"]),
    ("uat_gap", &["test", "truth"]),
    ("validation", &["phase"]),
    ("validation_row", &["validation", "task", "test_type"]),
    ("wave0_item", &["validation", "path", "kind"]),
    ("manual_check", &["validation", "behavior", "why_manual", "instructions"]),
    ("milestone_audit", &["milestone"]),
    ("broken_window", &["kind", "phase", "description"]),
    ("refactor_proposal", &["phase", "target_file", "target_function"]),
    // @gsd/security
    ("trust_boundary", &["plan", "boundary", "description"]),
    ("threat", &["plan", "category", "component", "severity", "disposition", "mitigation"]),
    ("security_audit", &["phase"]),
    ("accepted_risk", &["audit", "threat", "rationale", "accepted_by"]),
    ("audit_run", &["audit"]),
];

/// Each `@gsd/core` and `@gsd/security` block field's required, author-written members, as `(kind, block, members)` in
/// declaration order. A test holds it equal to both declarations. A record writing a block item without one is dropped
/// before the plan is built: the host would refuse the whole plan for it (E006 on a member).
#[rustfmt::skip]
pub const REQUIRED_MEMBERS: &[(&str, &str, &[&str])] = &[
    ("project", "out_of_scope", &["why"]),
    ("milestone", "abandoned", &["reason", "ruled_by", "at"]),
    ("plan", "coupling_justified", &["peer", "reason"]),
    ("task", "reversibility", &["rating"]),
    ("task", "verify", &["automated"]),
    ("checkpoint", "options", &["name"]),
    ("review", "reviewers", &["name", "model", "source"]),
    ("review", "lanes", &["reviewer", "summary"]),
    ("review", "coverage", &["reviewer"]),
    ("review", "unchecked", &["symbol", "why"]),
    ("user_setup", "env_vars", &["name"]),
    ("summary", "requires", &["phase", "provides"]),
    ("deliverable", "checks", &["kind", "ref"]),
    ("verification", "deferred", &["truth", "addressed_in", "evidence"]),
    ("milestone_audit", "tech_debt", &["item"]),
    ("milestone_audit", "debt_accepted", &["by", "at"]),
];

/// The required, author-written members of the block field `block` of `kind` ([`REQUIRED_MEMBERS`]).
pub fn required_members(kind: &str, block: &str) -> &'static [&'static str] {
    REQUIRED_MEMBERS
        .iter()
        .find(|(k, b, _)| *k == kind && *b == block)
        .map_or(&[], |(_, _, members)| members)
}

/// The required, author-written fields of `kind` ([`REQUIRED`]); none for a kind not listed.
pub fn required(kind: &str) -> &'static [&'static str] {
    REQUIRED
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or(&[], |(_, fields)| fields)
}

/// Where `kind` stands in [`CREATE_ORDER`].
pub fn rank(kind: &str) -> usize {
    CREATE_ORDER
        .iter()
        .position(|k| *k == kind)
        .unwrap_or(CREATE_ORDER.len())
}
