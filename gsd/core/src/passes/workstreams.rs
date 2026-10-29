//! `gsd/E517`: active plans of different workstreams share no path in `files_modified ∪ files_deleted`. A plan is
//! active until its summary says when it `completed`; a plan outside any workstream is not judged. The setting
//! `specforge.cross_workstream_conflicts` (on in gsd-plus) gates the code.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::plans::{link, of_kind, strings};

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("workstreams", |p| {
        p.phase("check").run(check);
    });
}

/// The paths a plan touches, trimmed.
fn touched(plan: &PassEntity) -> BTreeSet<String> {
    ["files_modified", "files_deleted"]
        .into_iter()
        .flat_map(|field| strings(plan, field))
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// The workstream a plan belongs to: its phase's milestone's workstream.
fn workstream_of<'a>(input: &'a PassInput, plan: &PassEntity) -> Option<&'a str> {
    let phase =
        of_kind(input, "phase").find(|p| link(input, plan, "phase") == Some(p.id.as_str()))?;
    let milestone = of_kind(input, "milestone")
        .find(|m| link(input, phase, "milestone") == Some(m.id.as_str()))?;
    link(input, milestone, "workstream")
}

fn active(input: &PassInput, plan: &PassEntity) -> bool {
    !of_kind(input, "summary").any(|s| {
        link(input, s, "plan") == Some(plan.id.as_str()) && s.fields.contains_key("completed")
    })
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut plans: Vec<(&PassEntity, &str, BTreeSet<String>)> = of_kind(input, "plan")
        .filter(|p| active(input, p))
        .filter_map(|p| Some((p, workstream_of(input, p)?, touched(p))))
        .collect();
    plans.sort_by(|a, b| a.0.id.cmp(&b.0.id));
    let mut found = Vec::new();
    for (at, (later, stream, paths)) in plans.iter().enumerate() {
        for (earlier, other_stream, other_paths) in &plans[..at] {
            if stream == other_stream {
                continue;
            }
            let shared: Vec<&String> = paths.intersection(other_paths).collect();
            if shared.is_empty() {
                continue;
            }
            found.push(
                PassDiagnostic::new(
                    "E517",
                    PassSeverity::Error,
                    format!(
                        "active plans '{}' ({other_stream}) and '{}' ({stream}) both touch {}",
                        earlier.id,
                        later.id,
                        shared
                            .iter()
                            .map(|p| format!("'{p}'"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                )
                .with_suggestion(
                    "finish one plan before the other starts, or keep the file in one workstream",
                )
                .with_entity(&later.id),
            );
        }
    }
    found
}
