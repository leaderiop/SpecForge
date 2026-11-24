//! E533: a breaking API change the milestone's version does not allow (ADR 0390 D5).
//!
//! At a gate on an unshipped milestone whose computed `api_breaks` is not empty, the baseline is the last shipped
//! other milestone of the same workstream scope (latest by `shipped`, ties by the higher version). No baseline is a
//! first release: nothing to refuse. A subject or baseline name that is not a version fails closed (one error). When
//! the subject's version is API-compatible with the baseline's (Cargo's rule, [`crate::release::compatible`]), each
//! break is one error, at most [`ERRORS`], then one summary error. The guard reads the computed field alone, never the
//! `api_item` facts (ADR 0353), so it judges the facts of the last `specforge collect`. Every echoed id is [`quoted`].

use specforge_extension_sdk::prelude::*;

use super::{gate, refuse};
use crate::passes::plans::{link, name_of, of_kind, strings, text};
use crate::passes::{one_line, quoted};
use crate::release::{Version, compatible, parse};

const ERRORS: usize = 25;
const COLLECT: &str =
    "then run `specforge collect` again: this check reads the API facts of the last collect";

fn shipped(milestone: &PassEntity) -> bool {
    milestone.fields.contains_key("completion")
}

fn version(milestone: &PassEntity) -> Option<Version> {
    parse(name_of(&milestone.id))
}

pub(super) fn e533(input: &PassInput) -> Vec<PassDiagnostic> {
    let Some(subject) = gate(input).flatten() else {
        return Vec::new();
    };
    let Some(milestone) = of_kind(input, "milestone").find(|m| m.id == subject) else {
        return Vec::new();
    };
    let breaks = strings(milestone, "api_breaks");
    if shipped(milestone) || breaks.is_empty() {
        return Vec::new();
    }
    let scope = link(input, milestone, "workstream");
    let Some(baseline) = of_kind(input, "milestone")
        .filter(|m| m.id != subject && shipped(m) && link(input, m, "workstream") == scope)
        .max_by(|a, b| (text(a, "shipped"), version(a)).cmp(&(text(b, "shipped"), version(b))))
    else {
        return Vec::new();
    };
    let named = quoted(subject);
    let last = quoted(&baseline.id);
    let (Some(newer), Some(older)) = (version(milestone), version(baseline)) else {
        let unversioned = if version(milestone).is_none() {
            &named
        } else {
            &last
        };
        return vec![refuse(
            "E533",
            PassSeverity::Error,
            subject,
            format!(
                "milestone {named} has API breaks and no major version bump over the last shipped {last} can be \
                 shown: {unversioned} is not a version"
            ),
            &format!(
                "name the milestone by its version (`v1.7`, `1.7` or `v1.7.2`), or restore the items, or commit the \
                 updated accepted report when the change is accepted; {COLLECT}"
            ),
        )];
    };
    if !compatible(&older, &newer) {
        return Vec::new();
    }
    let how = format!(
        "bump the milestone's major version (Cargo's rule: for 0.x the minor), or restore the item, or commit the \
         updated accepted report when the change is accepted; {COLLECT}"
    );
    // `derive` caps the field with a `more <N>` line: it counts, it names no item.
    let mut more = 0;
    let mut items = Vec::new();
    for line in &breaks {
        match line
            .strip_prefix("more ")
            .and_then(|n| n.parse::<usize>().ok())
        {
            Some(n) => more += n,
            None => items.push(line.split_once(' ').unwrap_or(("changed", line))),
        }
    }
    more += items.len().saturating_sub(ERRORS);
    let mut found: Vec<PassDiagnostic> = items
        .into_iter()
        .take(ERRORS)
        .map(|(change, id)| {
            refuse(
                "E533",
                PassSeverity::Error,
                subject,
                format!(
                    "API item {} is {} without a major version bump: milestone {named} is compatible with the last \
                     shipped {last}",
                    quoted(id),
                    one_line(change)
                ),
                &how,
            )
        })
        .collect();
    if more > 0 {
        found.push(refuse(
            "E533",
            PassSeverity::Error,
            subject,
            format!(
                "{more} more API items broken without a major version bump: milestone {named} is compatible with \
                 the last shipped {last}"
            ),
            &how,
        ));
    }
    found
}

#[cfg(test)]
mod tests;
