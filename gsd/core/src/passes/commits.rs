//! What the passes read from the recorded commits: each commit's plan, task, how the task was found, type, subject,
//! time and files, from the entity and its resolved `plan` and `task` edges.

use specforge_extension_sdk::prelude::*;

use crate::collectors::LinkedBy;
use crate::passes::plans::{of_kind, strings, text};

#[cfg(test)]
mod tests;

/// One recorded commit.
#[derive(Debug, Clone, Default)]
pub struct Commit {
    /// The entity id: the first 12 hex digits of its hash (`commit` is in the project scope).
    pub id: String,
    pub subject: String,
    /// Its subject's type; `None` for a subject with no `type:` header or another type.
    pub kind: Option<CommitType>,
    pub plan: Option<String>,
    pub task: Option<String>,
    /// How `task` was found: its `Gsd-Task:` trailer or its subject's title; `None` with no task.
    pub linked_by: Option<LinkedBy>,
    /// `YYYY-MM-DDTHH:MM:SSZ`: sorts as time does.
    pub at: String,
    pub files: Vec<String>,
    pub created: Vec<String>,
    /// The code items its diff changed, as gsd-git wrote them; empty when it named none or left the field unwritten.
    pub items_changed: Vec<String>,
    pub insertions: i64,
    pub deletions: i64,
    /// The ids of the recorded commits of its plan it descends from; `None` when the collector wrote no ancestry.
    pub descends_from: Option<Vec<String>>,
}

/// A commit's type, as its subject's `type(scope):` header names it: the executor's commit types
/// (`gsd-executor.md`'s commit step).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitType {
    Feat,
    Fix,
    Test,
    Refactor,
    Perf,
    Style,
    Chore,
    Docs,
}

impl CommitType {
    /// The type `commit.type` holds; `None` for any other word.
    pub fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "feat" => Self::Feat,
            "fix" => Self::Fix,
            "test" => Self::Test,
            "refactor" => Self::Refactor,
            "perf" => Self::Perf,
            "style" => Self::Style,
            "chore" => Self::Chore,
            "docs" => Self::Docs,
            _ => return None,
        })
    }

    /// A task commit's type: every one but `docs`, a plan's metadata commit, which names no task by design.
    pub fn is_task(self) -> bool {
        self != Self::Docs
    }
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
                    kind: text(e, "type").and_then(CommitType::parse),
                    plan: target(input, &e.id, "plan").map(String::from),
                    task: target(input, &e.id, "task").map(String::from),
                    linked_by: text(e, "linked_by").and_then(LinkedBy::parse),
                    at: text(e, "at").unwrap_or_default().to_string(),
                    files: strings(e, "files"),
                    created: strings(e, "created"),
                    items_changed: strings(e, "items_changed"),
                    insertions: number(e, "insertions"),
                    deletions: number(e, "deletions"),
                    // A written empty list is `""`, never absent (protocol 1.1.0).
                    descends_from: e.fields.contains_key("descends_from").then(|| {
                        input
                            .edges
                            .iter()
                            .filter(|d| d.source == e.id && d.label == "descends_from")
                            .map(|d| d.target.clone())
                            .collect()
                    }),
                },
                place,
            )
        })
        .collect();
    found.sort_by(|(a, pa), (b, pb)| a.at.cmp(&b.at).then_with(|| pa.cmp(pb)));
    found.into_iter().map(|(commit, _)| commit).collect()
}
