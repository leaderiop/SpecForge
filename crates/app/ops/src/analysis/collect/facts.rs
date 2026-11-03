//! A collector's facts (ADR 0327): admitted per collector (E116) and keyed once per collect (I209). The caller stores
//! them (`authoring::apply::apply_collected`): a write of the project is an `authoring` operation.

use specforge_common::{Diagnostic, codes};
use specforge_project::facts::{self, admit};
use specforge_protocol_types::Fact;

use super::Collector;
use crate::view::ProjectView;

/// The facts `collector` answered that the host stores: all of them, or none with E116 naming the first reason.
pub(super) fn admitted(
    view: &ProjectView,
    collector: &Collector,
    facts: Vec<Fact>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Fact> {
    if facts.is_empty() {
        return facts;
    }
    match admit(&facts, view.registries().kinds()) {
        Ok(()) => facts,
        Err(reasons) => {
            let more = match reasons.len() {
                1 => String::new(),
                n => format!(" (and {} more)", n - 1),
            };
            diagnostics.push(
                Diagnostic::new(
                    codes::E116,
                    format!(
                        "collector '{}' ({}) answered facts the host refuses, so none is stored: {}{more}",
                        collector.name, collector.extension, reasons[0]
                    ),
                )
                .with_suggestion("fix the collector: each fact is of a kind a collector writes, its id in one of the kind's schemes"),
            );
            Vec::new()
        }
    }
}

/// The key this collect files its facts under (ADR 0327 D3), with I209 when it is not a clean commit.
pub(super) fn key(view: &ProjectView, diagnostics: &mut Vec<Diagnostic>) -> String {
    let (key, reason) = facts::key(view.env());
    if let Some(reason) = reason {
        diagnostics.push(
            Diagnostic::new(
                codes::I209,
                format!("facts are filed under '{key}': {}", reason.says()),
            )
            .with_suggestion("commit, then collect again, to file them under the commit"),
        );
    }
    key
}
