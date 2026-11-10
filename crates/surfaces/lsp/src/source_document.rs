//! Source documents (ADR 0349 D7, D8): a Rust, TypeScript or JavaScript file under the project root, where a
//! collector placed facts. The server holds its buffer, to hash and place in, and never hands it to the session:
//! typing in a source file rebuilds nothing. A span in one is placed through its text (the open buffer, else the file
//! read inside the project root, `read_placed`) only while that text hashes to the digest each fact store placing
//! facts in it recorded at collect (`CollectedSources`); a stale file places nothing.

use std::path::Path;
use std::sync::Arc;

use specforge_common::{Diagnostic, SourceSpan, Sym};
use specforge_ops::authoring::rename::RenamePlan;
use specforge_ops::read::navigate::{CollectedSources, contains};
use specforge_ops::read::trace::route_traces;
use std::collections::BTreeMap;
use tower_lsp::jsonrpc::Error;
use tower_lsp::lsp_types::{CodeLens, Command, Location, Position, Range, Url};

use crate::LspState;
use crate::document::{LineIndex, Target};
use crate::navigation::Compiled;

/// The extensions of the files the server answers as source documents.
const EXTENSIONS: [&str; 5] = ["rs", "ts", "tsx", "js", "jsx"];

/// Whether `uri` is a source document: a `file:` URI of a `.rs`, `.ts`, `.tsx`, `.js` or `.jsx` file under the
/// served project's root. Never a spec source nor `specforge.json`, by extension.
pub fn is_source(state: &LspState, uri: &Url) -> bool {
    crate::uri::file_path(uri).is_some() && is_source_key(state, &state.key_of(uri))
}

/// [`is_source`] of the file session key `key` names.
pub(crate) fn is_source_key(state: &LspState, key: &str) -> bool {
    let source = Path::new(key)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e));
    source && state.view().root().is_some() && is_under_root(key)
}

/// Whether `key`, a session key, names a file under the project root: relative and not climbing out of it.
fn is_under_root(key: &str) -> bool {
    let key = Path::new(key);
    !key.is_absolute()
        && key
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}

/// The line index of `key`'s text, where `key` is a source file the session's facts are placed in: its open buffer,
/// else the file read inside the project root (`read_placed`: never through a link out, a second name or `..`), when
/// that text is the one each store placing facts in it recorded at collect (`CollectedSources::holds`). `None` for
/// any other file, and for a text that changed since or cannot be read.
pub(crate) fn placed_index(
    state: &LspState,
    collected: &CollectedSources,
    key: &str,
) -> Option<Arc<LineIndex>> {
    if let Some(document) = state.files().document(key) {
        return collected
            .holds(key, None, document.text().as_bytes())
            .then(|| Arc::clone(document.index()));
    }
    let bytes = specforge_ops::read::navigate::read_placed(&state.view(), key).ok()?;
    if !collected.holds(key, None, &bytes) {
        return None;
    }
    let text = String::from_utf8(bytes).ok()?;
    Some(Arc::new(LineIndex::new(&text)))
}

/// The references from inside a located fact of source document `key` at `position` (ADR 0349 D8), the innermost
/// whose place holds it: where a `.spec` cites the fact, and the name of each entity it is anchored to. `None` when
/// the cursor is in no fact, or the file is not the text collected.
pub(crate) fn fact_references(
    compiled: &Compiled,
    key: &str,
    position: Position,
) -> Option<Vec<Location>> {
    let index = compiled.source_index(key)?;
    let (line, col) = index.source_position(position)?;
    let at = SourceSpan {
        file: Sym::new(key),
        start_line: line,
        start_col: col,
        end_line: line,
        end_col: col,
    };
    let state = compiled.state();
    let view = state.view();
    let kinds = view.registries().kinds();
    let fact = state
        .graph()
        .nodes()
        .into_iter()
        .filter(|node| kinds.is_collected(node.kind.raw.as_str()))
        .filter_map(|node| {
            let place = node.located.iter().find(|place| contains(place, &at))?;
            Some((node, place.end_line - place.start_line))
        })
        .min_by_key(|(_, lines)| *lines)?
        .0;
    let navigator = compiled.navigator();
    let mut spans: Vec<SourceSpan> = navigator
        .tokens(fact.id.raw.as_str(), false)
        .unwrap_or_default()
        .into_iter()
        .map(|token| token.span)
        .collect();
    let anchors = specforge_ops::read::navigate::anchors_in(&view);
    for edge in anchors.of_fact(fact.id.raw.as_str()) {
        if let Ok(definition) = navigator.definition(edge.entity.id.raw.as_str()) {
            spans.push(definition.name);
        }
    }
    spans.sort_by_key(|s| (s.file, s.start_line, s.start_col));
    spans.dedup();
    Some(spans.iter().filter_map(|s| compiled.location(s)).collect())
}

/// The code lenses of source document `uri` (ADR 0349 D8): at the first line of each located fact placed in it with
/// an anchor, one per entity the fact is anchored to, `<kind> <id>` and its thread's summary when its kind is routed,
/// running [`crate::answers::FIND_REFERENCES`] at the fact's start (its citations and its entities). A file changed
/// since the collect has one lens at its start saying W198, and no other; a file no fact is placed in has none.
pub(crate) fn lenses(state: &LspState, uri: &Url) -> Option<Vec<CodeLens>> {
    let compiled = Compiled::new(state);
    let key = state.key_of(uri);
    if !compiled.placed(&key) {
        return None;
    }
    let Some(index) = compiled.source_index(&key) else {
        let notice = stale(state, &key);
        return Some(vec![CodeLens {
            range: Range::default(),
            command: Some(Command {
                title: format!("{}: {}", notice.code, notice.message),
                command: String::new(),
                arguments: None,
            }),
            data: None,
        }]);
    };
    let view = state.view();
    let anchors = specforge_ops::read::navigate::anchors_in(&view);
    // Each fact placed here, at its place in this file, with the entities it is anchored to, once each.
    let mut facts: BTreeMap<(usize, usize, &str), Vec<&specforge_graph::Node>> = BTreeMap::new();
    for edge in anchors.all() {
        let Some(place) = edge.fact.located.iter().find(|p| p.file.as_str() == key) else {
            continue;
        };
        let entities = facts
            .entry((place.start_line, place.start_col, edge.fact.id.raw.as_str()))
            .or_default();
        if !entities.iter().any(|e| e.id == edge.entity.id) {
            entities.push(edge.entity);
        }
    }
    let ids = facts.values().flatten().map(|e| e.id.raw.as_str());
    let threads: BTreeMap<String, String> = route_traces(&view, ids)
        .into_iter()
        .map(|t| (t.entity_id.clone(), t.summary()))
        .collect();
    let mut lenses = Vec::new();
    for ((line, col, _), entities) in facts {
        let start = index.position_at(line.saturating_sub(1), col.saturating_sub(1));
        let at = serde_json::json!({"line": start.line, "character": start.character});
        for entity in entities {
            let id = entity.id.raw.as_str();
            let mut title = format!("{} {id}", entity.kind.raw);
            if let Some(summary) = threads.get(id) {
                title = format!("{title} · {summary}");
            }
            lenses.push(CodeLens {
                range: Range::new(start, start),
                command: Some(Command {
                    title,
                    command: crate::answers::FIND_REFERENCES.to_string(),
                    arguments: Some(vec![serde_json::json!(uri), at.clone()]),
                }),
                data: None,
            });
        }
    }
    Some(lenses)
}

/// W198 of source file `key`, its text no longer the one collected: changed, or gone (no buffer, and no file read
/// inside the project root).
fn stale(state: &LspState, key: &str) -> Diagnostic {
    let gone = state.files().document(key).is_none()
        && specforge_ops::read::navigate::read_placed(&state.view(), key).is_err();
    specforge_ops::read::navigate::stale_notice(key, gone)
}

/// The entity a fact names at `position` of source document `key`, and the range of the name written there (ADR 0349
/// D8): what prepareRename and rename act on in a source file. `None` when the cursor is on no name a fact holds;
/// refused (`invalid_params`, W198's sentence) when the file changed since the collect.
pub(crate) fn renamed_at(
    compiled: &Compiled,
    key: &str,
    position: Position,
) -> tower_lsp::jsonrpc::Result<Option<(Sym, Range)>> {
    if compiled.placed(key) && compiled.source_index(key).is_none() {
        return Err(Error::invalid_params(stale(compiled.state(), key).message));
    }
    Ok(match compiled.target(key, position) {
        Some(Target::Entity { id, origin }) => Some((id, origin)),
        _ => None,
    })
}

/// Refused (`invalid_params`, W198's sentence) when a source file `plan` rewrites is open with a text other than the
/// one the rename planned on (the disk's, as collected): its edits would land elsewhere in what the editor holds.
pub(crate) fn rewrites_held_text(
    state: &LspState,
    plan: &RenamePlan,
) -> tower_lsp::jsonrpc::Result<()> {
    for (key, file) in plan.change().rewritten_sources() {
        if state
            .files()
            .document(key)
            .is_some_and(|document| document.text() != &*file.before)
        {
            return Err(Error::invalid_params(stale(state, key).message));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_relative_key_inside_the_root_is_under_it() {
        assert!(is_under_root("src/lib.rs"));
        assert!(!is_under_root("../other/lib.rs"));
        assert!(!is_under_root("/abs/lib.rs"));
        assert!(!is_under_root("src/../../lib.rs"));
    }
}
