//! Ported from gsd-core v1.19.0 src/edge-probe.cts, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The edge probe, the engine's first adapter: the five data and behaviour shapes a requirement's prose shows, the
//! closed taxonomy of eight edge categories each shape raises, the proposal of one unresolved row per raised category
//! (or one `unclassified` row when no cue matches), and the `explicit | backstop` tiers of a resolved edge. The shape
//! cues are upstream's JavaScript regular expressions, character for character, run by the crate's JavaScript regex
//! reader ([`crate::js_regex`]) with the `i` flag as its ASCII folding.
//!
//! gsd-core modules this file ports: edge-probe.cjs.
//!
//! Left out: the `require.main` CLI entry, which is `specforge gsd edge-probe`.

use std::sync::OnceLock;

use super::{CoverageReport, Item, Resolution, Validators, filled};
use crate::js_regex::Regex;

/// A data or behaviour shape of a requirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    NumericRange,
    Collection,
    Text,
    Stateful,
    Io,
}

impl Shape {
    /// Every shape, in upstream's order of `SHAPE_CUES`.
    pub const ALL: [Shape; 5] = [
        Shape::NumericRange,
        Shape::Collection,
        Shape::Text,
        Shape::Stateful,
        Shape::Io,
    ];

    /// Upstream's name: `numeric-range`.
    pub fn upstream(self) -> &'static str {
        match self {
            Shape::NumericRange => "numeric-range",
            Shape::Collection => "collection",
            Shape::Text => "text",
            Shape::Stateful => "stateful",
            Shape::Io => "io",
        }
    }

    /// The kernel enum's spelling, a `shapes` field value: `numeric_range`.
    pub fn value(self) -> &'static str {
        match self {
            Shape::NumericRange => "numeric_range",
            other => other.upstream(),
        }
    }

    pub fn from_upstream(name: &str) -> Option<Shape> {
        Shape::ALL.into_iter().find(|s| s.upstream() == name)
    }

    pub fn from_value(value: &str) -> Option<Shape> {
        Shape::ALL.into_iter().find(|s| s.value() == value)
    }
}

/// Upstream's word-boundary cues, each the source text between the regex literal's slashes (its flag is `i`).
pub const SHAPE_CUES: [(Shape, &str); 5] = [
    (
        Shape::NumericRange,
        r"\b(round(ing|ed)?|threshold|max(imum)?|min(imum)?|limit|bound(ary)?|between|cap|percent|amount|price|count|number|score|rate|decimal)\b",
    ),
    (
        Shape::Collection,
        r"\b(lists?|arrays?|sets?|items?|collections?|each|every|all|sort(ed|ing)?|merge|dedupe|group|ranges?|intervals?|overlap(ping)?)\b",
    ),
    (
        Shape::Text,
        r"\b(string|text|names?|labels?|truncate|substring|char(acter)?s?|length|slug|message|unicode)\b",
    ),
    (
        Shape::Stateful,
        r"\b(save|persist|store|update|toggle|create|delete|remove|submit|retry|apply|register|insert)\b",
    ),
    (
        Shape::Io,
        r"\b(files?|requests?|fetch|upload|download|network|api|endpoints?|connections?|sockets?)\b",
    ),
];

/// One category of the taxonomy: the shapes that raise it and the question it asks.
#[derive(Debug, PartialEq)]
pub struct Category {
    pub id: &'static str,
    pub name: &'static str,
    pub shapes: &'static [Shape],
    pub probe: &'static str,
}

/// Upstream's closed taxonomy of eight edge categories, in its order, with its text.
pub const TAXONOMY: [Category; 8] = [
    Category {
        id: "boundary",
        name: "Boundary values",
        shapes: &[Shape::NumericRange],
        probe: "What happens exactly at each min/max/threshold — and one step either side?",
    },
    Category {
        id: "adjacency",
        name: "Adjacency / touching",
        shapes: &[Shape::Collection],
        probe: "When two things are exactly equal or just touch, do they merge, collide, or separate?",
    },
    Category {
        id: "empty",
        name: "Empty / degenerate",
        shapes: &[Shape::Collection, Shape::Text],
        probe: "What is the result for empty, single-element, or null input?",
    },
    Category {
        id: "encoding",
        name: "Encoding / representation",
        shapes: &[Shape::Text],
        probe: "Whose definition of length/equality applies — bytes, code points, grapheme clusters, or normalized form?",
    },
    Category {
        id: "ordering",
        name: "Ordering / stability",
        shapes: &[Shape::Collection],
        probe: "When elements compare equal, is output order specified and stable?",
    },
    Category {
        id: "precision",
        name: "Precision / overflow",
        shapes: &[Shape::NumericRange],
        probe: "Where can precision loss, overflow, or rounding/tie-breaking occur — and what is the exact contract (e.g. half-up vs half-to-even, ceil/floor/truncate)?",
    },
    Category {
        id: "idempotency",
        name: "Idempotency / repetition",
        shapes: &[Shape::Stateful],
        probe: "What happens if this runs twice on the same input?",
    },
    Category {
        id: "concurrency",
        name: "Concurrency / effect ordering",
        shapes: &[Shape::Stateful, Shape::Io],
        probe: "If interrupted or run in parallel, what is guaranteed?",
    },
];

/// The soft "review manually" row of a requirement whose prose matched no cue: a category of an item, not of the
/// taxonomy.
pub const UNCLASSIFIED: &str = "unclassified";
pub const UNCLASSIFIED_PROBE: &str = "unclassified — review manually";

/// The edge probe's vocabularies: the taxonomy and `unclassified`, the tiers `explicit | backstop`, each requiring a
/// non-blank `resolution`.
pub const EDGE_VALIDATORS: Validators = Validators {
    categories: &[
        "boundary",
        "adjacency",
        "empty",
        "encoding",
        "ordering",
        "precision",
        "idempotency",
        "concurrency",
        UNCLASSIFIED,
    ],
    verification: &["explicit", "backstop"],
    required: &[("explicit", &["resolution"]), ("backstop", &["resolution"])],
};

/// A requirement as the edge probe reads it. `shapes` holds upstream's names and overrides the prose; `Some(vec![])`
/// proposes nothing. `text_en`, an English translation, is classified instead of `text` when present.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EdgeRequirement {
    pub id: String,
    pub text: Option<String>,
    pub text_en: Option<String>,
    pub shapes: Option<Vec<String>>,
}

/// The shapes `text` shows, in [`Shape::ALL`] order.
pub fn classify_shape(text: &str) -> Vec<Shape> {
    static CUES: OnceLock<Vec<(Shape, Regex)>> = OnceLock::new();
    let cues = CUES.get_or_init(|| {
        SHAPE_CUES
            .iter()
            .map(|(shape, source)| (*shape, Regex::new(source)))
            .collect()
    });
    let chars: Vec<char> = text.chars().collect();
    cues.iter()
        .filter(|(_, re)| re.is_match(&chars))
        .map(|(shape, _)| *shape)
        .collect()
}

/// The ids of the categories any of `shapes` raises, in taxonomy order.
pub fn applicable_categories(shapes: &[Shape]) -> Vec<&'static str> {
    TAXONOMY
        .iter()
        .filter(|c| c.shapes.iter().any(|s| shapes.contains(s)))
        .map(|c| c.id)
        .collect()
}

/// The generic check, then the edge probe's: a text when no shapes override it, and a `text_en` that is not blank.
pub fn validate_requirement(r: &EdgeRequirement) -> Result<(), String> {
    super::validate_requirement(&r.id, r.text.as_deref())?;
    if r.shapes.is_none() && !filled(r.text.as_deref()) {
        return Err(format!(
            "requirement {} text must be a non-empty string when no shapes override is provided",
            r.id
        ));
    }
    if r.text_en.is_some() && !filled(r.text_en.as_deref()) {
        return Err(format!(
            "requirement {} text_en must be a non-empty string when present",
            r.id
        ));
    }
    Ok(())
}

fn unresolved(requirement: &str, category: &str, probe: &str) -> Item {
    Item {
        requirement_id: requirement.to_string(),
        category: category.to_string(),
        status: "unresolved".to_string(),
        verification: None,
        resolution: None,
        reason: None,
        probe: probe.to_string(),
    }
}

/// The candidate rows of `r`, every one unresolved: one per category its shapes raise (the authored shapes, else the
/// prose's), or the single `unclassified` row when its prose matched no cue.
pub fn propose_edges(r: &EdgeRequirement) -> Result<Vec<Item>, String> {
    validate_requirement(r)?;
    let shapes = match &r.shapes {
        Some(names) => {
            let mut shapes = Vec::with_capacity(names.len());
            for name in names {
                let shape = Shape::from_upstream(name).ok_or_else(|| {
                    let valid: Vec<&str> = Shape::ALL.iter().map(|s| s.upstream()).collect();
                    format!(
                        "invalid shape {} for requirement {} — must be one of: {}",
                        serde_json::to_string(name).unwrap_or_default(),
                        r.id,
                        valid.join(", ")
                    )
                })?;
                shapes.push(shape);
            }
            shapes
        }
        None => {
            let text = r.text_en.as_deref().or(r.text.as_deref()).unwrap_or("");
            let shapes = classify_shape(text);
            if shapes.is_empty() {
                return Ok(vec![unresolved(&r.id, UNCLASSIFIED, UNCLASSIFIED_PROBE)]);
            }
            shapes
        }
    };
    Ok(applicable_categories(&shapes)
        .into_iter()
        .map(|id| {
            let probe = TAXONOMY.iter().find(|c| c.id == id).map_or("", |c| c.probe);
            unresolved(&r.id, id, probe)
        })
        .collect())
}

/// Every requirement's candidate rows, in order. Refused: an invalid requirement and a requirement id given twice.
pub fn propose_all(reqs: &[EdgeRequirement]) -> Result<Vec<Item>, String> {
    let mut items = Vec::new();
    let mut seen: Vec<&str> = Vec::with_capacity(reqs.len());
    for r in reqs {
        validate_requirement(r)?;
        if seen.contains(&r.id.as_str()) {
            return Err(format!(
                "duplicate requirement id {}",
                serde_json::to_string(&r.id).unwrap_or_default()
            ));
        }
        seen.push(&r.id);
        items.extend(propose_edges(r)?);
    }
    Ok(items)
}

/// Proposes every requirement's rows ([`propose_all`]), then merges `resolutions` and rolls up through the engine.
#[allow(
    dead_code,
    reason = "upstream's export, run by its ported suite; the command adds agent-raised rows between its halves"
)]
pub fn analyze_coverage(
    reqs: &[EdgeRequirement],
    resolutions: &[Resolution],
) -> Result<CoverageReport, String> {
    super::analyze_coverage(propose_all(reqs)?, resolutions, &EDGE_VALIDATORS)
}

#[cfg(test)]
mod tests;
