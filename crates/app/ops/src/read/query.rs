//! Query, list and search: the read views that select entities of the
//! project view (ADR 0015, section "Query"). `specforge query` and
//! `specforge.query` render one query; `specforge.list` and
//! `specforge://entities/{kind}` one listing; `specforge search` and
//! `specforge.search` one search, the document [`EntityHits`]. Each request
//! holds its defaults; a kind filter reports the kinds the project does not
//! know (I020) and still drops them.

use serde::Serialize;
use serde_json::{Map, Value, json};
use specforge_common::shape::Shape;
use specforge_graph::Node;

use crate::read::export::{self, AGENT_FORMAT, ExportField, FORMAT, Format, Schema};
use crate::read::navigate::{
    EntityMatch, EntityQuery, MatchScope, MatchedOn, find_entities, snippet,
};
use crate::view::{ProjectView, Select};
use crate::{Noticed, OpError, OpErrorKind, Refusal};

/// Hops from the queried entity when a query names none: its direct
/// neighbours. `specforge query --depth` and `specforge.query`'s `depth`
/// default to it.
pub const DEFAULT_DEPTH: usize = 1;
/// Matches a search returns when it names no limit.
pub const DEFAULT_SEARCH_LIMIT: usize = 20;
/// Characters of a matched field's text shown on each side of the match.
pub const SNIPPET_WIDTH: usize = 40;

/// The field of a query request a refusal can be about (ADR 0072).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryField {
    /// `QueryRequest::entity_id`.
    EntityId,
    /// `QueryRequest::format`.
    Format,
}

/// The subgraph around one entity.
#[derive(Debug, Clone, Default)]
pub struct QueryRequest<'a> {
    pub entity_id: &'a str,
    /// Hops from the entity; `None` is [`DEFAULT_DEPTH`]. 0 is the entity
    /// alone.
    pub depth: Option<usize>,
    /// Keep only entities of these kinds, the queried entity always, and
    /// an edge when both of its entities are kept. A kind the project does
    /// not know matches nothing and is a notice.
    pub kinds: Vec<&'a str>,
    /// `None` is [`AGENT_FORMAT`]'s default (graph). `dot` is refused: it
    /// is not an agent format.
    pub format: Option<Format>,
    /// Give every exported entity its coverage status (`coverage_status`,
    /// a [`crate::read::coverage::STATUS`] name).
    pub include_coverage: bool,
}

/// The query `request` over the view: the export of the entity's subgraph
/// (`export::export` of the scoped request under the export schema policy: a
/// graph-format query references the published schema, Graph Protocol 2.0);
/// with `include_coverage`, each node carrying its `coverage_status`, written
/// by the export. Its notices are what versioning the schema reported
/// (ADR 0111), then I020 for each kind of the filter the project does not
/// know (the export's). An entity the graph lacks is
/// [`crate::view::not_found`] (E003, did-you-mean), refused before the
/// recorded report is read; a recorded report that cannot be read, when
/// coverage is asked for, is its E045 failure.
pub fn query(
    view: &ProjectView,
    request: &QueryRequest,
) -> Result<Noticed<String>, Refusal<QueryField>> {
    let format = request
        .format
        .or(AGENT_FORMAT.default)
        .expect("the agent formats have a default");
    if !AGENT_FORMAT.admits(format) {
        return Err(AGENT_FORMAT
            .parse(FORMAT.name_of(format))
            .expect_err("a format the agent table does not admit is refused")
            .about(QueryField::Format));
    }
    view.entity(request.entity_id, QueryField::EntityId)?;
    let coverage = request
        .include_coverage
        .then(|| view.coverage())
        .transpose()
        .map_err(Refusal::from)?;
    let exported = export::export(
        view,
        &export::Request {
            format: Some(format),
            scope: Some(request.entity_id),
            depth: Some(request.depth.unwrap_or(DEFAULT_DEPTH)),
            kinds: request.kinds.clone(),
            schema: Schema::Default,
            coverage: coverage.as_deref(),
            ..export::Request::default()
        },
    )
    .map_err(|refusal| {
        refusal.map_subject(|field| match field {
            ExportField::Scope => Some(QueryField::EntityId),
            // A query always scopes, sends none of these and writes nothing.
            ExportField::MaxTokens
            | ExportField::SchemaVersion
            | ExportField::Schema
            | ExportField::Directory => None,
        })
    })?;
    Ok(exported)
}

/// The JSON Schema of the document [`query`] answers: the export document of
/// each agent format, derived by the emitter from the types that write it (a
/// node's optional `coverage_status` included). A brief document is also a
/// valid context document, so the formats are alternatives, not exclusive.
pub fn document_schema(defs: &mut specforge_common::shape::Definitions) -> Value {
    let branches: Vec<Value> = [Format::Graph, Format::Context, Format::Brief]
        .into_iter()
        .filter(|format| AGENT_FORMAT.admits(*format))
        .filter_map(|format| specforge_emitter::document_schema(format.emit_format(), defs))
        .collect();
    json!({ "type": "object", "anyOf": branches })
}

/// Entities of one kind (or every kind) whose fields hold given values.
#[derive(Debug, Clone, Default)]
pub struct ListRequest<'a> {
    /// `None`: every kind. A kind the project does not know, the empty one included, lists
    /// nothing and is a notice.
    pub kind: Option<&'a str>,
    /// Field name to the value it holds, compared as the field's JSON
    /// (`specforge_graph::field_form::write`), any kind's fields.
    pub fields: Option<&'a Map<String, Value>>,
    /// Skip this many, after sorting by id.
    pub offset: usize,
    /// At most this many; `None` is every one.
    pub limit: Option<usize>,
}

/// The entities `request` selects, sorted by id and paged, with I020 for a
/// kind the project does not know.
pub fn list<'v>(view: &ProjectView<'v>, request: &ListRequest) -> Noticed<Vec<&'v Node>> {
    let filter = view.kind_filter(request.kind.as_slice());
    let graph = view.graph();
    let selected = graph
        .select(None, None, filter.kinds())
        .expect("a selection without a root always selects");
    let entities = selected
        .ids()
        .filter_map(|id| graph.node(id.as_str()))
        .filter(|node| request.fields.is_none_or(|wanted| holds(node, wanted)))
        .skip(request.offset)
        .take(request.limit.unwrap_or(usize::MAX))
        .collect();
    Noticed::new(entities, filter.into_notices())
}

/// Whether every field of `wanted` is one `node` holds, with that value.
fn holds(node: &Node, wanted: &Map<String, Value>) -> bool {
    wanted.iter().all(|(field, value)| {
        node.fields
            .entries()
            .iter()
            .find(|entry| entry.key.as_str() == field)
            .is_some_and(|entry| {
                &Value::from(specforge_graph::field_form::write(&entry.value)) == value
            })
    })
}

/// A field and the text it must contain (ignoring case): both, or neither.
#[derive(Debug, Clone, Copy)]
pub struct FieldHolds<'a> {
    pub field: &'a str,
    pub value: &'a str,
}

/// The code of the refusal of a search's field without the text it must contain.
pub const FIELD_WITHOUT_VALUE: &str = "field_without_value";

/// The code of the refusal of a search's text to find in a field without the field.
pub const VALUE_WITHOUT_FIELD: &str = "value_without_field";

impl<'a> FieldHolds<'a> {
    /// The one reading of a search's field and value, for every surface (ADR 0101 D1): both are the field
    /// filter, neither is none, and one alone is invalid input about the other, the one it needs.
    pub fn of(
        field: Option<&'a str>,
        value: Option<&'a str>,
    ) -> Result<Option<Self>, Refusal<SearchField>> {
        match (field, value) {
            (Some(field), Some(value)) => Ok(Some(FieldHolds { field, value })),
            (None, None) => Ok(None),
            (Some(_), None) => Err(OpError::new(
                OpErrorKind::InvalidInput,
                FIELD_WITHOUT_VALUE,
                "'field' needs 'value': the text the field must contain",
            )
            .about(SearchField::Value)),
            (None, Some(_)) => Err(OpError::new(
                OpErrorKind::InvalidInput,
                VALUE_WITHOUT_FIELD,
                "'value' needs 'field': the field whose text it must be in",
            )
            .about(SearchField::Field)),
        }
    }
}

/// The field of a search request a refusal can be about (ADR 0072).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchField {
    /// `SearchRequest::referencing`.
    Referencing,
    /// The field of [`FieldHolds`]: a value given without it.
    Field,
    /// The text of [`FieldHolds`]: a field given without it.
    Value,
}

/// Entities a text finds, ranked as the LSP ranks completion and
/// workspace symbols ([`find_entities`], over names and string fields),
/// every filter ANDed.
#[derive(Debug, Clone, Default)]
pub struct SearchRequest<'a> {
    /// Empty: every entity that passes the filters.
    pub text: &'a str,
    pub kinds: Vec<&'a str>,
    pub field: Option<FieldHolds<'a>>,
    /// Only entities that reference this entity; one the graph lacks is
    /// refused (`view::not_found`).
    pub referencing: Option<&'a str>,
    /// `None` is [`DEFAULT_SEARCH_LIMIT`].
    pub limit: Option<usize>,
}

/// What a search finds, best first: the document `specforge search --format json` prints and `specforge.search`
/// replies (`McpSearchResults`).
#[derive(Debug, Serialize, Shape)]
pub struct EntityHits {
    pub results: Vec<EntityHit>,
}

/// One entity a search finds (`McpSearchResult`): its id, kind, title and place, its rank's score, and for a match
/// in a field's text the field and the text around it ([`SNIPPET_WIDTH`] characters each side).
#[derive(Debug, Serialize, Shape)]
pub struct EntityHit {
    pub entity_id: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub file_path: String,
    pub line: usize,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_snippet: Option<String>,
}

impl EntityHit {
    /// The hit of one match, with the snippet of a field-text match.
    fn of(found: &EntityMatch, match_snippet: Option<String>) -> Self {
        let node = found.node;
        EntityHit {
            entity_id: node.id.raw.to_string(),
            kind: node.kind.raw.to_string(),
            title: node.title.as_ref().map(ToString::to_string),
            file_path: node.source_span.file.to_string(),
            line: node.source_span.start_line,
            score: found.score,
            match_field: found.on.field_name().map(ToString::to_string),
            match_snippet,
        }
    }
}

/// The entities `request` finds, best first, at most the limit, with I020
/// for a kind of the filter the project does not know. A `referencing`
/// entity the graph lacks is [`crate::view::not_found`] (E003).
pub fn search(
    view: &ProjectView,
    request: &SearchRequest,
) -> Result<Noticed<EntityHits>, Refusal<SearchField>> {
    let selection = view.select(&Select {
        entity: request.referencing,
        named_by: SearchField::Referencing,
        depth: None,
        kinds: &request.kinds,
    })?;
    let query = EntityQuery {
        text: request.text,
        scope: MatchScope::NamesAndText,
        kinds: &request.kinds,
        field_contains: request.field.map(|held| (held.field, held.value)),
        referencing: request.referencing,
        limit: Some(request.limit.unwrap_or(DEFAULT_SEARCH_LIMIT)),
    };
    let results = find_entities(view.graph(), &query)
        .iter()
        .map(|found| {
            let snippet = match found.on {
                MatchedOn::Field(field) => snippet(found.node, field, request.text, SNIPPET_WIDTH),
                _ => None,
            };
            EntityHit::of(found, snippet)
        })
        .collect();
    Ok(Noticed::new(
        EntityHits { results },
        selection.into_notices(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_holds_of_one_alone_is_refused_about_the_other() {
        let refusal = FieldHolds::of(Some("contract"), None).unwrap_err();
        assert_eq!(refusal.error.kind, OpErrorKind::InvalidInput);
        assert_eq!(refusal.error.code, FIELD_WITHOUT_VALUE);
        assert_eq!(refusal.subject, Some(SearchField::Value));
        let refusal = FieldHolds::of(None, Some("alpha")).unwrap_err();
        assert_eq!(refusal.error.kind, OpErrorKind::InvalidInput);
        assert_eq!(refusal.error.code, VALUE_WITHOUT_FIELD);
        assert_eq!(refusal.subject, Some(SearchField::Field));
    }

    #[test]
    fn field_holds_of_both_or_neither_is_one_request() {
        let held = FieldHolds::of(Some("contract"), Some("alpha"))
            .unwrap()
            .unwrap();
        assert_eq!((held.field, held.value), ("contract", "alpha"));
        assert!(FieldHolds::of(None, None).unwrap().is_none());
    }
}
