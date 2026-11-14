//! Ported from gsd-core v1.19.0 src/probe-core.cts, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The probe engine's generic half: a probe row's resolution lifecycle (`status` × `verification`), the check of a
//! resolution against a probe's vocabularies, and the merge of resolutions onto proposed rows with its coverage
//! roll-up. Each probe is an adapter: the edge probe ([`edge`]) proposes its rows from a requirement's shapes, and the
//! prohibition probe has only its vocabulary here ([`PROHIBITION_VALIDATORS`]). Pure: no I/O, no graph. Every refusal
//! is upstream's message, word for word.
//!
//! gsd-core modules this file ports: probe-core.cjs.
//!
//! Left out, and where each lives: `runProbeCli` (the JSON-file CLI) is `specforge gsd edge-probe`;
//! `projectProhibitions` and `dispositionForProhibition` are the planner's lift into `prohibition` and the verifier;
//! the truth helpers (`truthStatement`, `truthVerification`, `dispositionForUnverifiableTruth`, `projectTruths`) are
//! `gsd/E501` and `gsd/E502`; `src/spec-section.cts` (its suite `tests/spec-section.test.cjs`) is `specforge query`
//! counting a spec's `edge` and `spec_prohibition` entities. The checks of a value's JSON type (a non-string text, a
//! non-array list) have no Rust twin: the types refuse them.

pub mod edge;

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::js_regex::js_space;

/// The resolution lifecycle every probe shares.
pub const VALID_STATUS: [&str; 3] = ["resolved", "dismissed", "unresolved"];

/// A probe's closed vocabularies, which the engine enforces: its categories, the verification tiers of a resolved row,
/// and the fields (`resolution`, `reason`) each tier requires to be non-blank.
pub struct Validators {
    pub categories: &'static [&'static str],
    pub verification: &'static [&'static str],
    pub required: &'static [(&'static str, &'static [&'static str])],
}

/// The prohibition probe's vocabulary: no closed category set, the tiers `test | judgment`, and no field a tier
/// requires (a resolved prohibition's content is its statement).
pub const PROHIBITION_VALIDATORS: Validators = Validators {
    categories: &[],
    verification: &["test", "judgment"],
    required: &[("test", &[]), ("judgment", &[])],
};

/// A proposed or resolved row of a requirement and a category.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub requirement_id: String,
    pub category: String,
    pub status: String,
    pub verification: Option<String>,
    pub resolution: Option<String>,
    pub reason: Option<String>,
    pub probe: String,
}

/// An author's resolution, merged onto the proposed row of its `(requirement_id, category)`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Resolution {
    pub requirement_id: String,
    pub category: String,
    pub status: String,
    pub verification: Option<String>,
    pub resolution: Option<String>,
    pub reason: Option<String>,
}

impl From<&Item> for Resolution {
    fn from(item: &Item) -> Resolution {
        Resolution {
            requirement_id: item.requirement_id.clone(),
            category: item.category.clone(),
            status: item.status.clone(),
            verification: item.verification.clone(),
            resolution: item.resolution.clone(),
            reason: item.reason.clone(),
        }
    }
}

/// The roll-up: `resolved` is the closed set (`resolved` and `dismissed`), `applicable - unresolved`; `unclassified`
/// counts the `unclassified` rows among the applicable; `by_verification` counts the resolved rows of each tier.
#[derive(Clone, Debug, PartialEq)]
pub struct Coverage {
    pub applicable: usize,
    pub resolved: usize,
    pub unresolved: usize,
    pub unclassified: usize,
    pub by_verification: BTreeMap<String, usize>,
}

/// The merged rows and their roll-up.
#[derive(Clone, Debug, PartialEq)]
pub struct CoverageReport {
    pub items: Vec<Item>,
    pub coverage: Coverage,
}

impl Item {
    /// Upstream's JSON of an item: its seven keys, an absent value as `null`.
    pub fn to_json(&self) -> Value {
        json!({
            "requirement_id": self.requirement_id,
            "category": self.category,
            "status": self.status,
            "verification": self.verification,
            "resolution": self.resolution,
            "reason": self.reason,
            "probe": self.probe,
        })
    }
}

impl Coverage {
    /// Upstream's JSON of the roll-up, `by_verification` as `byVerification`.
    pub fn to_json(&self) -> Value {
        json!({
            "applicable": self.applicable,
            "resolved": self.resolved,
            "unresolved": self.unresolved,
            "unclassified": self.unclassified,
            "byVerification": self.by_verification,
        })
    }
}

impl CoverageReport {
    /// Upstream's report: `{items, coverage}`.
    pub fn to_json(&self) -> Value {
        json!({
            "items": self.items.iter().map(Item::to_json).collect::<Vec<_>>(),
            "coverage": self.coverage.to_json(),
        })
    }
}

/// Whether `text` holds more than JavaScript's `trim()` strips.
pub(crate) fn filled(text: Option<&str>) -> bool {
    text.is_some_and(|t| !t.trim_matches(js_space).is_empty())
}

/// A requirement's generic check: its id is a non-empty string.
pub fn validate_requirement(id: &str, _text: Option<&str>) -> Result<(), String> {
    if id.trim_matches(js_space).is_empty() {
        let got = serde_json::to_string(id).unwrap_or_default();
        return Err(format!(
            "requirement id must be a non-empty string (got {got})"
        ));
    }
    Ok(())
}

/// A resolution against the probe's vocabularies: a valid status, a tier only when resolved, no payload when
/// unresolved, a reason when dismissed, and a resolved row's valid tier with the fields that tier requires.
pub fn validate_resolution(r: &Resolution, v: &Validators) -> Result<(), String> {
    let key = format!("{}::{}", r.requirement_id, r.category);
    if !VALID_STATUS.contains(&r.status.as_str()) {
        return Err(format!("invalid status \"{}\" for {key}", r.status));
    }
    if r.status != "resolved"
        && let Some(tier) = &r.verification
    {
        return Err(format!(
            "verification must be null unless status is \"resolved\" (got \"{tier}\") for {key}"
        ));
    }
    if r.status == "unresolved" {
        if filled(r.resolution.as_deref()) {
            return Err(format!("unresolved must not carry a resolution ({key})"));
        }
        if filled(r.reason.as_deref()) {
            return Err(format!("unresolved must not carry a reason ({key})"));
        }
    }
    if r.status == "dismissed" && !filled(r.reason.as_deref()) {
        return Err(format!("dismissed requires a reason ({key})"));
    }
    if r.status == "resolved" {
        let tiers = v.verification.join(", ");
        let Some(tier) = r.verification.as_deref() else {
            return Err(format!(
                "resolved requires a verification tier (one of: {tiers}) for {key}"
            ));
        };
        if !v.verification.contains(&tier) {
            return Err(format!(
                "invalid verification \"{tier}\" for {key} — must be one of: {tiers}"
            ));
        }
        let required = v
            .required
            .iter()
            .find(|(t, _)| *t == tier)
            .map_or(&[][..], |(_, fields)| *fields);
        for field in required {
            let value = match *field {
                "resolution" => r.resolution.as_deref(),
                _ => r.reason.as_deref(),
            };
            if !filled(value) {
                return Err(format!("{tier} requires a {field} ({key})"));
            }
        }
    }
    Ok(())
}

/// A prohibition's resolution against [`PROHIBITION_VALIDATORS`].
pub fn validate_prohibition_resolution(r: &Resolution) -> Result<(), String> {
    validate_resolution(r, &PROHIBITION_VALIDATORS)
}

/// Merges `resolutions` onto the proposed `items` and rolls up their coverage. Refused: an invalid resolution, a second
/// resolution of one row, an item outside the probe's categories, an unresolved item that is not valid as it stands,
/// and a resolution that matches no item (an orphan).
pub fn analyze_coverage(
    items: Vec<Item>,
    resolutions: &[Resolution],
    v: &Validators,
) -> Result<CoverageReport, String> {
    let key = |req: &str, cat: &str| format!("{req}::{cat}");
    let mut by_key: Vec<(String, &Resolution)> = Vec::new();
    for r in resolutions {
        validate_resolution(r, v)?;
        let k = key(&r.requirement_id, &r.category);
        if by_key.iter().any(|(seen, _)| *seen == k) {
            return Err(format!("duplicate resolution for {k}"));
        }
        by_key.push((k, r));
    }
    let mut merged = Vec::with_capacity(items.len());
    let mut item_keys = Vec::with_capacity(items.len());
    for item in items {
        let k = key(&item.requirement_id, &item.category);
        if !v.categories.contains(&item.category.as_str()) {
            return Err(format!(
                "item {k} has unknown category \"{}\" — not one of: {}",
                item.category,
                v.categories.join(", ")
            ));
        }
        match by_key.iter().find(|(seen, _)| *seen == k) {
            Some((_, o)) => merged.push(Item {
                status: o.status.clone(),
                verification: o.verification.clone(),
                resolution: o.resolution.clone(),
                reason: o.reason.clone(),
                ..item
            }),
            None => {
                validate_resolution(&Resolution::from(&item), v)?;
                merged.push(item);
            }
        }
        item_keys.push(k);
    }
    if let Some((k, _)) = by_key.iter().find(|(k, _)| !item_keys.contains(k)) {
        return Err(format!(
            "unknown resolution for {k} — no matching proposed item (typo'd category or non-applicable shape?)"
        ));
    }
    let unresolved = merged.iter().filter(|i| i.status == "unresolved").count();
    let applicable = merged.len();
    let unclassified = merged
        .iter()
        .filter(|i| i.category == "unclassified")
        .count();
    let mut by_verification: BTreeMap<String, usize> = v
        .verification
        .iter()
        .map(|tier| (tier.to_string(), 0))
        .collect();
    for i in &merged {
        if let (true, Some(tier)) = (i.status == "resolved", &i.verification) {
            *by_verification.entry(tier.clone()).or_default() += 1;
        }
    }
    Ok(CoverageReport {
        items: merged,
        coverage: Coverage {
            applicable,
            resolved: applicable - unresolved,
            unresolved,
            unclassified,
            by_verification,
        },
    })
}

#[cfg(test)]
mod tests;
