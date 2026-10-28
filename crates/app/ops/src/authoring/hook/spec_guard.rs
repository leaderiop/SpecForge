//! `core.spec_guard` (ADR 0292, D-r33-10): the host's own hook, wired by every install on a host's write or edit of a
//! `.spec` file. The file as the write leaves it is judged like a hand edit: the project is compiled over that text
//! and compared with the project as it is. A computed value (E082), a harness value with no ledger line (E083) or a
//! person's own kind or field is denied, the reason naming the entity, the field and the tool refused; any other
//! error the edit adds is context, since an agent in the middle of an edit is often briefly wrong.

use std::collections::BTreeMap;

use serde_json::Value;
use specforge_common::{Diagnostic, Severity};
use specforge_protocol_types::{Decision, HookEvent};

use super::{Outcome, facts, filter::Filter};
use crate::authoring::apply::hand;
use crate::host::render::{SPEC_GUARD_PATHS, SPEC_GUARD_TOOLS, spec_root_spelled};
use crate::host::{HookAnswer, HostHookEvent};
use crate::view::ProjectView;

/// `old` replaced by `new` in `text`: once, or everywhere with `all`; `None` where the host refuses the edit itself
/// (no match, or several without `all`).
fn replaced(text: &str, old: &str, new: &str, all: bool) -> Option<String> {
    let found = if old.is_empty() {
        0
    } else {
        text.matches(old).count()
    };
    (found == 1 || (found > 1 && all)).then(|| text.replace(old, new))
}

/// The text of the file after the tool's write, given the `current` text (none: the file is not there).
fn after_text(input: &Value, current: Option<&str>) -> Option<String> {
    let edit = |text: &str, e: &Value| {
        replaced(
            text,
            e["old_string"].as_str()?,
            e["new_string"].as_str()?,
            e["replace_all"].as_bool().unwrap_or(false),
        )
    };
    if let Some(edits) = input["edits"].as_array() {
        return edits
            .iter()
            .try_fold(current?.to_string(), |text, e| edit(&text, e));
    }
    if input.get("old_string").is_some() {
        return edit(current?, input);
    }
    ["content", "contents", "text"]
        .iter()
        .find_map(|key| input[*key].as_str())
        .map(str::to_string)
}

/// The errors `after` reports that `before` did not, by code and message.
fn added(before: &[Diagnostic], after: &[Diagnostic]) -> Vec<Diagnostic> {
    after
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .filter(|d| {
            !before
                .iter()
                .any(|b| b.code == d.code && b.message == d.message)
        })
        .cloned()
        .collect()
}

pub(super) fn run(view: &ProjectView<'_>, event: &HostHookEvent) -> Outcome {
    let root = view.env().root();
    let cwd = event
        .cwd
        .as_deref()
        .map_or_else(|| root.to_path_buf(), Into::into);
    let filter = Filter {
        event: HookEvent::PreTool,
        tools: SPEC_GUARD_TOOLS.iter().map(|t| (*t).to_string()).collect(),
        paths: vec![spec_root_spelled(view, SPEC_GUARD_PATHS)],
    };
    let Some(tool) = event
        .tool
        .as_ref()
        .filter(|_| filter.matches(event, root, &cwd))
    else {
        return Outcome::default();
    };
    let Some(key) = facts::path_names(&tool.input)
        .iter()
        .find_map(|name| facts::relative_to(root, &cwd, name))
    else {
        return Outcome::default();
    };
    let current = view
        .containment()
        .resolve(&root.join(&key))
        .ok()
        .and_then(|held| held.read_if_there().ok().flatten())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
    // An edit that does not apply is the host's to refuse.
    let Some(text) = after_text(&tool.input, current.as_deref()) else {
        return Outcome::default();
    };
    let before = view.project();
    let after = before.edited(&BTreeMap::from([(key, text)]));
    let mut denied: Vec<String> = Vec::new();
    let new = added(&before.checked_diagnostics(), &after.checked_diagnostics());
    for d in new.iter().filter(|d| d.code == "E082" || d.code == "E083") {
        let hint = d
            .suggestion
            .as_deref()
            .unwrap_or("write it through its tool");
        denied.push(format!("{}: {hint}", d.message));
    }
    denied.extend(hand::human_changes(view, &ProjectView::of(&after)));
    let mut answer = HookAnswer::default();
    if denied.is_empty() {
        let others: Vec<String> = new
            .iter()
            .map(|d| format!("{} {}", d.code, d.message))
            .collect();
        if !others.is_empty() {
            answer.context = Some(format!(
                "this write leaves {} error(s) in the project: {}",
                others.len(),
                others.join("; ")
            ));
        }
    } else {
        answer.decision = Some(Decision::Deny);
        answer.reason = Some(format!(
            "the {} tool may not write this: {}",
            tool.name,
            denied.join("; ")
        ));
    }
    Outcome {
        answer,
        called: false,
        ..Outcome::default()
    }
}
