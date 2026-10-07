//! Delivery evidence: what the recorded tests say about a feature, beside
//! the `status` its author declares.
//!
//! A feature's `status` is a claim; its evidence is derived. The behaviors
//! that implement a feature are the `behavior` entities whose `features`
//! field names it (the software extension's kind; without it a feature has
//! no implementers and nothing proves it). A feature is **proven** when at
//! least one behavior implements it and the coverage rule counts every one
//! of them proven: at least one obligation, every obligation named by a
//! passing test, no failing test (ADR 0004 D2-a, ADR 0039).
//!
//! The host scores each entity once ([`EntityEvidence`], in a command's
//! input); an analyze pass scores the same snapshot with the same rule
//! (`specforge-coverage`). This module only aggregates.

use serde::Serialize;
use specforge_extension_sdk::prelude::{CommandEvidence, CommandGraph, EntityEvidence};

/// The kind whose entities implement features, and the field they name
/// them in.
pub const IMPLEMENTER_KIND: &str = "behavior";
pub const IMPLEMENTS_FIELD: &str = "features";

/// What the recorded tests prove of one feature.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FeatureEvidence {
    pub feature_id: String,
    /// Behaviors that implement it.
    pub behaviors: usize,
    /// Of those, the ones the coverage rule counts proven.
    pub proven_behaviors: usize,
    /// Obligations the implementing behaviors declare, and how many a
    /// passing test names.
    pub obligations: usize,
    pub proven_obligations: usize,
    /// Recorded tests of the implementing behaviors that did not pass.
    pub failing: usize,
    /// At least one implementer, and every one proven.
    pub proven: bool,
}

impl FeatureEvidence {
    /// The evidence of `feature_id` from its implementers' scores; an
    /// implementer `score` has no entry for counts as unproven, with no
    /// obligations.
    pub fn of(
        feature_id: &str,
        implementers: &[String],
        score: impl Fn(&str) -> Option<EntityEvidence>,
    ) -> Self {
        let mut evidence = FeatureEvidence {
            feature_id: feature_id.to_string(),
            behaviors: implementers.len(),
            ..FeatureEvidence::default()
        };
        for id in implementers {
            let scored = score(id).unwrap_or_default();
            evidence.obligations += scored.obligations;
            evidence.proven_obligations += scored.proven;
            evidence.failing += scored.failing;
            if scored.is_proven() {
                evidence.proven_behaviors += 1;
            }
        }
        evidence.proven = evidence.behaviors > 0 && evidence.proven_behaviors == evidence.behaviors;
        evidence
    }
}

/// The behaviors of `graph` that implement `feature_id`, sorted.
pub fn implementers(graph: &CommandGraph, feature_id: &str) -> Vec<String> {
    let mut ids: Vec<String> = graph
        .edges_to(feature_id)
        .into_iter()
        .filter(|e| e.label == IMPLEMENTS_FIELD)
        .filter(|e| {
            graph
                .node(&e.source)
                .is_some_and(|n| n.kind == IMPLEMENTER_KIND)
        })
        .map(|e| e.source.clone())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// How a command's input stands on evidence, for a payload: `recorded`,
/// `none` (no report recorded: run `specforge collect`) or `unreadable`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceState {
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl EvidenceState {
    pub fn of(evidence: &CommandEvidence) -> Self {
        match evidence {
            CommandEvidence::None => EvidenceState {
                state: "none",
                reason: None,
            },
            CommandEvidence::Recorded { .. } => EvidenceState {
                state: "recorded",
                reason: None,
            },
            CommandEvidence::Unreadable { reason } => EvidenceState {
                state: "unreadable",
                reason: Some(reason.clone()),
            },
        }
    }

    /// One line for a human layout.
    pub fn describe(&self) -> String {
        match (self.state, &self.reason) {
            ("recorded", _) => "recorded".to_string(),
            ("unreadable", Some(reason)) => format!("unreadable: {reason}"),
            _ => "none recorded (run `specforge collect`)".to_string(),
        }
    }
}

/// The evidence of each of `features` in `graph`, in order; `None` when
/// the input carries no scored report.
pub fn of_features(
    graph: &CommandGraph,
    features: &[String],
    evidence: &CommandEvidence,
) -> Option<Vec<FeatureEvidence>> {
    let scored = evidence.entities()?;
    Some(
        features
            .iter()
            .map(|f| FeatureEvidence::of(f, &implementers(graph, f), |id| scored.get(id).copied()))
            .collect(),
    )
}
