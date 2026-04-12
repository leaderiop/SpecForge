# Model Command Architecture

## Data Flow

```
GraphProtocolSchema (assembled by KindRegistry + EdgeRegistry + FieldRegistry)
        |
        v
  ModelIntermediate (ERD-oriented IR)
        |
        +-->  MarkdownRenderer  -> String
        +-->  MermaidRenderer   -> String
        +-->  DotRenderer       -> String  (schema-level, distinct from instance-level dot.rs)
        +-->  JsonRenderer      -> String  (serde_json)
        +-->  DbmlRenderer      -> String
```

The `GraphProtocolSchema` is the input. It already contains:
- `entity_kinds: Vec<SchemaEntityKind>` with name, source_extension, testable, fields
- `edge_types: Vec<SchemaEdgeType>` with label, source_extension, source_kinds, target_kinds
- `extensions: Vec<SchemaExtensionInfo>` with name, version

The `ModelIntermediate` enriches this with:
- Cardinality inference (from field types)
- Synthetic `id` primary key per entity
- Field classification (primary key, foreign key, data)
- Extension grouping metadata

## Crate Locations

```
specforge-emitter/
  src/
    model/
      mod.rs           -- ModelIntermediate struct, builder, ModelOptions, render() dispatcher
      markdown.rs      -- Markdown renderer
      mermaid.rs       -- Mermaid erDiagram renderer
      dot.rs           -- DOT schema-level renderer (HTML-like labels)
      json.rs          -- ERD JSON renderer
      dbml.rs          -- DBML renderer
      cardinality.rs   -- Cardinality inference from field types

specforge-cli/
  src/
    model.rs           -- CLI command handler (clap args -> ModelOptions -> render)

specforge-mcp/
  src/
    tools/
      model.rs         -- MCP tool handler (JSON args -> ModelOptions -> render)
```

## Key Types

```rust
/// ERD-oriented intermediate representation.
/// Built from GraphProtocolSchema, consumed by all five renderers.
pub struct ModelIntermediate {
    pub model_version: String,
    pub extensions: Vec<ModelExtension>,
    pub entities: Vec<ModelEntity>,
    pub relationships: Vec<ModelRelationship>,
}

pub struct ModelExtension {
    pub name: String,
    pub version: String,
    pub entity_count: usize,
    pub edge_count: usize,
}

pub struct ModelEntity {
    pub name: String,
    pub extension: String,
    pub description: Option<String>,
    pub fields: Vec<ModelField>,
}

pub struct ModelField {
    pub name: String,
    pub field_type: ModelFieldType,
    pub required: bool,
    pub description: Option<String>,
    pub default_value: Option<String>,
    pub enum_values: Option<Vec<String>>,
    pub is_primary_key: bool,
    pub references: Option<String>,  // target entity name for reference fields
}

pub enum ModelFieldType {
    String,
    Integer,
    Boolean,
    Enum,
    StringList,
    Reference,
    ReferenceList,
    Block,
}

pub struct ModelRelationship {
    pub name: String,        // edge type label
    pub source: String,      // source entity kind
    pub target: String,      // target entity kind
    pub cardinality: Cardinality,
    pub source_field: Option<String>,  // field on source that declares this edge
    pub description: Option<String>,
}

pub enum Cardinality {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

pub struct ModelOptions {
    pub format: ModelFormat,
    pub group_by: GroupBy,
    pub fields: FieldLevel,
    pub extension_filter: Option<String>,
    pub kind_filter: Vec<String>,
    pub root: Option<String>,
    pub depth: Option<usize>,
}

pub enum ModelFormat { Markdown, Mermaid, Dot, Json, Dbml }
pub enum GroupBy { Extension, None }
pub enum FieldLevel { None, Keys, All }
```

## Cardinality Inference Algorithm

For each edge type in the schema:

1. Find the source entity kind (from `edge_type.source_kinds`)
2. Search the source entity's fields for one where `field.edge == edge_type.label`
3. Based on the field type:
   - `reference` -> `ManyToOne` (many source entities can point to one target)
   - `reference_list` -> `ManyToMany` (many source entities can point to many targets)
4. If no matching field found -> default to `ManyToMany` (safest assumption)

The `ManifestField` struct has `edge: Option<String>` and `target_kind: Option<String>` which provide the link between fields and edge types.

## Integration Points

### With existing `specforge schema`

`specforge schema` outputs the raw `GraphProtocolSchema` as JSON. `specforge model` consumes the same schema but transforms it into an ERD-oriented IR before rendering. They share the same input but produce fundamentally different outputs.

### With existing `specforge export --format=dot`

The existing DOT format in `dot.rs` renders **instances** (actual entities and edges). The model DOT renderer renders the **schema** (entity kinds as record-like nodes, edge types as labeled arrows). These are complementary.

### With MCP

The `specforge.model` MCP tool follows the same pattern as `specforge.export` and `specforge.schema`: parse JSON arguments, build options, call the shared render function, return the string result.
