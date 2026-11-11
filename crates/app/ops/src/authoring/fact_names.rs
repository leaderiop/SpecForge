//! What a rename rewrites where a collected fact names the entity (ADR 0346 D7): the source file at each place the
//! fact's store records the name written (`named_at`), and the store's stale line. The store itself stays the
//! collector's bytes until the next collect. Renames compose: a store is read as served, its names as the renames
//! before wrote them and its places where those renames left them.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use specforge_graph::{Found, Reading};
use specforge_project::facts::{self, Placed, Span, Write, Writes};
use specforge_project::harness::Line;
use specforge_protocol_types::FieldType;

use crate::authoring::apply::COLLECTED;
use crate::authoring::planned::Rewritten;
use crate::authoring::rename::{CAPTURED, NAME_SHARED, PLACES_CROSS, SOURCE_MOVED, STALE_CONFLICT};
use crate::read::navigate::{SourceText, TextEdit, apply_edits};
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind};

/// The source edits and stale lines of a rename: each rewritten file by its key from the project root, each edit, and a `stale:` line for each name rewritten where a store's facts
/// hold it.
#[derive(Debug, Default)]
pub(crate) struct Rewrite {
    pub files: BTreeMap<String, Rewritten>,
    pub edits: Vec<TextEdit>,
    pub stale: Vec<Marked>,
}

/// A stale line a rename marks, and the source files it writes the name at: what the line says of the store is true
/// of exactly those files' texts (an editor that holds them apart reads each one alone).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marked {
    pub line: Line,
    pub files: BTreeSet<String>,
}

/// The rewrite of the facts that name an entity `moved` moves (old path → new path, the renamed entity first), each
/// read from its store as the session serves it (`facts::store`: a name an earlier rename rewrote is the name it
/// wrote, at the place it left it) and each name resolved as the linker resolves it. For each such name, a stale line
/// from the name the store holds to the same suffix of the new path (replacing an earlier rename's line), and an edit
/// at every place the store writes the name: exactly the places the serve shifts by once the line is written. Places
/// nested one in another (two facts holding `v1/T1` and its `T1`) write one text, the outermost each (`Writes`). Each
/// file is read through the view (its held text, else the disk's: `read_placed`) and must be the text every store
/// whose places it rewrites serves them in (`Placed::holds`: the text collected, or the text the earlier renames
/// left). Refused `collected` when a fact records no place its name is written; `source_moved` when a file is not
/// that text (edited since by hand), is gone or no longer spells the name at a place; `name_captured` when the new
/// suffix would also name another entity; `name_shared` when another fact of the store holds the name in that field
/// for something else (a store's name is renamed in every fact holding it), or when a write would change a name any
/// fact holds in the file that the rename does not rename; `places_cross` when two places it writes cross (no one
/// text writes both); `stale_conflict` when the ledger renames
/// a name the store holds two ways (W202); `file_unreadable` when the served store cannot be read, or a file is no
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
    let mut judges: BTreeMap<String, BTreeMap<String, Placed>> = BTreeMap::new();
    let mut rewrite = Rewrite::default();
    // The names the rename rewrites, by store: (store, field, the name served).
    let mut rewritten: BTreeSet<(String, String, String)> = BTreeSet::new();
    for (key, facts) in &naming {
        // The store as the session serves it: the names and places the earlier renames left.
        let text = view
            .project()
            .source_text(key)
            .ok_or_else(|| unreadable(old_id, &format!("the fact store '{key}' is not served")))?;
        let served = facts::store(key, &text).map_err(|w192| unreadable(old_id, &w192.message))?;
        if !served.conflicts.is_empty() {
            return Err(conflicted(old_id, key, &served.conflicts));
        }
        let store = &served.store;
        // What a fact's name in `field` names, as the linker resolves a fact's reference (ADR 0346 D4): none when the
        // field holds no reference.
        let resolved = |fact: &specforge_protocol_types::Fact, field: &str, spelling: &str| {
            let target = match forms.reading(&fact.kind, field) {
                Reading::Declared(form)
                    if matches!(
                        form.field_type(),
                        FieldType::Reference | FieldType::ReferenceList
                    ) =>
                {
                    forms.target(&fact.kind, field)
                }
                _ => return None,
            };
            let admits = |kind: &str| target.as_ref().is_none_or(|t| t.admits(kind));
            match graph.scopes().resolve(None, spelling, &admits) {
                Found::One(path) | Found::Mistyped(path) => Some(path),
                Found::Ambiguous(_) | Found::None(_) => None,
            }
        };
        // The names the rename rewrites in the store: (field, the name served) → the path it names and the name it
        // writes.
        let mut names: BTreeMap<(String, String), (&str, String)> = BTreeMap::new();
        for fact in store.facts.iter().filter(|f| facts.contains(&f.id)) {
            for (field, value) in &fact.fields {
                let spellings = match value {
                    serde_json::Value::Array(items) => {
                        items.iter().filter_map(|i| i.as_str()).collect()
                    }
                    single => single.as_str().into_iter().collect::<Vec<_>>(),
                };
                for spelling in spellings {
                    let Some(named) = resolved(fact, field, spelling) else {
                        continue;
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
                    names.insert((field.clone(), spelling.to_string()), (old, new_text));
                }
            }
        }
        for ((field, spelling), (old, new_text)) in &names {
            rewritten.insert((key.clone(), field.clone(), spelling.clone()));
            let mut files = BTreeSet::new();
            for fact in &store.facts {
                let written = fact.named_at.get(field).and_then(|ids| ids.get(spelling));
                // A store's name is renamed in every fact holding it: each must name the entity moved.
                if written.is_some_and(|at| !at.is_empty())
                    && resolved(fact, field, spelling).is_none_or(|named| named.as_str() != *old)
                {
                    return Err(shared(
                        old_id,
                        &fact.id,
                        field,
                        spelling,
                        resolved(fact, field, spelling),
                    ));
                }
                for place in written.into_iter().flatten() {
                    files.insert(place.file.clone());
                    // The reader serves no store placing a fact in a file it records no digest of.
                    judges
                        .entry(place.file.clone())
                        .or_default()
                        .insert(key.clone(), served.placed[&place.file].clone());
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
            // One line for each name held that is served as the spelling: the edits above write every place of each.
            for held in served.collected(field, spelling) {
                rewrite.stale.push(Marked {
                    line: Line::stale(
                        &store.collector,
                        &store.key,
                        served.bytes.as_bytes(),
                        field,
                        held,
                        new_text,
                    ),
                    files: files.clone(),
                });
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
        let holds = judges[&file].values().all(|placed| placed.holds(&bytes));
        if !holds {
            return Err(moved());
        }
        let text: Arc<str> = Arc::from(String::from_utf8(bytes).map_err(|_| moved())?);
        let source = SourceText::new(Arc::clone(&text));
        let spelled = edits.values().all(|(edit, spelling)| {
            source.range(&edit.span).and_then(|range| text.get(range)) == Some(spelling.as_str())
        });
        if !spelled {
            return Err(moved());
        }
        // Places nested one in another write one text, the outermost each; crossing ones write none.
        let writes = Writes::of(edits.iter().map(|(span, (edit, spelling))| Write {
            span: *span,
            held: spelling.clone(),
            now: edit.new_text.clone(),
        }))
        .ok_or_else(|| crossed(old_id, &file))?;
        kept_by(view, old_id, &file, &writes, &edits, &rewritten)?;
        let mut edits = edits;
        let edits: Vec<TextEdit> = writes
            .outer()
            .iter()
            .filter_map(|write| edits.remove(&write.span).map(|(edit, _)| edit))
            .collect();
        let edited = apply_edits(&text, &edits).ok_or_else(moved)?;
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

/// `Ok` when what the rename writes in `file` (`writes`, planned as `edits`: each place's edit and the name read there)
/// leaves every name another fact holds there as it reads: every store placing a fact in the file, served, and every
/// name its facts hold at a place in it that the rename does not rewrite (`rewritten`). `name_shared` naming the first
/// that a write changes: one at a place written, or holding one, or inside one in a part the name written does not
/// keep. A rename writes only names of what it moves.
fn kept_by(
    view: &ProjectView<'_>,
    old_id: &str,
    file: &str,
    writes: &Writes,
    edits: &BTreeMap<Span, (TextEdit, String)>,
    rewritten: &BTreeSet<(String, String, String)>,
) -> Result<(), OpError> {
    let project = view.project();
    let sources = project.fact_sources();
    for key in sources
        .get(file)
        .into_iter()
        .flat_map(|stores| stores.keys())
    {
        let text = project
            .source_text(key)
            .ok_or_else(|| unreadable(old_id, &format!("the fact store '{key}' is not served")))?;
        let served = facts::store(key, &text).map_err(|w192| unreadable(old_id, &w192.message))?;
        for fact in &served.store.facts {
            for (field, ids) in &fact.named_at {
                for (spelling, places) in ids {
                    if rewritten.contains(&(key.to_string(), field.clone(), spelling.clone())) {
                        continue;
                    }
                    for place in places.iter().filter(|p| p.file == file) {
                        let at = (
                            place.start_line,
                            place.start_col,
                            place.end_line,
                            place.end_col,
                        );
                        let kept = match edits.get(&at) {
                            Some((edit, _)) => edit.new_text == *spelling,
                            None => writes.place(at, true).is_some(),
                        };
                        if !kept {
                            return Err(written_over(old_id, &fact.id, field, spelling, place));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// `name_shared`: the rename of `id` would write over the name `spelling` the fact `fact` holds in `field` at `place`,
/// which it does not rename: that fact would read another name there.
fn written_over(
    id: &str,
    fact: &str,
    field: &str,
    spelling: &str,
    place: &specforge_protocol_types::FactLocation,
) -> OpError {
    OpError::new(
        OpErrorKind::Conflict,
        NAME_SHARED,
        format!(
            "cannot rename '{id}': it writes over '{spelling}', which the fact '{fact}' holds in '{field}' at {}:{}:{}              and the rename does not rename; a rename writes only names of what it moves",
            place.file, place.start_line, place.start_col
        ),
    )
    .with_suggestion("change what the collector reads, run `specforge collect`, then rename")
}

/// `places_cross`: two places the rename writes in `file` cross one another, or nest with names that disagree, so no
/// one text writes both.
fn crossed(id: &str, file: &str) -> OpError {
    OpError::new(
        OpErrorKind::Conflict,
        PLACES_CROSS,
        format!(
            "cannot rename '{id}': two places of a name it writes in {file} cross one another, so no one text writes              both"
        ),
    )
    .with_suggestion("change what the collector reads, run `specforge collect`, then rename")
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

/// `name_shared`: the fact `fact` of the store holds `spelling` in `field` too, where it names `named` (another entity, or
/// nothing), and a store's name is renamed in every fact that holds it.
fn shared(
    id: &str,
    fact: &str,
    field: &str,
    spelling: &str,
    named: Option<specforge_common::Sym>,
) -> OpError {
    let names = named.map_or_else(|| "nothing".to_string(), |path| format!("'{path}'"));
    OpError::new(
        OpErrorKind::Conflict,
        NAME_SHARED,
        format!(
            "cannot rename '{id}': the fact '{fact}' holds '{spelling}' in '{field}' too, where it names {names}; a \
             fact store's name is renamed in every fact that holds it"
        ),
    )
    .with_suggestion("change what the collector reads, run `specforge collect`, then rename")
}

/// `stale_conflict`: the ledger renames a name the fact store `key` holds two ways (`conflicts`), so its places are
/// not served where the source has them.
fn conflicted(id: &str, key: &str, conflicts: &facts::Conflicts) -> OpError {
    let named: Vec<String> = conflicts
        .values()
        .flat_map(|names| names.iter())
        .map(|(held, now)| {
            let now: Vec<String> = now.iter().map(|n| format!("'{n}'")).collect();
            format!("'{held}' ({})", now.join(", "))
        })
        .collect();
    OpError::new(
        OpErrorKind::PreconditionFailed,
        STALE_CONFLICT,
        format!(
            "cannot rename '{id}': specforge-harness holds two renames of {} in the fact store '{key}'",
            named.join(" and of ")
        ),
    )
    .with_suggestion("run `specforge collect`, then rename")
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
