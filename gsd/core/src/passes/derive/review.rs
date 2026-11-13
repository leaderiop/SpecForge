//! The computed values of a review round (`spec/plan.md`, "Reviews and revision conflicts"): a concern's status and a
//! round's open counts.
//!
//! A concern is `open` until a disposition names it; the disposition with the highest `round` decides (`addressed`,
//! `deferred`). A round counts its open concerns: `high` ones, and `actionable` ones that are neither high nor drift
//! (a drift finding is advisory, never a blocker).

use std::collections::BTreeMap;

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::passes::plans::{link, of_kind, text};

/// Each concern's status, by id: the outcome of the disposition naming it with the highest round, else `open`.
fn concern_statuses(input: &PassInput) -> BTreeMap<&str, &str> {
    let mut ruled: BTreeMap<&str, (u64, &str)> = BTreeMap::new();
    for disposition in of_kind(input, "disposition") {
        let (Some(concern), Some(outcome)) = (
            link(input, disposition, "concern"),
            text(disposition, "outcome"),
        ) else {
            continue;
        };
        let round = text(disposition, "round")
            .and_then(|r| r.parse().ok())
            .unwrap_or(0);
        let held = ruled.entry(concern).or_insert((round, outcome));
        if round >= held.0 {
            *held = (round, outcome);
        }
    }
    of_kind(input, "review_concern")
        .map(|c| {
            let status = ruled.get(c.id.as_str()).map_or("open", |(_, o)| *o);
            (c.id.as_str(), status)
        })
        .collect()
}

pub fn values(input: &PassInput) -> Vec<PassValue> {
    let statuses = concern_statuses(input);
    let mut out: Vec<PassValue> = statuses
        .iter()
        .map(|(id, status)| value(id, "status", *status))
        .collect();
    for review in of_kind(input, "review") {
        let open: Vec<&PassEntity> = of_kind(input, "review_concern")
            .filter(|c| link(input, c, "review") == Some(review.id.as_str()))
            .filter(|c| statuses.get(c.id.as_str()) == Some(&"open"))
            .collect();
        let flag = |c: &PassEntity, field: &str| text(c, field) == Some("true");
        let high = open
            .iter()
            .filter(|c| text(c, "severity") == Some("high"))
            .count();
        let actionable = open
            .iter()
            .filter(|c| flag(c, "actionable") && !flag(c, "drift"))
            .filter(|c| text(c, "severity") != Some("high"))
            .count();
        out.push(value(
            &review.id,
            "open",
            json!({ "high": high, "actionable": actionable }),
        ));
    }
    out
}

#[cfg(test)]
mod tests;
