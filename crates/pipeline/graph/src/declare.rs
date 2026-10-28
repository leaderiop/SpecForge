//! The one sweep over the files that decides which declaration is each entity's node, and where the graph places
//! it (ADR 0284): an entity's path is its scope's path and its name, and a name is unique within its scope.

use crate::build::{GraphConfig, GraphInputs, Ids, about, is_define_block};
use crate::defaults::{FileDefaults, is_defaults_block};
use crate::scope::Found;
use crate::{Graph, Node, title};
use specforge_common::{Diagnostic, SourceSpan, Sym, codes, structural};
use specforge_parser::{Entity, EntityId, FieldValue, REF_SCHEME_FIELD, SpecFile};
use std::collections::HashMap;

/// What the sweep over the files decided.
pub(crate) struct Declarations<'a> {
    /// The node's declaration of each path: the first that is not a `define` block, and the first of its path's
    /// kind (ADR 0284: a name is unique within its scope).
    pub(crate) winners: HashMap<Sym, Winner<'a>>,
    /// The path of every declaration that is a node's, winner or not, by file.
    pub(crate) paths: HashMap<Sym, Vec<Sym>>,
    /// Each file's defaults (ADR 0149), by path.
    pub(crate) defaults: HashMap<Sym, FileDefaults<'a>>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

/// The declaration a path's node is made from.
#[derive(Clone, Copy)]
pub(crate) struct Winner<'a> {
    pub(crate) entity: &'a Entity,
    /// The entity it is scoped by, as it resolved or is written (`None`: the project).
    pub(crate) scope: Option<Sym>,
}

/// What a declaration of a path found already there.
enum Outcome {
    /// The path's first declaration.
    Wins,
    /// A second declaration of the same kind: E002, naming the first.
    Duplicate(SourceSpan),
    /// A declaration of another kind than the first's: W060.
    Shared(Sym),
}

/// How many kinds a kind is scoped under: 0 when nothing scopes it, else one more than its scope field's target
/// kind. Registration refuses a cycle (W021), and the walk stops at one all the same.
fn depth_of(kind: &str, inputs: &GraphInputs) -> usize {
    let (mut depth, mut at) = (0, kind);
    while let Some(field) = inputs.scopes.get(at) {
        let Some(up) = inputs
            .field_forms
            .target(at, field)
            .and_then(|t| t.only_loaded())
        else {
            break;
        };
        depth += 1;
        if depth > inputs.scopes.len() {
            break;
        }
        at = up;
    }
    depth
}

/// The scope `entity` writes: its scope field's value, as the graph stores it (through its file's defaults), `None`
/// when its kind is scoped by nothing or it writes none.
pub(crate) fn written_scope(
    inputs: &GraphInputs,
    defaults: &FileDefaults<'_>,
    entity: &Entity,
) -> Option<String> {
    let kind = entity.kind.raw.as_str();
    let field = inputs.scopes.get(kind)?;
    let held = defaults.held(entity, field);
    let entry = held.entry()?;
    match inputs.field_forms.stored(kind, entry).as_ref() {
        FieldValue::Identifier(text) | FieldValue::String(text) => Some(text.clone()),
        _ => None,
    }
}

/// The scope and the path of the node `entity` declares: its scope field resolved from the project (the one place
/// besides a surface's input that takes a unique suffix), the path being the scope's path and the name. A scope that
/// resolves to no entity of the field's kind is kept as written, so the path is deterministic, cannot equal a
/// resolved one, and two such declarations still collide (E002); the field reports E003 (ADR 0284).
pub(crate) fn declared_path(
    inputs: &GraphInputs,
    defaults: &FileDefaults<'_>,
    scopes: &crate::Scopes,
    entity: &Entity,
) -> (Option<Sym>, Sym) {
    let Some(text) = written_scope(inputs, defaults, entity) else {
        return (None, entity.id.raw);
    };
    let kind = entity.kind.raw.as_str();
    let target = inputs
        .scopes
        .get(kind)
        .and_then(|field| inputs.field_forms.target(kind, field));
    let admits = |k: &str| target.as_ref().is_none_or(|t| t.admits(k));
    let scope = match scopes.resolve(None, &text, &admits) {
        Found::One(path) => path,
        _ => Sym::new(&text),
    };
    (Some(scope), Sym::new(&format!("{scope}/{}", entity.id.raw)))
}

/// The path each block of `file` declares against `scopes`, in file order: none for a `defaults` or `define` block. A
/// duplicate declaration has its winner's path, so a typed edit finds a block by its path whatever the file holds.
pub fn block_paths(
    file: &SpecFile,
    inputs: &GraphInputs,
    scopes: &crate::Scopes,
) -> Vec<Option<Sym>> {
    let (defaults, _) = FileDefaults::of(file, &inputs.field_forms);
    file.entities
        .iter()
        .map(|entity| {
            (!is_defaults_block(entity) && !is_define_block(entity))
                .then(|| declared_path(inputs, &defaults, scopes, entity).1)
        })
        .collect()
}

/// The one sweep over the files, in the iteration order they are given:
/// every file's E001s and the I007 / E019 its
/// format version header reports first, then per file and entity W143 / E076 / E081 / E002 / W060, then I005. It is
/// also first-writer-wins, per scope (ADR 0284): an entity's path is its scope's path and its name, resolved
/// shallowest kinds first, so a scope field finds the scope it names. `seen` is keyed on (kind, path), so a second
/// declaration of a kind in a scope is E002 naming the first of that kind; a project has one `spec` block, whatever
/// its name (ADR 0282 D2); `winners` is keyed on the path, so a declaration of another kind is W060. Placement
/// reads `winners`. Each diagnostic about one declaration names it in its data (`Subject`), by its path; W143 names
/// nothing. A project with no scoped kind has every path its name, and sweeps in file order.
pub(crate) fn declarations<'a>(
    files: impl Iterator<Item = &'a SpecFile> + Clone,
    config: &GraphConfig,
) -> Declarations<'a> {
    let mut diagnostics = Vec::new();

    let mut defaults = HashMap::new();
    // Surface parse errors as diagnostics so CLI/MCP consumers see them.
    for spec_file in files.clone() {
        diagnostics.extend(spec_file.errors.iter().map(Diagnostic::from));
        // What the format version header reports (I007, E019), where the
        // file declares it.
        diagnostics.extend(spec_file.format_diagnostics.iter().cloned());
        let (file_defaults, problems) = FileDefaults::of(spec_file, &config.inputs.field_forms);
        diagnostics.extend(problems);
        defaults.insert(spec_file.path, file_defaults);
    }

    // Every block in file order; a defaults block is no node.
    let blocks: Vec<&'a Entity> = files
        .clone()
        .flat_map(|file| &file.entities)
        .filter(|entity| !is_defaults_block(entity))
        .collect();

    // Resolve the paths, shallowest kinds first (file order within a depth), so a scope field finds its scope.
    let mut order: Vec<usize> = (0..blocks.len())
        .filter(|&i| !is_define_block(blocks[i]))
        .collect();
    order.sort_by_key(|&i| depth_of(blocks[i].kind.raw.as_str(), &config.inputs));
    let mut index = crate::Scopes::new();
    // Where each (kind, path) was first declared, so a duplicate names both sites.
    let mut seen: HashMap<(Sym, Sym), SourceSpan> = HashMap::new();
    let mut winners: HashMap<Sym, Winner<'a>> = HashMap::new();
    let mut decided: Vec<Option<(Sym, Outcome)>> = Vec::new();
    decided.resize_with(blocks.len(), || None);
    for i in order {
        let entity = blocks[i];
        let file_defaults = &defaults[&entity.span.file];
        let (scope, path) = declared_path(&config.inputs, file_defaults, &index, entity);
        // A project has one spec block, whatever its name (ADR 0282 D2).
        let root = entity.kind.raw == structural::SPEC;
        let key = (entity.kind.raw, if root { entity.kind.raw } else { path });
        let outcome = if let Some(first) = seen.get(&key) {
            Outcome::Duplicate(first.clone())
        } else {
            seen.insert(key, entity.span.clone());
            // The same path used by several kinds is W060; the first declaration is retained.
            if let Some(first) = winners.get(&path) {
                Outcome::Shared(first.entity.kind.raw)
            } else {
                winners.insert(path, Winner { entity, scope });
                index.insert(path, entity.kind.raw, scope);
                Outcome::Wins
            }
        };
        decided[i] = Some((path, outcome));
    }

    let mut paths: HashMap<Sym, Vec<Sym>> = HashMap::new();
    for (i, entity) in blocks.iter().copied().enumerate() {
        if is_define_block(entity) {
            diagnostics.push(define_block_warning(entity));
            continue;
        }
        let Some((path, outcome)) = decided[i].take() else {
            continue;
        };
        paths.entry(entity.span.file).or_default().push(path);
        // Placed all the same: the LSP keeps the block navigable and every check after the build sees it.
        if title::owes_title(entity) {
            diagnostics.push(title::untitled_block_error(entity, path));
        }
        diagnostics.extend(repeated_field_errors(entity, path));
        match outcome {
            Outcome::Wins => {}
            Outcome::Duplicate(first) => {
                let (what, site, fix) = if entity.kind.raw == structural::SPEC {
                    (
                        "a second spec block",
                        "the project's spec block is declared at",
                        "remove it, or merge it into the first: a project has one spec block",
                    )
                } else {
                    (
                        "duplicate entity ID",
                        "first declared at",
                        "rename one of the entities to avoid the collision",
                    )
                };
                let named = if entity.kind.raw == structural::SPEC {
                    entity.id.raw
                } else {
                    path
                };
                diagnostics.push(
                    Diagnostic::new(
                        codes::E002,
                        format!(
                            "{what} '{named}' ({site} {}:{}:{})",
                            first.file, first.start_line, first.start_col
                        ),
                    )
                    .with_span(entity.span.clone())
                    .with_data(about(path))
                    .with_suggestion(fix),
                );
            }
            Outcome::Shared(first) => {
                diagnostics.push(
                    Diagnostic::new(
                        codes::W060,
                        format!(
                            "entity ID '{path}' is used by kind '{first}' and kind '{}'; first declaration (kind '{first}') is retained",
                            entity.kind.raw
                        ),
                    )
                    .with_span(entity.span.clone())
                    .with_data(about(path))
                    .with_suggestion("use distinct IDs for entities of different kinds"),
                );
            }
        }
    }

    // Check ref nodes for unknown provider schemes (I005)
    if !config.known_provider_schemes.is_empty() {
        for spec_file in files {
            for entity in &spec_file.entities {
                if entity.kind.raw == structural::REF
                    && let Some(FieldValue::String(scheme)) = entity.fields.get(REF_SCHEME_FIELD)
                    && !config.known_provider_schemes.contains(scheme)
                {
                    diagnostics.push(
                        Diagnostic::new(
                            codes::I005,
                            format!(
                                "unrecognized ref scheme '{}' in '{}' --- no provider installed for this scheme",
                                scheme, entity.id.raw
                            ),
                        )
                        .with_span(entity.span.clone())
                        .with_data(about(entity.id.raw)),
                    );
                }
            }
        }
    }

    Declarations {
        winners,
        paths,
        defaults,
        diagnostics,
    }
}

/// Make the graph's nodes the declared winners: take out a node no winner declares (a changed scope re-keys the
/// entities under it, wherever they are declared), then put in each winner `ids` names, or that the graph lacks.
pub(crate) fn place(graph: &mut Graph, declared: &Declarations<'_>, ids: Ids<'_>) {
    let stale: Vec<Sym> = graph
        .nodes()
        .iter()
        .map(|n| n.id.raw)
        .filter(|id| !declared.winners.contains_key(id))
        .collect();
    for id in stale {
        graph.remove_node(id);
    }
    for (path, winner) in &declared.winners {
        if match ids {
            Ids::All => true,
            Ids::Only(ids) => ids.contains(path) || graph.node(path.as_str()).is_none(),
        } {
            graph.add_node(node_from_entity(
                *path,
                winner,
                declared.defaults.get(&winner.entity.span.file),
            ));
        }
    }
}

/// The graph node for one parsed entity, at `path`.
fn node_from_entity(path: Sym, winner: &Winner<'_>, defaults: Option<&FileDefaults<'_>>) -> Node {
    let entity = winner.entity;
    Node {
        id: EntityId { raw: path },
        kind: entity.kind,
        title: entity.title.clone(),
        fields: defaults.map_or_else(|| entity.fields.first_of_each_key(), |d| d.resolve(entity)),
        source_span: entity.span.clone(),
        methods: entity.methods.clone(),
        obligations: entity.obligations.clone(),
        verdicts: entity.verdicts.clone(),
        scope: winner.scope,
    }
}

/// E081: each later entry of a key `entity` writes twice, at any depth (ADR 0215); its node keeps the first.
fn repeated_field_errors(entity: &Entity, path: Sym) -> impl Iterator<Item = Diagnostic> + '_ {
    entity.fields.repeats().into_iter().map(move |r| {
        let line = r.first.value_span.as_ref().map_or(entity.span.start_line, |s| s.start_line);
        Diagnostic::new(
            codes::E081,
            format!(
                "'{}' is written twice in {} '{}'; the first, at line {line}, is kept",
                r.path, entity.kind.raw, entity.id.raw
            ),
        )
        .with_span(r.later.value_span.clone().unwrap_or_else(|| entity.span.clone()))
        .with_data(about(path))
        .with_suggestion(
            "delete this line, or give every value in the first: a list takes all its items in one [...]",
        )
    })
}

/// W143: a define block, which declares nothing.
fn define_block_warning(entity: &Entity) -> Diagnostic {
    Diagnostic::new(
        codes::W143,
        format!(
            "define blocks are not supported: '{}' is not registered as an entity kind",
            entity.id.raw
        ),
    )
    .with_span(entity.span.clone())
    .with_suggestion(
        "declare custom entity kinds in an extension (`specforge extension init`), then enable it",
    )
}
