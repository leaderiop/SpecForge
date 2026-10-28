use serde::Serialize;
use specforge_common::shape::Shape;
use specforge_coverage::Status;
use specforge_graph::field_form::FieldJson;
use specforge_graph::{
    AnnotationJson, Computed, Graph, Members, MethodJson, Node, Obligation, VerdictJson,
};
use specforge_protocol_types::Judgement;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use crate::budget::TokenBudget;
use crate::error::EmitterError;
use crate::schema::{GraphProtocolSchema, SchemaRefBlock, SchemaVersion};

/// The envelope version an export declares: 1.0 for the schemaless graph
/// export, 2.0 once a schema is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Shape)]
pub(crate) enum ExportVersion {
    #[serde(rename = "1.0")]
    V1,
    #[serde(rename = "2.0")]
    V2,
}

/// The envelope every export is written in: the graph's entities as `N` and
/// its edges, with the format and schema versions, the schema (embedded, or a
/// reference to it) and the `token_budget` block when they apply. Field order
/// is the wire order.
#[derive(Serialize, Shape)]
pub(crate) struct Export<'a, N: Serialize> {
    /// "1.0" for the schemaless graph format, "2.0" once a schema is
    /// attached; context and brief without a schema carry none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format_version: Option<ExportVersion>,
    pub schema_version: Cow<'a, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<&'a GraphProtocolSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<SchemaRefBlock>,
    pub nodes: Vec<N>,
    pub edges: Vec<JsonEdge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_budget: Option<&'a TokenBudget>,
}

impl<'a, N: Serialize> Export<'a, N> {
    /// `graph`'s edges and `nodes`, in `format_version` with no schema and no
    /// budget block.
    pub(crate) fn plain(
        format_version: Option<ExportVersion>,
        schema_version: &SchemaVersion,
        graph: &Graph,
        nodes: Vec<N>,
    ) -> Self {
        Export {
            format_version,
            schema_version: Cow::Owned(schema_version.to_string()),
            schema: None,
            schema_ref: None,
            nodes,
            edges: sorted_edges(graph),
            token_budget: None,
        }
    }

    pub(crate) fn to_json(&self) -> Result<String, EmitterError> {
        serde_json::to_string(self).map_err(|e| EmitterError::Serialization(e.to_string()))
    }
}

#[derive(Serialize, Shape)]
pub(crate) struct JsonNode {
    id: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    file: String,
    line: usize,
    fields: BTreeMap<String, FieldJson>,
    /// The annotations written after its fields, by field name (a block field's entry's by `outer.key`); omitted
    /// when it has none (ADR 0222).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    annotations: BTreeMap<String, Vec<AnnotationJson>>,
    /// Its methods, in written order; omitted when it has none (ADR 0222).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    methods: Vec<MethodJson>,
    /// The entity's own `verify` statements, beside its fields (ADR 0094); omitted when it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    verify: Option<Vec<Obligation>>,
    /// The entity's coverage status, when the export was given verdicts
    /// (`EmitOptions::coverage`): what `specforge query --include-coverage`
    /// shows.
    #[serde(skip_serializing_if = "Option::is_none")]
    coverage_status: Option<Status>,
    /// The written paths of the fields and members in `fields` a compute pass gave (ADR 0287); omitted when none.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    computed: BTreeSet<String>,
    /// The entity's `verdict` statements as written, in order; omitted when it has none (ADR 0290).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    verdicts: Vec<VerdictJson>,
    /// Where each judged rule that judges it stands (ADR 0290); omitted when none does (a gated rule is absent).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    judgements: Vec<JudgementJson>,
}

/// One judged rule's state on the entity: `Judgement` without the files it read.
#[derive(Debug, Serialize, Shape)]
#[shape(define = "JudgementJson")]
pub struct JudgementJson {
    rule: String,
    /// `fresh`, `stale`, `missing`, `unrecorded` or `unreadable`.
    state: String,
    /// Fresh, recorded and its outcome passing.
    passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<String>,
    /// The inputs that changed since the verdict (stale only).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    changed: Vec<String>,
}

impl JudgementJson {
    pub fn of(judgement: &Judgement) -> Self {
        Self {
            rule: judgement.rule.clone(),
            state: judgement.state.clone(),
            passed: judgement.passed,
            outcome: judgement.outcome.clone(),
            at: judgement.at.clone(),
            changed: judgement.changed.clone(),
        }
    }
}

/// `n` as the graph export writes an entity.
pub(crate) fn graph_node(
    n: &Node,
    coverage_status: Option<Status>,
    computed: Option<&Computed>,
    judgements: Option<&BTreeMap<String, Vec<Judgement>>>,
) -> JsonNode {
    let Members {
        fields,
        annotations,
        methods,
        computed,
        verdicts,
    } = crate::emit::members_of(n, computed);
    JsonNode {
        id: n.id.raw.to_string(),
        kind: n.kind.raw.to_string(),
        title: n.title.clone(),
        file: n.source_span.file.to_string(),
        line: n.source_span.start_line,
        fields,
        annotations,
        methods,
        verify: (!n.obligations.is_empty())
            .then(|| n.obligations.iter().map(Obligation::of).collect()),
        coverage_status,
        computed,
        verdicts,
        judgements: judgements
            .and_then(|all| all.get(n.id.raw.as_str()))
            .map_or_else(Vec::new, |own| own.iter().map(JudgementJson::of).collect()),
    }
}

#[derive(Serialize, Shape)]
#[shape(define = "Edge")]
pub(crate) struct JsonEdge {
    pub source: String,
    pub target: String,
    pub label: String,
}

pub(crate) fn sorted_edges(graph: &Graph) -> Vec<JsonEdge> {
    let mut edges: Vec<JsonEdge> = graph
        .edges()
        .iter()
        .map(|e| JsonEdge {
            source: e.source.to_string(),
            target: e.target.to_string(),
            label: e.label.to_string(),
        })
        .collect();
    edges.sort_by(|a, b| (&a.source, &a.target, &a.label).cmp(&(&b.source, &b.target, &b.label)));
    edges
}
