requirement FEATURE-057 "JSON and DOT Render" {
  category "Feature"
  tier     v1
  text     """
    External tools (dashboards, analyzers, visualizers) need machine-readable
    access to the spec graph. The graph must be exportable in standard formats.

    specforge render json exports the full graph as JSON conforming to the
    Graph Protocol schema — the versioned, open interchange format that any
    tool or agent framework can consume. specforge render dot exports a DOT
    format graph compatible with Graphviz for visualization.
  """
}

requirement FEATURE-130 "Traceability Serialization" {
  category "Feature"
  tier     v1
  text     """
    Architects and auditors need to trace entities upstream and downstream
    through the graph. Manual traceability matrices are error-prone and
    always stale. AI agents produce implementation plans that must be
    validated against the spec graph to ensure completeness and consistency.

    specforge trace computes and serializes traceability chains by traversing the
    graph across all registered edge types. Single-entity trace shows
    upstream and downstream links. Full trace shows every chain with gap
    detection. specforge trace --plan validates agent implementation plans
    against the graph, ensuring every referenced entity exists, every testable
    entity is covered, and dependency ordering is consistent.

    This closes the P5 feedback loop: declare intent (verify declarations and extension-declared test file fields) →
    trace gaps (specforge trace) → implement → collect proof
    (specforge collect) → trace again to confirm coverage.
  """
}

requirement FEATURE-001 "Agent-Optimized Export" {
  category "Feature"
  tier     v1
  text     """
    AI agents need structured, token-efficient context from the spec graph.
    Full JSON export wastes context window. Agents need scoped, multi-resolution
    access to exactly the subgraph relevant to their task. Agents operating
    in MCP server mode need resource-based access without CLI invocation.

    specforge export provides agent-optimized formats: context (token-optimized
    with contracts and relationships), brief (IDs and contracts only), and
    graph (full JSON). specforge query provides multi-resolution access via
    --scope, --depth, and --kind parameters for precise context slicing.
    In MCP server mode, specforge://graph, specforge://context, and
    specforge://brief resources provide the same formats as MCP resources
    for direct agent consumption.
  """
}

requirement FEATURE-112 "Review Export" {
  category "Feature"
  tier     v2
  text     """
    A reviewer who signs a specification (a QA lead, a qualified person) reads
    documents, not JSON. Every export format is JSON or Graphviz, so a review
    needs a hand conversion that can drop or misstate what the spec says.

    specforge export --format markdown writes the graph as one Markdown
    document: per entity its statements, fields, links with titles, the
    entities that reference it and its obligations. It reads only the field
    declarations, so every extension's kinds render without templates.
  """
}

requirement FEATURE-006 "CI Integration" {
  category "Feature"
  tier     v1
  text     """
    CI pipelines need a single command that validates all .spec files,
    exits with an appropriate code, and produces deterministic output
    suitable for automated checks.

    specforge check parses, resolves, and validates without writing files;
    nothing writes a file but the commands that say so.
    Exit code 0 for clean, 1 for errors. --strict treats warnings as errors.
    --format=json produces machine-readable diagnostic output for CI tools
    and AI agents. Output is deterministic for reproducible CI runs.

    Bridge: diagnostic_reporting (features/validation.spec) defines the
    diagnostic infrastructure that ci_integration consumes. Diagnostic codes,
    severity levels, and structured output format are shared between both
    features via the DiagnosticBag type.
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
    third-party validation. See also: spec_file_migration feature for
    migration tooling.
  """
}
