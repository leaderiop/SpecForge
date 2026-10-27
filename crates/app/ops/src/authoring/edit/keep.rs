//! A move or a create keeps every reference's target (ADR 0284): an edit never changes what an existing token names.
//! The texts as they were and as the edit left them are read twice, with the scopes as they were and as the edit leaves
//! them; a token that named an entity and names another or none afterwards is respelled to the shortest spelling that
//! still names it, and a move that would make a token naming nothing name the moved entity is refused.

use super::{EditField, Planner};
use crate::authoring::rename::CAPTURED;
use crate::read::navigate::{SourceText, TextEdit, apply_edits};
use crate::{OpError, OpErrorKind, Refusal};
use specforge_common::Sym;
use specforge_graph::{Form, Holder, Scopes, WrittenToken, written_tokens};
use specforge_parser::parse;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// A token's identity across the two readings: its file, its holder's name (its path may differ, as its scope resolves
/// differently), its field and form, and its place among the tokens of that holder's field.
type Key = (String, String, Sym, String, String, usize);

/// What a move or a create does to the project's scopes.
pub(super) struct Rekey<'a> {
    /// The scopes after the edit.
    pub after: Scopes,
    /// (old path, new path) of each entity a set or unset of a scope field moves; none for a create.
    pub moves: &'a [(String, String)],
    /// The scope field the edit itself writes, as (the moved entity's old path, field): its token is the edit.
    pub written: Option<(&'a str, &'a str)>,
    /// The path of the entity the edit creates, whose tokens have no counterpart.
    pub created: Option<&'a str>,
}

/// The text `token` writes in `text`.
fn written_text(source: &SourceText, text: &str, token: &WrittenToken) -> String {
    source
        .range(&token.written)
        .and_then(|range| text.get(range))
        .unwrap_or_default()
        .to_string()
}

/// The name a holder is declared with: the last name of its path.
fn holder_name(holder: Holder) -> String {
    match holder {
        Holder::Block(id) if !id.as_str().contains(':') => id
            .as_str()
            .rsplit('/')
            .next()
            .unwrap_or(id.as_str())
            .to_string(),
        other => other.to_string(),
    }
}

impl Planner<'_> {
    /// The text of every source as planned so far.
    pub(super) fn snapshot(&self) -> BTreeMap<String, String> {
        let mut keys: Vec<String> = self.texts.keys().cloned().collect();
        keys.extend(
            self.view
                .project()
                .source_keys()
                .into_iter()
                .map(String::from),
        );
        keys.into_iter()
            .filter_map(|key| Some((key.clone(), self.text(&key)?)))
            .collect()
    }

    // shortcut: the scopes before are the graph's, so an edit that follows another one of the same plan reads the
    // project as it was before the plan; upgrade by keeping the scopes with the planned texts.
    pub(super) fn keep_targets(
        &mut self,
        before: &BTreeMap<String, String>,
        rekey: &Rekey<'_>,
    ) -> Result<(), Refusal<EditField>> {
        let inputs = self.view.registries().graph_inputs();
        let scopes = self.view.graph().scopes();
        let moved = |path: &str| -> String {
            match rekey.moves.iter().find(|(old, _)| old == path) {
                Some((_, new)) => new.clone(),
                None => path.to_string(),
            }
        };
        let mut found: HashMap<Key, WrittenToken> = HashMap::new();
        for (key, text) in &self.snapshot() {
            let source = SourceText::new(Arc::from(text.as_str()));
            let mut seen: HashMap<(String, Sym, String, String), usize> = HashMap::new();
            for token in written_tokens(&parse(text, key), inputs, &rekey.after) {
                if token.form == Form::Qualifier
                    || rekey.created == Some(token.holder.to_string().as_str())
                {
                    continue;
                }
                let at = (
                    holder_name(token.holder),
                    token.field,
                    format!("{:?}", token.form),
                    written_text(&source, text, &token),
                );
                let n = seen.entry(at.clone()).or_default();
                found.insert((key.clone(), at.0, at.1, at.2, at.3, *n), token);
                *n += 1;
            }
        }
        let mut edits: BTreeMap<String, Vec<TextEdit>> = BTreeMap::new();
        let mut places = Vec::new();
        for (key, text) in before {
            let source = SourceText::new(Arc::from(text.as_str()));
            let mut seen: HashMap<(String, Sym, String, String), usize> = HashMap::new();
            for token in written_tokens(&parse(text, key), inputs, scopes) {
                if token.form == Form::Qualifier {
                    continue;
                }
                let at = (
                    holder_name(token.holder),
                    token.field,
                    format!("{:?}", token.form),
                    written_text(&source, text, &token),
                );
                let n = seen.entry(at.clone()).or_default();
                let index = *n;
                *n += 1;
                let written = rekey.written.is_some_and(|(holder, field)| {
                    token.holder.to_string() == holder && token.field.as_str() == field
                });
                let Some(now) = found.get(&(key.clone(), at.0, at.1, at.2, at.3, index)) else {
                    continue;
                };
                if written {
                    continue;
                }
                if !scopes.contains(token.target.as_str()) {
                    // A token naming nothing: a move may not make it name the moved entity.
                    let path = now.target.as_str();
                    if rekey.moves.iter().any(|(_, new)| new == path) && rekey.after.contains(path)
                    {
                        places.push(now.place());
                    }
                    continue;
                }
                let wanted = moved(token.target.as_str());
                if now.target.as_str() == wanted && rekey.after.contains(&wanted) {
                    continue;
                }
                let from = match now.holder {
                    Holder::Block(id)
                        if rekey.after.kind(id.as_str()).is_some_and(|kind| {
                            inputs.scopes.get(kind.as_str()).map(String::as_str)
                                != Some(now.field.as_str())
                        }) =>
                    {
                        Some(id)
                    }
                    _ => None,
                };
                // The graph's node is the one that changes, at the path it has once the edit is made.
                if let Holder::Block(id) = token.holder {
                    self.declared
                        .field(&moved(id.as_str()), token.field.as_str());
                }
                edits
                    .entry(now.written.file.to_string())
                    .or_default()
                    .push(TextEdit {
                        span: now.written.clone(),
                        new_text: rekey.after.spell(from, Sym::new(&wanted), &|_| true),
                    });
            }
        }
        if !places.is_empty() {
            places.sort();
            places.dedup();
            return Err(OpError::new(
                OpErrorKind::Conflict,
                CAPTURED,
                format!(
                    "the move would make a name written for no entity at {} name the moved entity",
                    places.join(", ")
                ),
            )
            .with_suggestion("change the name written there first")
            .about(EditField::Entity));
        }
        for (key, mut edits) in edits {
            edits.dedup_by(|a, b| a.span == b.span);
            let Some(text) = self.text(&key) else {
                continue;
            };
            if let Some(new) = apply_edits(&text, edits.iter()) {
                self.texts.insert(key, new);
            }
        }
        Ok(())
    }
}

/// `scopes` once the entities `moves` (old path, new path) are at their new paths, each in the scope its path says.
pub(super) fn rekeyed(scopes: &Scopes, moves: &[(String, String)]) -> Scopes {
    let mut after = scopes.clone();
    for (old, _) in moves {
        after.remove(Sym::new(old));
    }
    for (old, new) in moves {
        if let Some(kind) = scopes.kind(old) {
            after.insert(Sym::new(new), kind, parent(new));
        }
    }
    after
}

/// The scope a path says: all but its last name; none for a name alone or a scheme ref id.
pub(super) fn parent(path: &str) -> Option<Sym> {
    if path.contains(':') {
        return None;
    }
    path.rsplit_once('/').map(|(scope, _)| Sym::new(scope))
}
