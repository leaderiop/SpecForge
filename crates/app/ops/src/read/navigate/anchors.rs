//! The anchors read view (ADR 0349): an anchor is an edge of the compiled graph between an authored entity (of a kind
//! no collector writes) and a located fact (one with a place in source), either way: a `.spec` field citing the fact,
//! or a fact naming the entity. [`anchors_in`] walks the edges once; the anchors of an entity
//! ([`anchors_of_entity`]), of a source file ([`anchors_of_file`]) and the covered items inference gaps leave out
//! ([`CoveredItems`]) read it, as the language server does. MCP's `specforge.find_implementation`,
//! `specforge.find_spec_for_source`, the infer prompt's file scope and `specforge anchors` render these documents.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::Serialize;
use specforge_common::shape::Shape;
use specforge_common::{Diagnostic, DiagnosticList, SourceSpan, Sym, codes};
use specforge_graph::{Found, Graph, Node};

use super::{FileMatch, match_file};
use crate::view::{EntityField, ProjectView, not_found};
use crate::{Containment, Noticed, OpError, OpErrorKind, ReadFailure, Refusal};

/// The field of a file query a refusal can be about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAnchorsField {
    /// The source-code path asked for.
    FilePath,
}

/// The entities anchored to the located facts the source files `file_path` names
/// (`McpSpecForSourceResult`; the infer prompt's file scope lists its `entities`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct FileAnchors {
    /// The query, as asked.
    pub file_path: String,
    /// `exact` when some located fact's file is the query; else `directory` when one is under it, else
    /// `suffix_path`; `none` when none matches ([`match_file`]).
    pub match_mode: FileMatch,
    /// The anchors of the tightest non-empty mode, by file, line, fact and `via`.
    pub entities: Vec<AnchoredEntity>,
    pub count: usize,
}

/// A file answer as a payload with no `_meta` carries it (the infer prompt's file scope, ADR 0111 D6): its match
/// and rows, and its notices (I212, W198) beside them.
#[derive(Debug, Clone, PartialEq)]
pub struct FileAnchorsDocument {
    pub match_mode: FileMatch,
    pub entities: Vec<AnchoredEntity>,
    pub notices: DiagnosticList,
}

impl Noticed<FileAnchors> {
    /// The answer and its notices, owned, for a payload that says them inside.
    pub fn document(&self) -> FileAnchorsDocument {
        let found = self.value();
        FileAnchorsDocument {
            match_mode: found.match_mode,
            entities: found.entities.clone(),
            notices: DiagnosticList::from(self.notices().to_vec()),
        }
    }
}

/// One entity anchored to a located fact of a source file (`McpSpecForSourceEntity`): the entity, beside the anchor's
/// fact end, whose keys sit at the row's top level.
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct AnchoredEntity {
    pub entity_id: String,
    /// The entity's kind.
    pub kind: String,
    #[serde(flatten)]
    pub anchor: Anchor,
}

/// The located facts one entity is anchored to (`McpImplementationResult`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct EntityAnchors {
    pub entity_id: String,
    /// By file, line, fact and `via`.
    pub implementations: Vec<Anchor>,
    pub count: usize,
}

/// One anchor's fact end, where its source places it (`McpImplementation`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct Anchor {
    /// The located fact's id.
    pub fact_id: String,
    /// The source file of the place, a key from the project root.
    pub file: String,
    /// The place's first and last lines (1-based).
    pub line: usize,
    pub end_line: usize,
    /// What the fact's field named `kind` lists (`function`, `struct, module`), else the fact's kind (`test`).
    pub item_kind: String,
    /// The collector whose store holds the fact.
    pub collector: String,
    /// The edge: `<holder kind>.<field>`, `item.names` when the fact names the entity, `task.items` when the
    /// entity's field names the fact.
    pub via: String,
}

impl Anchor {
    /// The order rows are listed in: file, line, fact, `via`.
    fn order(&self) -> (&str, usize, &str, &str) {
        (&self.file, self.line, &self.fact_id, &self.via)
    }
}

/// One anchor of the graph: the edge between `entity` and `fact`, written in its holder's `field`.
#[derive(Debug, Clone, Copy)]
pub struct AnchorEdge<'g> {
    /// The authored end.
    pub entity: &'g Node,
    /// The located fact.
    pub fact: &'g Node,
    /// The kind of the end that writes the reference (the edge's source).
    pub holder: Sym,
    /// The field it is written in (the edge's label).
    pub field: Sym,
}

impl AnchorEdge<'_> {
    /// `<holder kind>.<field>`.
    pub fn via(&self) -> String {
        format!("{}.{}", self.holder, self.field)
    }

    /// The row of this anchor at the fact's place `place`.
    fn row(&self, place: &SourceSpan) -> Anchor {
        let kinds = crate::read::infer::definition_kinds(self.fact);
        Anchor {
            fact_id: self.fact.id.raw.to_string(),
            file: place.file.to_string(),
            line: place.start_line,
            end_line: place.end_line,
            item_kind: if kinds.is_empty() {
                self.fact.kind.raw.to_string()
            } else {
                kinds
            },
            collector: specforge_common::facts::filed(self.fact.source_span.file.as_str())
                .map_or_else(String::new, |(collector, _)| collector.to_string()),
            via: self.via(),
        }
    }
}

/// Every anchor of a graph, read in one walk of its edges, by entity and by fact ([`anchors_in`]).
pub struct Anchors<'g> {
    edges: Vec<AnchorEdge<'g>>,
    by_entity: HashMap<Sym, Vec<usize>>,
    by_fact: HashMap<Sym, Vec<usize>>,
}

impl<'g> Anchors<'g> {
    /// Every anchor, in the graph's edge order.
    pub fn all(&self) -> &[AnchorEdge<'g>] {
        &self.edges
    }

    /// The anchors of the authored entity `id`.
    pub fn of_entity(&self, id: &str) -> impl Iterator<Item = &AnchorEdge<'g>> {
        Self::indexed(&self.edges, &self.by_entity, id)
    }

    /// The anchors of the located fact `id`.
    pub fn of_fact(&self, id: &str) -> impl Iterator<Item = &AnchorEdge<'g>> {
        Self::indexed(&self.edges, &self.by_fact, id)
    }

    fn indexed<'a>(
        edges: &'a [AnchorEdge<'g>],
        index: &'a HashMap<Sym, Vec<usize>>,
        id: &str,
    ) -> impl Iterator<Item = &'a AnchorEdge<'g>> {
        index
            .get(&Sym::new(id))
            .into_iter()
            .flatten()
            .map(move |&i| &edges[i])
    }
}

/// Every anchor of the view's graph: each edge whose one end is an authored entity (of a kind no collector writes)
/// and whose other end is a located fact (a node of a collected kind with a place in source), whichever end holds
/// the reference. An edge between two facts or two authored entities is none.
pub fn anchors_in<'g>(view: &ProjectView<'g>) -> Anchors<'g> {
    let kinds = view.registries().kinds();
    of_graph(view.graph(), &|kind| kinds.is_collected(kind))
}

/// [`anchors_in`] over `graph`, `collected` saying which kinds a collector writes.
fn of_graph<'g>(graph: &'g Graph, collected: &dyn Fn(&str) -> bool) -> Anchors<'g> {
    let mut anchors = Anchors {
        edges: Vec::new(),
        by_entity: HashMap::new(),
        by_fact: HashMap::new(),
    };
    let authored = |node: &Node| !collected(node.kind.raw.as_str());
    let located = |node: &Node| !node.located.is_empty() && collected(node.kind.raw.as_str());
    for edge in graph.edges() {
        let (Some(source), Some(target)) = (
            graph.node(edge.source.as_str()),
            graph.node(edge.target.as_str()),
        ) else {
            continue;
        };
        let (entity, fact) = if authored(source) && located(target) {
            (source, target)
        } else if located(source) && authored(target) {
            (target, source)
        } else {
            continue;
        };
        let at = anchors.edges.len();
        anchors.edges.push(AnchorEdge {
            entity,
            fact,
            holder: source.kind.raw,
            field: edge.label,
        });
        anchors.by_entity.entry(entity.id.raw).or_default().push(at);
        anchors.by_fact.entry(fact.id.raw).or_default().push(at);
    }
    anchors
}

/// Which source items the project covers (CONTEXT.md "Covered source item", ADR 0349): an item fact with an anchor
/// to an entity the graph declares. What inference gaps leave out.
pub(crate) struct CoveredItems<'a> {
    anchors: Anchors<'a>,
}

impl<'a> CoveredItems<'a> {
    /// The view's covered items.
    pub(crate) fn of(view: &ProjectView<'a>) -> Self {
        CoveredItems {
            anchors: anchors_in(view),
        }
    }

    /// Whether the item fact `item` of the view's graph is covered: it has an anchor.
    pub(crate) fn covers(&self, item: &Node) -> bool {
        self.anchors.of_fact(item.id.raw.as_str()).next().is_some()
    }
}

/// What an anchors answer notices (ADR 0349 D5): I212 when the session serves no located fact, then one W198 for each
/// file of the answer's `rows` (`(file, collector)`) that is not the text the store of a row's collector recorded
/// ([`CollectedSources::holds`]). Only the answer's files are read, each once, inside the project root.
fn noticed<'f, T>(
    view: &ProjectView<'_>,
    rows: impl IntoIterator<Item = (&'f str, &'f str)>,
    value: T,
) -> Noticed<T> {
    let kinds = view.registries().kinds();
    let mut notices = Vec::new();
    let located = view
        .graph()
        .nodes()
        .into_iter()
        .any(|node| !node.located.is_empty() && kinds.is_collected(node.kind.raw.as_str()));
    if !located {
        notices.push(
            Diagnostic::new(
                codes::I212,
                "the session serves no located fact, so nothing can be anchored: no collector's facts carry a place in source",
            )
            .with_suggestion("load a source pack (`specforge add @rust/source`) and run `specforge collect`"),
        );
    }
    if view.root().is_some() {
        let sources = CollectedSources::of(view);
        let mut read: BTreeMap<&str, Option<Vec<u8>>> = BTreeMap::new();
        let mut stale: BTreeMap<&str, bool> = BTreeMap::new();
        for (file, collector) in rows.into_iter().collect::<BTreeSet<_>>() {
            let now = read
                .entry(file)
                .or_insert_with(|| read_placed(view, file).ok());
            if !now
                .as_deref()
                .is_some_and(|text| sources.holds(file, Some(collector), text))
            {
                stale.insert(file, now.is_none());
            }
        }
        notices.extend(
            stale
                .into_iter()
                .map(|(file, gone)| stale_notice(file, gone)),
        );
    }
    Noticed::new(value, notices)
}

/// What the served fact stores say of the source files their facts are placed in (ADR 0346 D2, D7, ADR 0349 D5): each
/// store's [`Placed`](specforge_project::facts::Placed) of each file, by file and by the store's collector, read once
/// per answer. The one owner of whether a source file is the text a store's served places are in (the text collected,
/// or the text a rename since left): the anchors read view's W198, the language server's placing and a rename read it.
pub struct CollectedSources<'p> {
    by_file: BTreeMap<&'p str, BTreeMap<&'p str, &'p specforge_project::facts::Placed>>,
}

impl<'p> CollectedSources<'p> {
    /// The view's (`CompiledProject::fact_sources`).
    pub fn of(view: &ProjectView<'p>) -> Self {
        let by_file = view
            .project()
            .fact_sources()
            .into_iter()
            .map(|(file, stores)| {
                let by_collector = stores
                    .into_iter()
                    .map(|(store, placed)| {
                        let collector =
                            specforge_common::facts::filed(store).map_or(store, |(c, _)| c);
                        (collector, placed)
                    })
                    .collect();
                (file, by_collector)
            })
            .collect();
        CollectedSources { by_file }
    }

    /// Whether a served store places a fact in `file`, a key from the project root.
    pub fn places(&self, file: &str) -> bool {
        self.by_file.contains_key(file)
    }

    /// Whether `text`, the bytes of `file` now, is the text the places of each store placing a fact in it are in, or
    /// of only the store of `collector` when given ([`Placed::holds`](specforge_project::facts::Placed::holds)).
    /// `false` for a file no such store records: nothing says its text is the one collected.
    pub fn holds(&self, file: &str, collector: Option<&str>, text: &[u8]) -> bool {
        let Some(stores) = self.by_file.get(file) else {
            return false;
        };
        let mut recorded = stores
            .iter()
            .filter(|(of, _)| collector.is_none_or(|wanted| wanted == **of))
            .map(|(_, placed)| *placed)
            .peekable();
        if recorded.peek().is_none() {
            return false;
        }
        recorded.all(|placed| placed.holds(text))
    }
}

/// The bytes of `file`, a source file a fact is placed in (a key from the project root), read inside the project root
/// whatever the view's containment: a key that climbs out of it, a link that lands outside it and a file with another
/// name are refused (`PermissionDenied`, ADR 0144, ADR 0146), as a file of a view with no root is (`NotFound`). The
/// one read of a placed source file: what the anchors read view hashes, a rename rewrites and the language server
/// places in. A text the view holds for `file` ([`ProjectView::holding`]: an editor's buffer, a plan's rewrite not yet
/// written) is that text.
pub fn read_placed(view: &ProjectView<'_>, file: &str) -> std::io::Result<Vec<u8>> {
    if let Some(text) = view.held(file) {
        return Ok(text.as_bytes().to_vec());
    }
    let root = view.root().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "the view has no project root")
    })?;
    match Containment::within(root).read(&root.join(file)) {
        Ok((_, bytes)) => Ok(bytes),
        Err(ReadFailure::Outside(outside)) => Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            OpError::from(outside).message,
        )),
        Err(ReadFailure::Io { error, .. }) => Err(error),
    }
}

/// W198 of `file`, a source file a fact is placed in that changed since the collect read it, or is `gone` (no file
/// [`read_placed`] reads): what the anchors read view and the language server's source lens say.
pub fn stale_notice(file: &str, gone: bool) -> Diagnostic {
    let said = if gone {
        "is gone, or no longer a file inside the project, since `specforge collect` read it"
    } else {
        "changed since `specforge collect` read it"
    };
    Diagnostic::new(
        codes::W198,
        format!("'{file}' {said}: the lines its anchors give may have moved"),
    )
    .with_suggestion("run `specforge collect` again")
}

/// How tight a match is: exact first, then under the query or ending with it.
fn tightness(mode: FileMatch) -> u8 {
    match mode {
        FileMatch::Exact => 0,
        FileMatch::Directory | FileMatch::SuffixPath => 1,
        FileMatch::None => 2,
    }
}

/// Which entities are anchored to the located facts of the source files `file_path` names: each place of each
/// anchored fact matched under [`match_file`], one row per anchor, at the fact's tightest-matching place, with the
/// entity's id and kind. The tightest non-empty mode wins: exact matches, else those under the query or ending with
/// it. Rows are by file, line, fact and `via`. `file_path` is read from the project root as a named spec file is (an
/// absolute path under the root is that path, a `..` applied where it stands: [`ProjectView::source_root`], ADR
/// 0253), then compared as text against each place's file, a key from the project root; one whose key lies outside
/// the project root, or that names nothing, is `not_a_source_path`, about [`FileAnchorsField::FilePath`]. No file is
/// opened to answer; each file the answer names is hashed for W198, and I212 says no located fact is served
/// ([`noticed`]). A view with no root is `no_project`.
pub fn anchors_of_file(
    view: &ProjectView<'_>,
    file_path: &str,
) -> Result<Noticed<FileAnchors>, Refusal<FileAnchorsField>> {
    let query = view
        .source_root(file_path)
        .map_err(|error| error.about(FileAnchorsField::FilePath))?;
    view.project_root()?;
    let anchors = anchors_in(view);
    let mut exact = Vec::new();
    let mut loose = Vec::new();
    let mut under = false;
    for edge in anchors.all() {
        let best = edge
            .fact
            .located
            .iter()
            .map(|place| (match_file(query.as_str(), place.file.as_str()), place))
            .min_by_key(|(mode, _)| tightness(*mode));
        let Some((mode, place)) = best else {
            continue;
        };
        let row = AnchoredEntity {
            entity_id: edge.entity.id.raw.to_string(),
            kind: edge.entity.kind.raw.to_string(),
            anchor: edge.row(place),
        };
        match mode {
            FileMatch::Exact => exact.push(row),
            FileMatch::Directory => {
                under = true;
                loose.push(row);
            }
            FileMatch::SuffixPath => loose.push(row),
            FileMatch::None => {}
        }
    }
    let (match_mode, mut entities) = if !exact.is_empty() {
        (FileMatch::Exact, exact)
    } else if loose.is_empty() {
        (FileMatch::None, loose)
    } else if under {
        (FileMatch::Directory, loose)
    } else {
        (FileMatch::SuffixPath, loose)
    };
    entities
        .sort_by(|a, b| (a.anchor.order(), &a.entity_id).cmp(&(b.anchor.order(), &b.entity_id)));
    let rows: Vec<(String, String)> = entities
        .iter()
        .map(|row| (row.anchor.file.clone(), row.anchor.collector.clone()))
        .collect();
    let found = FileAnchors {
        file_path: file_path.to_string(),
        match_mode,
        count: entities.len(),
        entities,
    };
    Ok(noticed(
        view,
        rows.iter()
            .map(|(file, collector)| (file.as_str(), collector.as_str())),
        found,
    ))
}

/// The located facts `entity_id` is anchored to, one row per anchor at the fact's first place, by file, line, fact
/// and `via`; none for an entity with no anchor. An id the graph does not declare is E003 (with a did-you-mean), one
/// it declares more than once is ambiguous, and a fact's id is refused (`invalid_input`): its anchors are those of
/// its file. The answer's files are hashed for W198, and I212 says no located fact is served ([`noticed`]).
pub fn anchors_of_entity(
    view: &ProjectView<'_>,
    entity_id: &str,
) -> Result<Noticed<EntityAnchors>, Refusal<EntityField>> {
    let graph = view.graph();
    let refused = |error: OpError| Refusal::about(EntityField::EntityId, error);
    let id = match graph.find(entity_id) {
        Found::One(path) => path,
        Found::Ambiguous(paths) => {
            return Err(refused(crate::view::ambiguous(entity_id, &paths)));
        }
        Found::Mistyped(_) | Found::None(_) => return Err(refused(not_found(graph, entity_id))),
    };
    let kinds = view.registries().kinds();
    if graph
        .node(id.as_str())
        .is_some_and(|node| kinds.is_collected(node.kind.raw.as_str()))
    {
        return Err(refused(
            OpError::new(
                OpErrorKind::InvalidInput,
                "invalid_input",
                format!("'{id}' is a fact, not an authored entity: its anchors are its file's"),
            )
            .with_suggestion("ask the anchors of the fact's source file"),
        ));
    }
    let anchors = anchors_in(view);
    let mut implementations: Vec<Anchor> = anchors
        .of_entity(id.as_str())
        .map(|edge| edge.row(edge.fact.place()))
        .collect();
    implementations.sort_by(|a, b| a.order().cmp(&b.order()));
    let rows: Vec<(String, String)> = implementations
        .iter()
        .map(|row| (row.file.clone(), row.collector.clone()))
        .collect();
    let found = EntityAnchors {
        entity_id: id.to_string(),
        count: implementations.len(),
        implementations,
    };
    Ok(noticed(
        view,
        rows.iter()
            .map(|(file, collector)| (file.as_str(), collector.as_str())),
        found,
    ))
}
