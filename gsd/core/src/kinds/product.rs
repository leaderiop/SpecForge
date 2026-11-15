//! `persona`, `journey` and `term` (`spec/product.md`): beyond GSD, ADR 0357.

use gsd_kit::dsl::*;

/// MoSCoW: a journey or a requirement nobody wants is not written, so there is no "won't".
pub(super) const PRIORITY: &[&str] = &["must", "should", "could"];
/// Technical proficiency, most technical first: words that fit any domain (ADR 0279 D1).
const TECHNICAL_LEVEL: &[&str] = &[
    "expert",
    "advanced",
    "intermediate",
    "beginner",
    "non_technical",
];
const PERSONA_STATUS: &[&str] = &["active", "deprecated"];
/// The kinds whose prose `term.used_in` reads: every kind with a field its table marks prose
/// (`passes::vocabulary::PROSE`'s kinds), in the kinds' order.
const USED_IN: &[&str] = &[
    "project",
    "requirement",
    "persona",
    "journey",
    "milestone",
    "phase",
    "workstream",
    "retrospective",
    "lesson",
    "methodology",
    "context",
    "decision",
    "deferred_idea",
    "phase_spec",
    "locked_req",
    "interview",
    "edge",
    "spec_prohibition",
    "discussion",
    "research",
    "finding",
    "stack_choice",
    "pattern",
    "pitfall",
    "dont_hand_roll",
    "open_question",
    "plan",
    "task",
    "checkpoint",
    "truth",
    "artifact",
    "prohibition",
    "disposition",
    "review",
    "review_concern",
    "revision_conflict",
    "user_setup",
    "summary",
    "deliverable",
    "deviation",
    "pause",
    "handoff",
    "blocker",
    "verify_gap",
    "override",
    "human_item",
    "uat_test",
    "uat_gap",
    "manual_check",
    "milestone_audit",
    "broken_window",
    "risk",
    "invariant",
];
const STEP: &[Field] = &[
    str("action")
        .prose()
        .r()
        .doc("what the persona does at this step"),
    rf("requirement", "requirement").doc("the requirement the step exercises, once known"),
];

/// Who the product serves. Unscoped: a persona's id is project-wide.
pub const PERSONA: Kind = Kind::new(
    "persona",
    "A persona: who the product serves and what they want from it (beyond GSD)",
    &[
        strs("goals")
            .prose()
            .doc("what the persona wants to achieve"),
        str("context")
            .prose()
            .r()
            .doc("who they are and the situation they work in"),
        strs("pain_points")
            .prose()
            .doc("what gets in their way today"),
        en("technical_level", TECHNICAL_LEVEL).doc("technical proficiency, most technical first"),
        en("status", PERSONA_STATUS)
            .doc("deprecated when the product no longer serves them; absent means active"),
        strs("tags").doc("free words that group personas"),
    ],
);

/// A persona's path through the product. Unscoped: a journey's id is project-wide, and it lists every milestone it
/// ships in rather than being scoped by one (a scope is one reference, ADR 0284 D1).
pub const JOURNEY: Kind = Kind::new(
    "journey",
    "A persona's path through the product, step by step (beyond GSD)",
    &[
        rf("persona", "persona")
            .r()
            .inverse("journeys")
            .doc("who takes the journey; the persona reads it back as `journeys`"),
        strs("channels")
            .doc("the surfaces it goes through, as words (cli, web): a channel is no kind"),
        blocks("steps", STEP)
            .r()
            .doc("the ordered steps; a step names the requirement it exercises once known"),
        rfs("requirements", "requirement").doc(
            "the requirements it exercises as a whole; a step's `requirement` is the finer link",
        ),
        rfs("milestones", "milestone")
            .inverse("journeys")
            .doc("every milestone it ships in; each milestone reads it back as `journeys`"),
        en("priority", PRIORITY).doc("MoSCoW; a journey nobody wants is not written"),
        strs("tags").doc("free words"),
        rf("last_uat", "uat_session").c().doc(
            "the session of a UAT test that names it (`uat_test.journey`): the latest `started`, an undated session \
             older than every dated one, then the greatest id; absent when no test names it (ADR 0357 D22)",
        ),
        bool("covered").c().doc(
            "it names a requirement, each one it names is proven by the requirement thread (the predicate gsd/E515 \
             reads), and each step names one or the journey lists `requirements` (ADR 0357 D20)",
        ),
        rfs("tests", "test").c().doc(
            "the `@code/core` test facts tagged with it (`[gsd:<journey>]`), run or not: evidence beside `covered`, \
             which reads the requirement thread only; `[]` when none (ADR 0357 D21)",
        ),
    ],
);

/// A glossary entry. Unscoped: a term's id is project-wide.
pub const TERM: Kind = Kind::new(
    "term",
    "A glossary entry: a word of the project and what it means (beyond GSD)",
    &[
        str("name")
            .r()
            .doc("the word or phrase as prose writes it; a pass sees no title, so the vocabulary checks read this"),
        str("definition").r().doc("what it means in this project"),
        strs("avoid").doc(
            "synonyms not to use for it; a word the glossary accepts (this term's `name` or `aliases`, or another \
             term's) is gsd/W538",
        ),
        strs("aliases").doc("other accepted names; one also in a term's `avoid` is gsd/W538"),
        rfs("related", "term").doc("related terms"),
        str("context").doc("where or how it is used"),
        strs("tags").doc("free words"),
        rfs_any("used_in", USED_IN).c().doc(
            "the entities whose prose holds its `name` or one of its `aliases` as a whole word or phrase, by id; `[]` \
             when none (ADR 0357 D17)",
        ),
    ],
);
