//! What the derive pass reads from the recorded commits: each commit's plan, task, type, time and files, from the
//! entity and its resolved `plan` and `task` edges.

use specforge_extension_sdk::prelude::*;

use crate::passes::plans::{of_kind, strings, text};

/// One recorded commit.
#[derive(Debug, Clone, Default)]
pub struct Commit {
    pub plan: Option<String>,
    pub task: Option<String>,
    /// `YYYY-MM-DDTHH:MM:SSZ`: sorts as time does.
    pub at: String,
    pub files: Vec<String>,
    pub created: Vec<String>,
}

fn target<'a>(input: &'a PassInput, source: &str, label: &str) -> Option<&'a str> {
    input
        .edges
        .iter()
        .find(|e| e.source == source && e.label == label)
        .map(|e| e.target.as_str())
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
                    plan: target(input, &e.id, "plan").map(String::from),
                    task: target(input, &e.id, "task").map(String::from),
                    at: text(e, "at").unwrap_or_default().to_string(),
                    files: strings(e, "files"),
                    created: strings(e, "created"),
                },
                place,
            )
        })
        .collect();
    found.sort_by(|(a, pa), (b, pb)| a.at.cmp(&b.at).then_with(|| pa.cmp(pb)));
    found.into_iter().map(|(commit, _)| commit).collect()
}
