//! What a rename rewrites where a collected fact names the entity (ADR 0346 D7): the source file at each place the
//! fact's store records the name written (`named_at`), and the store's stale line. The store itself stays the
//! collector's bytes until the next collect.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use specforge_graph::{Found, Reading};
use specforge_project::facts;
use specforge_project::harness::Line;
use specforge_protocol_types::FieldType;

use crate::authoring::apply::COLLECTED;
use crate::authoring::planned::Rewritten;
use crate::authoring::rename::{CAPTURED, SOURCE_MOVED};
use crate::read::navigate::{SourceText, TextEdit, apply_edits};
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind};

/// The source edits and stale lines of a rename: each rewritten file by its key from the project root, each edit, and a `stale:` line for each name rewritten where a store's facts
/// hold it.
#[derive(Debug, Default)]
pub(crate) struct Rewrite {
    pub files: BTreeMap<String, Rewritten>,
    pub edits: Vec<TextEdit>,
    pub stale: Vec<Line>,
}

/// The rewrite of the facts that name an entity `moved` moves (old path → new path, the renamed entity first), each
/// read from its store as the session serves it (`facts::store`: a name an earlier rename rewrote is the name it
/// wrote) and each name resolved as the linker resolves it: every place each such name is written, its written suffix
/// becoming the same suffix of the new path. Refused `collected` when a fact records no place its name is written;
/// `source_moved` when a file changed since the collect read it (its digest is not the store's: an earlier rename
/// rewrote it) or no longer spells the name at a place; `name_captured` when the new suffix would also name another
/// entity; `file_unreadable` when the served store cannot be read.
pub(crate) fn rewrite(
    view: &ProjectView<'_>,
    old_id: &str,
    moved: &[(String, String)],
) -> Result<Rewrite, OpError> {
    let graph = view.graph();
    let forms = view.registries().field_forms();
    let moved_to: BTreeMap<&str, &str> = moved
        .iter()
        .map(|(old, new)| (old.as_str(), new.as_str()))
        .collect();
    // Each store whose facts name a moved entity, and those facts.
    let mut naming: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (old, _) in moved {
        for edge in graph.edges_to(old) {
            let Some(fact) = graph.node(edge.source.as_str()) else {
                continue;
            };
            let store = fact.source_span.file.as_str();
            if specforge_common::facts::is_fact_store(store) {
                naming
                    .entry(store.to_string())
                    .or_default()
                    .insert(fact.id.raw.to_string());
            }
        }
    }
    // Each file's edits, with the spelling each place must still hold.
    let mut places: BTreeMap<String, Vec<(TextEdit, String)>> = BTreeMap::new();
    let mut digests: BTreeMap<String, String> = BTreeMap::new();
    let mut rewrite = Rewrite::default();
    for (key, facts) in &naming {
        // The store as the session serves it: the names a rename marked it stale with are the ones it wrote.
        let text = view
            .project()
            .source_text(key)
            .ok_or_else(|| unreadable(old_id, &format!("the fact store '{key}' is not served")))?;
        let served = facts::store(key, &text).map_err(|w192| unreadable(old_id, &w192.message))?;
        let store = &served.store;
        for fact in store.facts.iter().filter(|f| facts.contains(&f.id)) {
            for (field, value) in &fact.fields {
                let target = match forms.reading(&fact.kind, field) {
                    Reading::Declared(form)
                        if matches!(
                            form.field_type(),
                            FieldType::Reference | FieldType::ReferenceList
                        ) =>
                    {
                        forms.target(&fact.kind, field)
                    }
                    _ => continue,
                };
                let admits = |kind: &str| target.as_ref().is_none_or(|t| t.admits(kind));
                let spellings = match value {
                    serde_json::Value::Array(items) => {
                        items.iter().filter_map(|i| i.as_str()).collect()
                    }
                    single => single.as_str().into_iter().collect::<Vec<_>>(),
                };
                for spelling in spellings {
                    // What the name names, as the linker resolves a fact's reference (ADR 0346 D4).
                    let named = match graph.scopes().resolve(None, spelling, &admits) {
                        Found::One(path) | Found::Mistyped(path) => path,
                        Found::Ambiguous(_) | Found::None(_) => continue,
                    };
                    let Some((old, new)) = moved_to.get_key_value(named.as_str()) else {
                        continue;
                    };
                    let written = fact
                        .named_at
                        .get(field)
                        .and_then(|ids| ids.get(spelling))
                        .filter(|at| !at.is_empty())
                        .ok_or_else(|| no_place(old_id, field, &fact.id))?;
                    let new_text = specforge_common::facts::respelled(new, spelling);
                    captured(view, old, &new_text)?;
                    rewrite.stale.push(Line::stale(
                        &store.collector,
                        &store.key,
                        served.bytes.as_bytes(),
                        field,
                        spelling,
                        &new_text,
                    ));
                    for place in written {
                        digests.insert(
                            place.file.clone(),
                            store.sources.get(&place.file).cloned().unwrap_or_default(),
                        );
                        let edit = TextEdit {
                            span: facts::source_span(place),
                            new_text: new_text.clone(),
                        };
                        places
                            .entry(place.file.clone())
                            .or_default()
                            .push((edit, spelling.to_string()));
                    }
                }
            }
        }
    }
    for (file, edits) in places {
        let moved = || {
            OpError::new(
                OpErrorKind::PreconditionFailed,
                SOURCE_MOVED,
                format!(
                    "cannot rename '{old_id}': {file} changed since `specforge collect` read it"
                ),
            )
            .with_suggestion("run `specforge collect`, then rename")
        };
        let bytes = read(view, &file).ok_or_else(moved)?;
        if crate::revision::of_bytes(&bytes) != digests[&file] {
            return Err(moved());
        }
        let text: Arc<str> = Arc::from(String::from_utf8(bytes).map_err(|_| moved())?);
        let source = SourceText::new(Arc::clone(&text));
        let spelled = edits.iter().all(|(edit, spelling)| {
            source.range(&edit.span).and_then(|range| text.get(range)) == Some(spelling.as_str())
        });
        let edits: Vec<TextEdit> = edits.into_iter().map(|(edit, _)| edit).collect();
        let edited = spelled
            .then(|| apply_edits(&text, &edits))
            .flatten()
            .ok_or_else(moved)?;
        rewrite.files.insert(
            file,
            Rewritten {
                before: text,
                after: edited,
            },
        );
        rewrite.edits.extend(edits);
    }
    Ok(rewrite)
}

/// The bytes of `key`, a path from the project root, read through the view's containment.
fn read(view: &ProjectView<'_>, key: &str) -> Option<Vec<u8>> {
    let root = view.project().root()?;
    view.containment()
        .read(&root.join(key))
        .ok()
        .map(|(_, bytes)| bytes)
}

/// `name_captured` when `written`, the spelling the rename writes, would also name an entity other than the one moved
/// from `old`: a fact's name resolves by unique suffix.
fn captured(view: &ProjectView<'_>, old: &str, written: &str) -> Result<(), OpError> {
    let other = view
        .graph()
        .nodes()
        .into_iter()
        .map(|node| node.id.raw.as_str())
        .find(|path| *path != old && specforge_common::facts::names(path, written));
    match other {
        Some(other) => Err(OpError::new(
            OpErrorKind::Conflict,
            CAPTURED,
            format!(
                "cannot rename '{old}': a fact's source would write '{written}', which also names '{other}'"
            ),
        )
        .with_suggestion("choose another name")),
        None => Ok(()),
    }
}

/// The refusal of a rename of `id` whose fact store a session serves cannot be read, and `why`.
fn unreadable(id: &str, why: &str) -> OpError {
    OpError::new(
        OpErrorKind::PreconditionFailed,
        crate::FILE_UNREADABLE,
        format!("cannot rename '{id}': {why}"),
    )
    .with_suggestion("run `specforge collect`, then rename")
}

/// `collected`: the fact names `id` in `field` and records no place it is written, so there is no text to rewrite.
fn no_place(id: &str, field: &str, fact: &str) -> OpError {
    OpError::new(
        OpErrorKind::PermissionDenied,
        COLLECTED,
        format!(
            "{id} is named by '{field}' of the fact '{fact}', which records no place it is written; rename it in the \
             source, then run `specforge collect`"
        ),
    )
    .with_suggestion("change what the collector reads, then run `specforge collect`")
}
