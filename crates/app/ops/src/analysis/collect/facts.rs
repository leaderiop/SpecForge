//! A collector's facts (ADR 0327): admitted per collector (E116), their places keyed from the root and their files
//! digested (ADR 0346 D2), and keyed once per collect (I209). The caller stores them
//! (`authoring::apply::apply_collected`): a write of the project is an `authoring` operation.

use std::collections::BTreeMap;
use std::path::Path;

use specforge_common::{Diagnostic, codes};
use specforge_project::facts::{self, admit};
use specforge_protocol_types::{Fact, FactLocation};

use super::Collector;
use crate::source_key::SourceKey;
use crate::view::ProjectView;

/// Admitted facts, their places keyed, and the digest of each file a place names, by key.
type Placed = (Vec<Fact>, BTreeMap<String, String>);

/// The facts `collector` answered that the host stores, each place keyed from the project `root`, and the digest of
/// every file a place names: all of them, or none with E116 naming the first reason.
pub(super) fn admitted(
    view: &ProjectView,
    root: &Path,
    collector: &Collector,
    facts: Vec<Fact>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Placed {
    if facts.is_empty() {
        return (facts, BTreeMap::new());
    }
    let admitted = admit(&facts, view.registries().kinds(), &collector.answers)
        .and_then(|()| placed(view, root, facts));
    match admitted {
        Ok(placed) => placed,
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
                .with_suggestion("fix the collector: each fact is of a kind a collector writes and it lists in `answers`, its id in one of the kind's schemes, its places files under the project root"),
            );
            (Vec::new(), BTreeMap::new())
        }
    }
}

/// `facts` with every place's file keyed from the project `root` (ADR 0271: the key replaces the written spelling),
/// and the digest (`sha256:<hex>`) of each distinct file, read once through the view's containment (ADR 0346 D2). A
/// place that is no key under the root, or names no regular file there, is a reason.
fn placed(view: &ProjectView, root: &Path, mut facts: Vec<Fact>) -> Result<Placed, Vec<String>> {
    let mut sources = BTreeMap::new();
    let mut reasons = Vec::new();
    for fact in &mut facts {
        let places = fact.at.iter_mut().chain(
            fact.named_at
                .values_mut()
                .flat_map(|ids| ids.values_mut().flatten()),
        );
        for place in places {
            match keyed(view, root, place, &mut sources) {
                Some(key) => place.file = key,
                None => reasons.push(format!(
                    "fact '{}' is placed at '{}', which is no file under the project root",
                    fact.id, place.file
                )),
            }
        }
    }
    if reasons.is_empty() {
        Ok((facts, sources))
    } else {
        Err(reasons)
    }
}

/// The key of `place`'s file under `root`, its digest recorded in `sources` the first time; `None` when it is no key
/// under the root or names no regular file there.
fn keyed(
    view: &ProjectView,
    root: &Path,
    place: &FactLocation,
    sources: &mut BTreeMap<String, String>,
) -> Option<String> {
    let key = SourceKey::read(root, &place.file)
        .ok()?
        .as_str()
        .to_string();
    if !sources.contains_key(&key) {
        let (_, bytes) = view.containment().read(&root.join(&key)).ok()?;
        sources.insert(key.clone(), crate::revision::of_bytes(&bytes));
    }
    Some(key)
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
