//! What the passes read from the recorded commits: each commit's plan, task, type, subject, time and files, from the
//! entity and its resolved `plan` and `task` edges.

use specforge_extension_sdk::prelude::*;

use crate::passes::plans::{of_kind, strings, text};

/// One recorded commit.
#[derive(Debug, Clone, Default)]
pub struct Commit {
    /// The entity id: `<plan>/<12 hex>`.
    pub id: String,
    pub subject: String,
    /// `feat`, `fix`, `test` ...
    pub kind: String,
    pub plan: Option<String>,
    pub task: Option<String>,
    /// `YYYY-MM-DDTHH:MM:SSZ`: sorts as time does.
    pub at: String,
    pub files: Vec<String>,
    pub created: Vec<String>,
    pub insertions: i64,
    pub deletions: i64,
}

pub fn target<'a>(input: &'a PassInput, source: &str, label: &str) -> Option<&'a str> {
    input
        .edges
        .iter()
        .find(|e| e.source == source && e.label == label)
        .map(|e| e.target.as_str())
}

fn number(entity: &PassEntity, field: &str) -> i64 {
    text(entity, field)
        .and_then(|t| t.parse().ok())
        .unwrap_or(0)
}

/// Every commit, oldest first: by time, and in the file's order for the same second (the collector writes the log
/// oldest first).
pub fn commits(input: &PassInput) -> Vec<Commit> {
    let mut found: Vec<(Commit, (String, usize))> = of_kind(input, "commit")
        .map(|e| {
            let place = e
                .span
                .as_ref()
                .map(|s| (s.file.clone(), s.start_line))
                .unwrap_or_default();
            (
                Commit {
                    id: e.id.clone(),
                    subject: text(e, "subject").unwrap_or_default().to_string(),
                    kind: text(e, "type").unwrap_or_default().to_string(),
                    plan: target(input, &e.id, "plan").map(String::from),
                    task: target(input, &e.id, "task").map(String::from),
                    at: text(e, "at").unwrap_or_default().to_string(),
                    files: strings(e, "files"),
                    created: strings(e, "created"),
                    insertions: number(e, "insertions"),
                    deletions: number(e, "deletions"),
                },
                place,
            )
        })
        .collect();
    found.sort_by(|(a, pa), (b, pb)| a.at.cmp(&b.at).then_with(|| pa.cmp(pb)));
    found.into_iter().map(|(commit, _)| commit).collect()
}
