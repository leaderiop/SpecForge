//! E533: a breaking API change the milestone's version does not allow (ADR 0390 D5), and E535: release evidence that
//! does not match the milestone ([`e535`], D6). Both share the baseline, [`last_shipped`].
//!
//! At a gate on an unshipped milestone whose computed `api_breaks` is not empty, the baseline is the last shipped
//! other milestone of the same workstream scope whose name is a version (latest by `shipped`, ties by the higher
//! version); a shipped milestone whose name is not one (`mvp`, `v1.7-rc1`) is skipped by both guards. No baseline is a
//! first release: nothing to refuse. A subject name that is not a version fails closed (one error). When
//! the subject's version is API-compatible with the baseline's (Cargo's rule, [`crate::release::compatible`]), each
//! break is one error, at most [`ERRORS`], then one summary error. The guard reads the computed field alone, never the
//! `api_item` facts (ADR 0353), so it judges the facts of the last `specforge collect`. Every echoed id is [`quoted`].

use serde_json::Value;
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

fn version(milestone: &PassEntity) -> Option<Version<'_>> {
    parse(name_of(&milestone.id))
}

/// The first [`ERRORS`] lines of a computed list, and how many more it holds: past them, plus the count of the
/// `more <N>` line `derive` caps the field with (it counts, it names nothing).
fn capped(lines: &[String]) -> (Vec<&str>, usize) {
    let mut more = 0;
    let mut named = Vec::new();
    for line in lines {
        match line
            .strip_prefix("more ")
            .and_then(|n| n.parse::<usize>().ok())
        {
            Some(n) => more += n,
            None => named.push(line.as_str()),
        }
    }
    more += named.len().saturating_sub(ERRORS);
    named.truncate(ERRORS);
    (named, more)
}

/// The last shipped milestone of `milestone`'s workstream scope whose name is a version, itself excluded: latest by
/// `shipped`, ties by the higher version, with that version. A shipped `mvp` or `v1.7-rc1` is no release to compare
/// with: skipped.
fn last_shipped<'a>(
    input: &'a PassInput,
    milestone: &PassEntity,
) -> Option<(&'a PassEntity, Version<'a>)> {
    let scope = link(input, milestone, "workstream");
    of_kind(input, "milestone")
        .filter(|m| m.id != milestone.id && shipped(m) && link(input, m, "workstream") == scope)
        .filter_map(|m| Some((text(m, "shipped"), version(m)?, m)))
        .max_by(|(at_a, a, _), (at_b, b, _)| (at_a, a).cmp(&(at_b, b)))
        .map(|(_, v, m)| (m, v))
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
    let Some((baseline, older)) = last_shipped(input, milestone) else {
        return Vec::new();
    };
    let named = quoted(subject);
    let last = quoted(&baseline.id);
    let Some(newer) = version(milestone) else {
        return vec![refuse(
            "E533",
            PassSeverity::Error,
            subject,
            format!(
                "milestone {named} has API breaks and no major version bump over the last shipped {last} can be \
                 shown: {named} is not a version"
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
    let (items, more) = capped(&breaks);
    let mut found: Vec<PassDiagnostic> = items
        .into_iter()
        .map(|line| line.split_once(' ').unwrap_or(("changed", line)))
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

/// A text member of a milestone's computed `release`.
fn release_of<'a>(milestone: &'a PassEntity, member: &str) -> Option<&'a str> {
    milestone.values.get("release")?.get(member)?.as_str()
}

/// E535: release evidence that does not match the milestone (ADR 0390 D6). At a gate on a milestone whose name is a
/// version: one error when the recorded tag of the subject once shipped, else of the last shipped milestone of its
/// scope ([`last_shipped`]), resolves to no commit; while the subject is unshipped, one error per line of its computed
/// `release.packages` (a package not at its version), at most [`ERRORS`] then one summary, and one error when its
/// version is not later than the last shipped one. Absent evidence is no observation, never a failure.
pub(super) fn e535(input: &PassInput) -> Vec<PassDiagnostic> {
    let Some(subject) = gate(input).flatten() else {
        return Vec::new();
    };
    let Some(milestone) = of_kind(input, "milestone").find(|m| m.id == subject) else {
        return Vec::new();
    };
    let Some(own) = version(milestone) else {
        return Vec::new();
    };
    let named = quoted(subject);
    let mut found = Vec::new();
    let baseline = last_shipped(input, milestone);
    // GSD tags after the close: the release judged is the subject once shipped, else the last one before it.
    let released = if shipped(milestone) {
        Some(milestone)
    } else {
        baseline.map(|(last, _)| last)
    };
    if let Some(released) = released
        && let Some(tag) = release_of(released, "tag")
        && release_of(released, "commit").is_none()
    {
        found.push(refuse(
            "E535",
            PassSeverity::Error,
            subject,
            format!(
                "the tag {} of the shipped milestone {} resolves to no commit",
                quoted(tag),
                quoted(&released.id)
            ),
            "create and push the tag, then run `specforge collect` again: the tag is observed only at collect",
        ));
    }
    if shipped(milestone) {
        return found;
    }
    if let Some((last, _)) = baseline.filter(|(_, older)| own <= *older) {
        let last = quoted(&last.id);
        found.push(refuse(
            "E535",
            PassSeverity::Error,
            subject,
            format!("milestone {named} goes backwards from the last shipped {last}: its version is not later"),
            &format!("name the milestone by a version later than the last shipped {last}"),
        ));
    }
    let packages: Vec<String> = milestone
        .values
        .get("release")
        .and_then(|r| r.get("packages"))
        .and_then(Value::as_array)
        .map(|lines| {
            lines
                .iter()
                .filter_map(|l| l.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let how = "set the package's version to the milestone's, or list the package in `specforge.release_exempt` when \
               the milestone does not release it; then run `specforge collect` again: this check reads the package \
               versions of the last collect";
    let (lines, more) = capped(&packages);
    for line in lines {
        let (name, at) = line.split_once(' ').unwrap_or((line, ""));
        found.push(refuse(
            "E535",
            PassSeverity::Error,
            subject,
            format!(
                "package {} is at version {}, not at milestone {named}'s version",
                quoted(name),
                quoted(at)
            ),
            how,
        ));
    }
    if more > 0 {
        found.push(refuse(
            "E535",
            PassSeverity::Error,
            subject,
            format!("{more} more packages are not at milestone {named}'s version"),
            how,
        ));
    }
    found
}

#[cfg(test)]
mod tests;
