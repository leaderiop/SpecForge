use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use specforge_graph::Graph;
use specforge_registry::{
    EdgeRegistry, FieldRegistry, KindRegistry, ManifestFieldType,
};

use crate::error::EmitterError;

use crate::json::{field_map_to_json, sorted_edges, JsonEdge};

// ---------------------------------------------------------------------------
// Slice 1: Schema Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl SchemaVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch, label: None }
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref label) = self.label {
            write!(f, "-{}", label)?;
        }
        Ok(())
    }
}

impl FromStr for SchemaVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (version_part, label) = if let Some(idx) = s.find('-') {
            (&s[..idx], Some(s[idx + 1..].to_string()))
        } else {
            (s, None)
        };

        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("expected MAJOR.MINOR.PATCH, got '{}'", s));
        }

        let major = parts[0].parse::<u32>().map_err(|e| format!("invalid major: {}", e))?;
        let minor = parts[1].parse::<u32>().map_err(|e| format!("invalid minor: {}", e))?;
        let patch = parts[2].parse::<u32>().map_err(|e| format!("invalid patch: {}", e))?;

        Ok(SchemaVersion { major, minor, patch, label })
    }
}

impl Ord for SchemaVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major.cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl PartialOrd for SchemaVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    /// Extension that contributed this field (may differ from the entity's owning extension
    /// when the field comes from an entity enhancement).
    pub source_extension: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaEntityKind {
    pub name: String,
    pub source_extension: String,
    pub testable: bool,
    pub fields: Vec<SchemaField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaEdgeType {
    pub label: String,
    pub source_extension: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kinds: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_kinds: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaExtensionInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphProtocolSchema {
    pub schema_version: SchemaVersion,
    pub extensions: Vec<SchemaExtensionInfo>,
    pub entity_kinds: Vec<SchemaEntityKind>,
    pub edge_types: Vec<SchemaEdgeType>,
}

impl GraphProtocolSchema {
    pub fn empty() -> Self {
        Self {
            schema_version: SchemaVersion::new(1, 0, 0),
            extensions: Vec::new(),
            entity_kinds: Vec::new(),
            edge_types: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaMigrationChange {
    KindAdded(String),
    KindRemoved(String),
    EdgeAdded(String),
    EdgeRemoved(String),
    FieldAdded { kind: String, field: String, required: bool },
    FieldRemoved { kind: String, field: String },
    FieldTypeChanged { kind: String, field: String, old_type: String, new_type: String },
}

impl SchemaMigrationChange {
    pub fn is_breaking(&self) -> bool {
        matches!(
            self,
            SchemaMigrationChange::KindRemoved(_)
                | SchemaMigrationChange::EdgeRemoved(_)
                | SchemaMigrationChange::FieldRemoved { .. }
                | SchemaMigrationChange::FieldAdded { required: true, .. }
                | SchemaMigrationChange::FieldTypeChanged { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaMigration {
    pub changes: Vec<SchemaMigrationChange>,
}

impl SchemaMigration {
    pub fn has_breaking_changes(&self) -> bool {
        self.changes.iter().any(|c| c.is_breaking())
    }

    pub fn has_additions(&self) -> bool {
        self.changes.iter().any(|c| matches!(
            c,
            SchemaMigrationChange::KindAdded(_)
                | SchemaMigrationChange::EdgeAdded(_)
                | SchemaMigrationChange::FieldAdded { .. }
        ))
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaCompatibility {
    pub requested: SchemaVersion,
    pub resolved: SchemaVersion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersionError {
    pub requested: SchemaVersion,
    pub min: SchemaVersion,
    pub max: SchemaVersion,
    pub reason: String,
}

impl fmt::Display for SchemaVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E027: {}", self.reason)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaCacheEntry {
    pub schema: GraphProtocolSchema,
    pub content_hash: String,
}

// ---------------------------------------------------------------------------
// Slice 2: Generate Schema from Registries
// ---------------------------------------------------------------------------

fn map_field_type(ft: &ManifestFieldType) -> String {
    match ft {
        ManifestFieldType::String => "string".to_string(),
        ManifestFieldType::Integer => "integer".to_string(),
        ManifestFieldType::Bool => "boolean".to_string(),
        ManifestFieldType::Enum(_) => "enum".to_string(),
        ManifestFieldType::StringList => "string_list".to_string(),
        ManifestFieldType::Reference => "reference".to_string(),
        ManifestFieldType::ReferenceList => "reference_list".to_string(),
        ManifestFieldType::Block => "block".to_string(),
    }
}

fn enum_values(ft: &ManifestFieldType) -> Option<Vec<String>> {
    match ft {
        ManifestFieldType::Enum(values) => Some(values.clone()),
        _ => None,
    }
}

pub fn generate_schema(
    kinds: &KindRegistry,
    edges: &EdgeRegistry,
    fields: &FieldRegistry,
    extensions: &[(String, String)],
) -> GraphProtocolSchema {
    let ext_infos: Vec<SchemaExtensionInfo> = extensions
        .iter()
        .map(|(name, version)| SchemaExtensionInfo {
            name: name.clone(),
            version: version.clone(),
        })
        .collect();

    let mut entity_kinds: Vec<SchemaEntityKind> = kinds
        .iter()
        .map(|(_, entry)| {
            let mut kind_fields: Vec<SchemaField> = fields
                .fields_for_kind(&entry.kind_name)
                .into_iter()
                .map(|f| SchemaField {
                    name: f.field_name.clone(),
                    field_type: map_field_type(&f.field_type),
                    required: f.required,
                    enum_values: enum_values(&f.field_type),
                    edge: f.edge.clone(),
                    target_kind: f.target_kind.clone(),
                    description: f.description.clone(),
                    default_value: None,
                    source_extension: f.source_extension.clone(),
                })
                .collect();
            kind_fields.sort_by(|a, b| a.name.cmp(&b.name));

            SchemaEntityKind {
                name: entry.kind_name.clone(),
                source_extension: entry.source_extension.clone(),
                testable: entry.testable,
                fields: kind_fields,
            }
        })
        .collect();
    entity_kinds.sort_by(|a, b| a.name.cmp(&b.name));

    let mut edge_types: Vec<SchemaEdgeType> = edges
        .iter()
        .map(|(_, entry)| SchemaEdgeType {
            label: entry.label.clone(),
            source_extension: entry.source_extension.clone(),
            source_kinds: entry.source_kind.as_ref().map(|k| vec![k.clone()]),
            target_kinds: entry.target_kind.as_ref().map(|k| vec![k.clone()]),
        })
        .collect();

    // Infer missing source/target kinds from field-level edge mappings.
    // When an edge type has no sourceKind/targetKind in the registry, scan
    // all entity fields for matching edge labels to discover the actual
    // source and target kinds. This handles polymorphic edges like References.
    for edge_type in &mut edge_types {
        if edge_type.source_kinds.is_some() && edge_type.target_kinds.is_some() {
            continue;
        }
        let mut sources: Vec<String> = edge_type.source_kinds.clone().unwrap_or_default();
        let mut targets: Vec<String> = edge_type.target_kinds.clone().unwrap_or_default();

        for (_, kind_entry) in kinds.iter() {
            for field in fields.fields_for_kind(&kind_entry.kind_name) {
                if field.edge.as_deref() == Some(&edge_type.label) {
                    if !sources.contains(&kind_entry.kind_name) {
                        sources.push(kind_entry.kind_name.clone());
                    }
                    if let Some(tk) = field.target_kind.as_ref().filter(|tk| !targets.contains(tk)) {
                        targets.push(tk.clone());
                    }
                }
            }
        }

        if edge_type.source_kinds.is_none() && !sources.is_empty() {
            sources.sort();
            edge_type.source_kinds = Some(sources);
        }
        if edge_type.target_kinds.is_none() && !targets.is_empty() {
            targets.sort();
            edge_type.target_kinds = Some(targets);
        }
    }

    edge_types.sort_by(|a, b| a.label.cmp(&b.label));

    GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: ext_infos,
        entity_kinds,
        edge_types,
    }
}

// ---------------------------------------------------------------------------
// Slice 3: Embed Schema in Export
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct JsonGraphV2 {
    format_version: &'static str,
    schema_version: String,
    schema: GraphProtocolSchema,
    nodes: Vec<JsonNodeV2>,
    edges: Vec<JsonEdge>,
}

#[derive(Serialize)]
struct JsonNodeV2 {
    id: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    file: String,
    line: usize,
    fields: BTreeMap<String, Value>,
}

pub fn emit_json_with_schema(graph: &Graph, schema: &GraphProtocolSchema) -> String {
    let nodes: Vec<JsonNodeV2> = graph
        .nodes()
        .iter()
        .map(|n| JsonNodeV2 {
            id: n.id.raw.to_string(),
            kind: n.kind.raw.to_string(),
            title: n.title.clone(),
            file: n.source_span.file.to_string(),
            line: n.source_span.start_line,
            fields: field_map_to_json(&n.fields),
        })
        .collect();

    let output = JsonGraphV2 {
        format_version: "2.0",
        schema_version: schema.schema_version.to_string(),
        schema: schema.clone(),
        nodes,
        edges: sorted_edges(graph),
    };

    serde_json::to_string_pretty(&output).expect("graph serialization cannot fail")
}

#[derive(Serialize)]
struct ContextGraphV2 {
    format_version: &'static str,
    schema_version: String,
    schema: GraphProtocolSchema,
    nodes: Vec<ContextNodeV2>,
    edges: Vec<JsonEdge>,
}

#[derive(Serialize)]
struct ContextNodeV2 {
    id: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contract: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verify: Option<Value>,
}

pub fn emit_context_with_schema(graph: &Graph, schema: &GraphProtocolSchema) -> String {
    let nodes: Vec<ContextNodeV2> = graph
        .nodes()
        .iter()
        .map(|n| {
            let contract = n.fields.get("contract").and_then(|v| match v {
                specforge_graph::FieldValue::String(s) => Some(s.clone()),
                _ => None,
            });
            let status = n.fields.get("status").and_then(|v| match v {
                specforge_graph::FieldValue::Identifier(s) => Some(s.clone()),
                specforge_graph::FieldValue::String(s) => Some(s.clone()),
                _ => None,
            });
            let verify = n.fields.get("verify").map(|v| {
                crate::json::field_value_to_json(v)
            });

            ContextNodeV2 {
                id: n.id.raw.to_string(),
                kind: n.kind.raw.to_string(),
                title: n.title.clone(),
                contract,
                status,
                verify,
            }
        })
        .collect();

    let output = ContextGraphV2 {
        format_version: "2.0",
        schema_version: schema.schema_version.to_string(),
        schema: schema.clone(),
        nodes,
        edges: sorted_edges(graph),
    };

    serde_json::to_string_pretty(&output).expect("graph serialization cannot fail")
}

#[derive(Serialize)]
struct BriefGraphV2 {
    format_version: &'static str,
    schema_version: String,
    schema: GraphProtocolSchema,
    nodes: Vec<BriefNodeV2>,
    edges: Vec<JsonEdge>,
}

#[derive(Serialize)]
struct BriefNodeV2 {
    id: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

pub fn emit_brief_with_schema(graph: &Graph, schema: &GraphProtocolSchema) -> String {
    let nodes: Vec<BriefNodeV2> = graph
        .nodes()
        .iter()
        .map(|n| BriefNodeV2 {
            id: n.id.raw.to_string(),
            kind: n.kind.raw.to_string(),
            title: n.title.clone(),
        })
        .collect();

    let output = BriefGraphV2 {
        format_version: "2.0",
        schema_version: schema.schema_version.to_string(),
        schema: schema.clone(),
        nodes,
        edges: sorted_edges(graph),
    };

    serde_json::to_string_pretty(&output).expect("graph serialization cannot fail")
}

// ---------------------------------------------------------------------------
// Slice 4: Schema Diffing
// ---------------------------------------------------------------------------

pub fn diff_schemas(old: &GraphProtocolSchema, new: &GraphProtocolSchema) -> SchemaMigration {
    let mut changes = Vec::new();

    // Entity kinds
    let old_kinds: BTreeMap<&str, &SchemaEntityKind> =
        old.entity_kinds.iter().map(|k| (k.name.as_str(), k)).collect();
    let new_kinds: BTreeMap<&str, &SchemaEntityKind> =
        new.entity_kinds.iter().map(|k| (k.name.as_str(), k)).collect();

    for name in old_kinds.keys() {
        if !new_kinds.contains_key(name) {
            changes.push(SchemaMigrationChange::KindRemoved(name.to_string()));
        }
    }
    for name in new_kinds.keys() {
        if !old_kinds.contains_key(name) {
            changes.push(SchemaMigrationChange::KindAdded(name.to_string()));
        }
    }

    // Edge types
    let old_edges: BTreeMap<&str, &SchemaEdgeType> =
        old.edge_types.iter().map(|e| (e.label.as_str(), e)).collect();
    let new_edges: BTreeMap<&str, &SchemaEdgeType> =
        new.edge_types.iter().map(|e| (e.label.as_str(), e)).collect();

    for label in old_edges.keys() {
        if !new_edges.contains_key(label) {
            changes.push(SchemaMigrationChange::EdgeRemoved(label.to_string()));
        }
    }
    for label in new_edges.keys() {
        if !old_edges.contains_key(label) {
            changes.push(SchemaMigrationChange::EdgeAdded(label.to_string()));
        }
    }

    // Fields per shared kind
    for (name, new_kind) in &new_kinds {
        if let Some(old_kind) = old_kinds.get(name) {
            let old_fields: BTreeMap<&str, &SchemaField> =
                old_kind.fields.iter().map(|f| (f.name.as_str(), f)).collect();
            let new_fields: BTreeMap<&str, &SchemaField> =
                new_kind.fields.iter().map(|f| (f.name.as_str(), f)).collect();

            for field_name in old_fields.keys() {
                if !new_fields.contains_key(field_name) {
                    changes.push(SchemaMigrationChange::FieldRemoved {
                        kind: name.to_string(),
                        field: field_name.to_string(),
                    });
                }
            }
            for (field_name, field) in &new_fields {
                if let Some(old_field) = old_fields.get(field_name) {
                    if old_field.field_type != field.field_type {
                        changes.push(SchemaMigrationChange::FieldTypeChanged {
                            kind: name.to_string(),
                            field: field_name.to_string(),
                            old_type: old_field.field_type.clone(),
                            new_type: field.field_type.clone(),
                        });
                    }
                } else {
                    changes.push(SchemaMigrationChange::FieldAdded {
                        kind: name.to_string(),
                        field: field_name.to_string(),
                        required: field.required,
                    });
                }
            }
        }
    }

    changes.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
    SchemaMigration { changes }
}

pub fn diff_schemas_optional(
    old: Option<&GraphProtocolSchema>,
    new: &GraphProtocolSchema,
) -> SchemaMigration {
    match old {
        Some(old_schema) => diff_schemas(old_schema, new),
        None => {
            let mut changes = Vec::new();
            for kind in &new.entity_kinds {
                changes.push(SchemaMigrationChange::KindAdded(kind.name.clone()));
            }
            for edge in &new.edge_types {
                changes.push(SchemaMigrationChange::EdgeAdded(edge.label.clone()));
            }
            SchemaMigration { changes }
        }
    }
}

// ---------------------------------------------------------------------------
// Slice 5: Compute Schema Version
// ---------------------------------------------------------------------------

pub fn compute_schema_version(
    migration: &SchemaMigration,
    previous: Option<&SchemaVersion>,
) -> SchemaVersion {
    let prev = match previous {
        Some(v) => v.clone(),
        None => return SchemaVersion::new(1, 0, 0),
    };

    if migration.is_empty() {
        return prev;
    }

    if migration.has_breaking_changes() {
        SchemaVersion::new(prev.major + 1, 0, 0)
    } else if migration.has_additions() {
        SchemaVersion::new(prev.major, prev.minor + 1, 0)
    } else {
        SchemaVersion::new(prev.major, prev.minor, prev.patch + 1)
    }
}

// ---------------------------------------------------------------------------
// Slice 6: Version Negotiation
// ---------------------------------------------------------------------------

#[allow(clippy::result_large_err)]
pub fn negotiate_version(
    requested: &SchemaVersion,
    min: &SchemaVersion,
    max: &SchemaVersion,
) -> Result<SchemaCompatibility, SchemaVersionError> {
    if requested.major != max.major {
        return Err(SchemaVersionError {
            requested: requested.clone(),
            min: min.clone(),
            max: max.clone(),
            reason: format!(
                "requested schema version {} has incompatible major version (supported: {}.x)",
                requested, max.major
            ),
        });
    }

    if requested < min || requested > max {
        return Err(SchemaVersionError {
            requested: requested.clone(),
            min: min.clone(),
            max: max.clone(),
            reason: format!(
                "requested schema version {} is out of range [{}, {}]",
                requested, min, max
            ),
        });
    }

    Ok(SchemaCompatibility {
        requested: requested.clone(),
        resolved: requested.clone(),
    })
}

// ---------------------------------------------------------------------------
// Slice 7: Schema Cache Persistence
// ---------------------------------------------------------------------------

const CACHE_FILE: &str = "schema-cache.json";

fn compute_content_hash(schema: &GraphProtocolSchema) -> String {
    let json = serde_json::to_string(schema).expect("schema serialization cannot fail");
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn persist_schema_cache(schema: &GraphProtocolSchema, cache_dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(cache_dir)?;

    let entry = SchemaCacheEntry {
        content_hash: compute_content_hash(schema),
        schema: schema.clone(),
    };

    let json = serde_json::to_string_pretty(&entry)
        .map_err(io::Error::other)?;

    // Atomic write: temp file then rename
    let target = cache_dir.join(CACHE_FILE);
    let tmp = cache_dir.join(".schema-cache.tmp");
    std::fs::write(&tmp, &json)?;
    std::fs::rename(&tmp, &target)?;

    Ok(())
}

pub fn load_schema_cache(cache_dir: &Path) -> io::Result<Option<SchemaCacheEntry>> {
    let path = cache_dir.join(CACHE_FILE);
    if !path.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&path)?;
    let entry: SchemaCacheEntry = serde_json::from_str(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(Some(entry))
}

// ---------------------------------------------------------------------------
// Slice 7b: Detect Breaking Changes with Diagnostics
// ---------------------------------------------------------------------------

use specforge_common::{Diagnostic, Severity};

pub fn detect_breaking_with_diagnostics(
    cache_dir: &Path,
    current: &GraphProtocolSchema,
    output_dir_has_exports: bool,
) -> (SchemaMigration, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    let cached = match load_schema_cache(cache_dir) {
        Ok(Some(entry)) => Some(entry.schema),
        Ok(None) => {
            if output_dir_has_exports {
                diagnostics.push(Diagnostic {
                    code: "I016".to_string(),
                    severity: Severity::Info,
                    message: "Schema cache not found; breaking change detection skipped. \
                              Prior exports exist but .specforge/schema-cache.json is missing."
                        .to_string(),
                    span: None,
                    suggestion: Some("Run a full compilation to regenerate the schema cache.".to_string()),
                });
            }
            None
        }
        Err(_) => None,
    };

    let migration = diff_schemas_optional(cached.as_ref(), current);
    (migration, diagnostics)
}

// ---------------------------------------------------------------------------
// Slice 6b: Version Negotiation — default to latest
// ---------------------------------------------------------------------------

#[allow(clippy::result_large_err)]
pub fn negotiate_version_or_latest(
    requested: Option<&SchemaVersion>,
    min: &SchemaVersion,
    max: &SchemaVersion,
) -> Result<SchemaCompatibility, SchemaVersionError> {
    match requested {
        Some(v) => negotiate_version(v, min, max),
        None => Ok(SchemaCompatibility {
            requested: max.clone(),
            resolved: max.clone(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Slice 3b: Scoped V2 Exports
// ---------------------------------------------------------------------------

pub fn emit_json_scoped_with_schema(
    graph: &Graph,
    scope: &str,
    schema: &GraphProtocolSchema,
) -> Result<String, EmitterError> {
    let sub = graph.subgraph(scope).ok_or_else(|| {
        EmitterError::EntityNotFound(format!("E003: unresolved scope entity '{}' — entity not found in graph", scope))
    })?;
    Ok(emit_json_with_schema(&sub, schema))
}

pub fn emit_context_scoped_with_schema(
    graph: &Graph,
    scope: &str,
    schema: &GraphProtocolSchema,
) -> Result<String, EmitterError> {
    let sub = graph.subgraph(scope).ok_or_else(|| {
        EmitterError::EntityNotFound(format!("E003: unresolved scope entity '{}' — entity not found in graph", scope))
    })?;
    Ok(emit_context_with_schema(&sub, schema))
}

pub fn emit_brief_scoped_with_schema(
    graph: &Graph,
    scope: &str,
    schema: &GraphProtocolSchema,
) -> Result<String, EmitterError> {
    let sub = graph.subgraph(scope).ok_or_else(|| {
        EmitterError::EntityNotFound(format!("E003: unresolved scope entity '{}' — entity not found in graph", scope))
    })?;
    Ok(emit_brief_with_schema(&sub, schema))
}

// ---------------------------------------------------------------------------
// Slice 8: Serve Schema
// ---------------------------------------------------------------------------

pub fn emit_schema(schema: &GraphProtocolSchema) -> String {
    serde_json::to_string_pretty(schema).expect("schema serialization cannot fail")
}

pub fn emit_schema_for_kind(
    schema: &GraphProtocolSchema,
    kind: &str,
) -> Result<String, EmitterError> {
    schema
        .entity_kinds
        .iter()
        .find(|k| k.name == kind)
        .map(|k| serde_json::to_string_pretty(k).expect("schema serialization cannot fail"))
        .ok_or_else(|| EmitterError::EntityNotFound(format!("unknown entity kind: '{}'", kind)))
}

// ---------------------------------------------------------------------------
// Slice 9: Publish JSON Schema
// ---------------------------------------------------------------------------

pub fn publish_json_schema(schema: &GraphProtocolSchema) -> String {
    let kind_names: Vec<Value> = schema
        .entity_kinds
        .iter()
        .map(|k| Value::String(k.name.clone()))
        .collect();

    let edge_labels: Vec<Value> = schema
        .edge_types
        .iter()
        .map(|e| Value::String(e.label.clone()))
        .collect();

    let node_kind_schema = if kind_names.is_empty() {
        serde_json::json!({ "type": "string" })
    } else {
        serde_json::json!({ "type": "string", "enum": kind_names })
    };

    let edge_label_schema = if edge_labels.is_empty() {
        serde_json::json!({ "type": "string" })
    } else {
        serde_json::json!({ "type": "string", "enum": edge_labels })
    };

    let json_schema = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "SpecForge Graph Protocol",
        "description": format!(
            "Graph Protocol schema v{} — auto-generated from extension registries",
            schema.schema_version
        ),
        "type": "object",
        "required": ["format_version", "schema_version", "nodes", "edges"],
        "properties": {
            "format_version": {
                "type": "string",
                "const": "2.0"
            },
            "schema_version": {
                "type": "string"
            },
            "schema": {
                "type": "object",
                "description": "Embedded Graph Protocol schema"
            },
            "nodes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["id", "kind", "file", "line", "fields"],
                    "properties": {
                        "id": { "type": "string" },
                        "kind": node_kind_schema,
                        "title": { "type": "string" },
                        "file": { "type": "string" },
                        "line": { "type": "integer" },
                        "fields": { "type": "object" }
                    }
                }
            },
            "edges": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["source", "target", "label"],
                    "properties": {
                        "source": { "type": "string" },
                        "target": { "type": "string" },
                        "label": edge_label_schema
                    }
                }
            }
        }
    });

    serde_json::to_string_pretty(&json_schema).expect("JSON Schema serialization cannot fail")
}
