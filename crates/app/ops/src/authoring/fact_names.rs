//! What a rename rewrites where a collected fact names the entity (ADR 0346 D7): the source file at each place the
//! fact's store records the name written (`named_at`), and the store's stale line. The store itself stays the
//! collector's bytes until the next collect. Renames compose: a store is read as served, its names as the renames
//! before wrote them and its places where those renames left them.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use specforge_graph::{Found, Reading};
use specforge_project::facts::{self, Placed};
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

/// A place by its start and end (line, byte column): one edit per place, however many facts name it.
type Span = (usize, usize, usize, usize);

/// The rewrite of the facts that name an entity `moved` moves (old path → new path, the renamed entity first), each
/// read from its store as the session serves it (`facts::store`: a name an earlier rename rewrote is the name it
/// wrote, at the place it left it) and each name resolved as the linker resolves it. For each such name, a stale line
/// from the name the store holds to the same suffix of the new path (replacing an earlier rename's line), and an edit
/// at every place the store writes the name: exactly the places the serve shifts by once the line is written. Each
/// file is read through the view (its held text, else the disk's: `read_placed`) and must be the text every store
/// whose places it rewrites serves them in (`Placed::holds`: the text collected, or the text the earlier renames
/// left). Refused `collected` when a fact records no place its name is written; `source_moved` when a file is not
/// that text (edited since by hand), is gone or no longer spells the name at a place; `name_captured` when the new
/// suffix would also name another entity; `file_unreadable` when the served store cannot be read, or a file is no
/// file inside the project (a key that climbs out, a link that lands outside, a file with another name:
/// `read_placed`).
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
    // Each file's edits by place, with the spelling each place must still hold.
    let mut places: BTreeMap<String, BTreeMap<Span, (TextEdit, String)>> = BTreeMap::new();
    // What each store whose places a file's edits are says of the file.
    let mut judges: BTreeMap<String, BTreeMap<String, Option<Placed>>> = BTreeMap::new();
    let mut rewrite = Rewrite::default();
    for (key, facts) in &naming {
        // The store as the session serves it: the names and places the earlier renames left.
        let text = view
            .project()
            .source_text(key)
            .ok_or_else(|| unreadable(old_id, &format!("the fact store '{key}' is not served")))?;
        let served = facts::store(key, &text).map_err(|w192| unreadable(old_id, &w192.message))?;
        let store = &served.store;
        // The names the rename rewrites in the store: (field, the name served) → the name it writes.
        let mut names: BTreeMap<(String, String), String> = BTreeMap::new();
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
                    fact.named_at
                        .get(field)
                        .and_then(|ids| ids.get(spelling))
                        .filter(|at| !at.is_empty())
                        .ok_or_else(|| no_place(old_id, field, &fact.id))?;
                    let new_text = specforge_common::facts::respelled(new, spelling);
                    captured(view, old, &new_text)?;
                    names.insert((field.clone(), spelling.to_string()), new_text);
                }
            }
        }
        for ((field, spelling), new_text) in &names {
            // One line for each name held that is served as the spelling: the edits below write every place of each.
            for held in served.collected(field, spelling) {
                rewrite.stale.push(Line::stale(
                    &store.collector,
                    &store.key,
                    served.bytes.as_bytes(),
                    field,
                    held,
                    new_text,
                ));
            }
            for fact in &store.facts {
                let written = fact.named_at.get(field).and_then(|ids| ids.get(spelling));
                for place in written.into_iter().flatten() {
                    judges
                        .entry(place.file.clone())
                        .or_default()
                        .insert(key.clone(), served.placed.get(&place.file).cloned());
                    let edit = TextEdit {
                        span: facts::source_span(place),
                        new_text: new_text.clone(),
                    };
                    let span = (
                        place.start_line,
                        place.start_col,
                        place.end_line,
                        place.end_col,
                    );
                    places
                        .entry(place.file.clone())
                        .or_default()
                        .insert(span, (edit, spelling.clone()));
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
        let bytes = crate::read::navigate::read_placed(view, &file).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                moved()
            } else {
                unreadable(
                    old_id,
                    &format!("{file} is no file inside the project to rewrite: {error}"),
                )
            }
        })?;
        let holds = judges[&file]
            .values()
            .all(|placed| placed.as_ref().is_some_and(|placed| placed.holds(&bytes)));
        if !holds {
            return Err(moved());
        }
        let text: Arc<str> = Arc::from(String::from_utf8(bytes).map_err(|_| moved())?);
        let source = SourceText::new(Arc::clone(&text));
        let spelled = edits.values().all(|(edit, spelling)| {
            source.range(&edit.span).and_then(|range| text.get(range)) == Some(spelling.as_str())
        });
        let edits: Vec<TextEdit> = edits.into_values().map(|(edit, _)| edit).collect();
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
