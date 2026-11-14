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
const STEP: &[Field] = &[
    str("action").r().doc("what the persona does at this step"),
    rf("requirement", "requirement").doc("the requirement the step exercises, once known"),
];

/// Who the product serves. Unscoped: a persona's id is project-wide.
pub const PERSONA: Kind = Kind::new(
    "persona",
    "A persona: who the product serves and what they want from it (beyond GSD)",
    &[
        strs("goals").doc("what the persona wants to achieve"),
        str("context")
            .r()
            .doc("who they are and the situation they work in"),
        strs("pain_points").doc("what gets in their way today"),
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
        rfs("milestones", "milestone")
            .inverse("journeys")
            .doc("every milestone it ships in; each milestone reads it back as `journeys`"),
        en("priority", PRIORITY).doc("MoSCoW; a journey nobody wants is not written"),
        strs("tags").doc("free words"),
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
        strs("avoid").doc("synonyms not to use for it"),
        strs("aliases").doc("other accepted names; never the same as `avoid`"),
        rfs("related", "term").doc("related terms"),
        str("context").doc("where or how it is used"),
        strs("tags").doc("free words"),
    ],
);
