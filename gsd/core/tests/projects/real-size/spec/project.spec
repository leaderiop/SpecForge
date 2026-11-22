project specforge "SpecForge" {
  name        "SpecForge"
  what        "A compiler that turns .spec files into a typed, validated graph agents and people read"
  core_value  "The graph is the one state: every read, check and edit goes through it"
  context     "A Rust workspace: a kernel, extensions compiled to WebAssembly components, a CLI and an MCP server"
  constraints [
    {
      kind tech_stack
      what "Rust, one workspace, extensions as wasm32-wasip2 components"
      why  "one toolchain, nothing to install"
    }
    {
      kind compatibility
      what "Linux, macOS and Windows"
      why  "the CLI ships as one binary per platform"
    }
  ]
}

requirement FEATURE-047 "Graph Protocol Export" {
  category "Feature"
  tier     v1
  text     """
    The compiled graph is exported as Graph Protocol JSON: every node with
    its kind, title, fields, file and line, every edge with its label, and
    a format version. The export is deterministic: two compilations of the
    same files export the same bytes, so a diff of two exports is a diff of
    the specification.
  """
}

requirement FEATURE-120 "Self-Describing Graph Protocol" {
  category "Feature"
  tier     v1
  text     """
    The Graph Protocol JSON export contains nodes and edges but no schema
    describing what entity kinds exist, what fields they have, or what edge
    types connect them. Agents consuming the graph must rely on external
    documentation or heuristics to understand the graph structure, reducing
    first-attempt accuracy.

    Self-describing schema embedded in Graph Protocol exports. After all
    extension registries are populated, the compiler serializes the
    KindRegistry and FieldRegistry into a GraphProtocolSchema object.
    Each registered entity kind produces a SchemaEntityKind entry; each
    registered edge type produces a SchemaEdgeType entry. The schema is
    generated once per compilation and embedded as a top-level "schema"
    key in JSON exports (format_version 2.0). specforge schema CLI command
    and specforge://schema MCP resource provide standalone schema access
    for agent introspection. The schema dynamically reflects whichever
    extensions are installed — no static maintenance required.
  """
}

requirement FEATURE-048 "Graph Protocol Versioning" {
  category "Feature"
  tier     v1
  text     """
    The Graph Protocol schema evolves as extensions add entity kinds and edge
    types, but there is no versioning mechanism. Agents consuming the graph
    cannot negotiate compatible versions, and breaking changes are undetectable.

    Semantic versioning for the Graph Protocol schema, derived from the
    extensions: each export versions the schema against the one the last
    export recorded in .specforge/schema-cache.json, comparing every
    attribute. Additions bump the minor version, documentation and
    provenance the patch version, and anything a reader of the recorded
    schema relies on that is removed or reshaped (removed kinds, edge types
    or fields, a new required field, a changed field type, required flag,
    edge type or target kind, a lost enum value or edge end) the major
    version, warned as W053. One version names one document. A consumer
    may name the version it reads (--schema-version, the export tool's
    schema_version): it is admitted when a reader of that version reads the
    current schema, and refused otherwise. specforge schema --publish
    exports the schema as a standalone JSON Schema (draft 2020-12) for
    graph, context and brief exports.
  """
}
