// Output behaviors — schema generation, versioning, MCP graph serving, embeddings

use invariants/core
use invariants/extensions
use invariants/validation
use invariants/zero-entity-core
use types/graph
use types/output
use types/zero-entity-core
use ports/inbound
use ports/outbound
use events/compilation

// ── Self-Describing Graph Protocol Schema ─────────────────────────────

behavior generate_schema_from_registries "Generate Schema From Registries" {
  invariants [graph_schema_completeness, diagnostic_determinism, registry_population_before_validation, zero_domain_knowledge_core]
  types      [GraphProtocolSchema, SchemaEntityKind, SchemaEdgeType, SchemaField, KindRegistryEntry, FieldRegistryEntry]
  ports      [CompilerApi]
  consumes   [registries_populated]
  produces   [schema_generated]

  contract """
    After all extension registries are populated, the system MUST serialize
    the KindRegistry and FieldRegistry into a GraphProtocolSchema object.
    Each registered entity kind MUST produce a SchemaEntityKind entry with
    its fields, testable flag, and singleton flag. Each registered edge type
    MUST produce a SchemaEdgeType entry. The schema MUST be generated once
    per compilation and cached for reuse by embed_schema_in_export and
    serve_schema_resource. In watch mode or LSP mode, the schema MUST be
    regenerated on each compilation cycle — "once per compilation" means
    once per compile cycle, not once for the lifetime of the process. The
    cache MUST be invalidated at the start of each new compilation cycle.
  """

  verify unit "schema includes all registered entity kinds"
  verify unit "schema includes all registered edge types"
  verify unit "schema fields match FieldRegistry entries"
  verify unit "schema generated once per compilation and cached"
  verify unit "zero extensions produces valid empty schema"

}

behavior embed_schema_in_export "Embed Schema in Export" {
  invariants [graph_traversal_integrity, graph_schema_completeness, diagnostic_determinism, zero_domain_knowledge_core, schema_version_backward_compatibility]
  types      [Graph, GraphProtocolSchema, OutputFile]
  ports      [CompilerApi, FileSystem]
  consumes   [validation_complete]

  contract """
    When exporting the graph as JSON (specforge export or specforge render json),
    the system MUST embed the GraphProtocolSchema as a top-level "schema" key
    in the output, placed before the "nodes" and "edges" keys. The
    format_version field MUST be set to "2.0" when schema is present. The
    --no-schema flag MUST suppress schema embedding for backward compatibility,
    in which case format_version remains "1.0". The schema MUST reflect
    the full project (all registered entity kinds and edge types), not
    just a scoped subgraph. This allows agents to understand the broader
    context even when operating on a scoped extraction. After embedding
    the schema in a JSON export, the system MUST also persist the current
    GraphProtocolSchema to `.specforge/schema-cache.json` for use by
    detect_breaking_schema_changes in subsequent compilations. The cache
    file MUST be overwritten atomically (write to temp, then rename).
  """

  verify unit "schema embedded as top-level key in JSON export"
  verify unit "format_version set to 2.0 with schema"
  verify unit "--no-schema suppresses schema and keeps format_version 1.0"
  verify unit "schema-cache.json updated after JSON export"

}

behavior serve_schema_resource "Serve Schema Resource" {
  invariants [graph_traversal_integrity, graph_schema_completeness, diagnostic_determinism, zero_domain_knowledge_core]
  types      [GraphProtocolSchema, SchemaEntityKind]
  ports      [CompilerApi]
  consumes   [validation_complete]

  contract """
    When specforge schema is invoked, the system MUST output the
    GraphProtocolSchema as JSON to stdout. An optional --kind filter MUST
    restrict output to a single entity kind's schema. In MCP server mode,
    the schema MUST be available as the specforge://schema resource for
    agent introspection. The schema MUST always reflect the current
    compilation state.
  """

  verify unit "specforge schema outputs full schema as JSON"
  verify unit "--kind filter restricts to single entity kind"
  verify unit "MCP resource specforge://schema returns schema"
  verify unit "schema reflects current compilation state"

}

// ── MCP Graph Resources (Principle 3: agents are first-class consumers) ──

// This behavior provides the data layer for MCP resource exposure. The MCP
// behaviors (expose_graph_as_mcp_resource, expose_context_as_mcp_resource,
// expose_brief_as_mcp_resource) delegate to this behavior for graph serialization.
behavior serve_graph_resource "Serve Graph Resource via MCP" {
  invariants [graph_traversal_integrity, graph_schema_completeness, diagnostic_determinism]
  types      [Graph, GraphProtocolSchema, AgentExportConfig, DiagnosticSummary]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [graph_resource_served]

  contract """
    In MCP server mode, the system MUST expose graph data as MCP resources
    for agent introspection without requiring CLI invocation. The
    specforge://graph resource MUST return the full Graph Protocol JSON
    (equivalent to specforge export --format=graph). The
    specforge://context resource MUST return the token-optimized context
    format (equivalent to specforge export --format=context). The
    specforge://brief resource MUST return the minimal brief format
    (equivalent to specforge export --format=brief). All three resources
    MUST support a scope query parameter to restrict output to a subgraph
    rooted at a specific entity (e.g., specforge://context?scope=auth_login).
    All resources MUST include the embedded GraphProtocolSchema and a
    schema_version field. Resources MUST reflect the current compilation
    state and update when the graph changes. When compilation fails, the
    server MUST return an error resource containing a diagnostic summary
    instead of an empty resource. The error resource MUST include the
    DiagnosticSummary and a top-level "error" key set to true. When zero
    extensions are installed, resources MUST return the structural-only
    graph containing raw keyword strings as entity kinds.
  """

  verify unit "specforge://graph returns full Graph Protocol JSON"
  verify unit "specforge://context returns token-optimized format"
  verify unit "specforge://brief returns minimal format"
  verify unit "scope query parameter restricts to subgraph"
  verify unit "resources include embedded schema and schema_version"
  verify unit "resources reflect current compilation state"
  verify unit "compilation failure returns error resource with diagnostic summary"

}

// ── Graph Protocol Versioning ─────────────────────────────────────

behavior negotiate_schema_version "Negotiate Schema Version" {
  invariants [graph_schema_completeness, diagnostic_determinism, schema_version_backward_compatibility]
  types      [SchemaVersion, SchemaCompatibility, GraphProtocolSchema, ExportResult]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [schema_version_negotiated]

  contract """
    When an agent requests a specific Graph Protocol schema version, the
    system MUST check the requested version against the supported compatibility
    range (supported_min to supported_max). If compatible, the system MUST
    resolve the nearest compatible version and use it for the export. If
    incompatible, the system MUST emit an E027 diagnostic with the supported
    range and reject the request. The default behavior when no version is
    requested MUST use the latest supported version. Consumers MAY request
    a specific schema version via the --schema-version CLI flag or the
    schema_version query parameter in MCP resource URIs.
  """

  verify unit "compatible version within range is resolved"
  verify unit "incompatible version produces E027 with supported range"
  verify unit "no version requested defaults to latest"
  verify unit "--schema-version CLI flag selects requested version"
  verify unit "schema_version MCP query parameter selects requested version"

}

behavior detect_breaking_schema_changes "Detect Breaking Schema Changes" {
  invariants [graph_schema_completeness, diagnostic_determinism, schema_version_backward_compatibility]
  types      [SchemaVersion, SchemaMigration, GraphProtocolSchema]
  ports      [CompilerApi]
  consumes   [schema_generated]
  produces   [schema_breaking_change_detected]

  contract """
    When the Graph Protocol schema version changes between compilations, the
    system MUST detect breaking changes by comparing the previous and current
    schemas. Removed entity kinds, changed edge type semantics, and new
    required fields MUST be classified as breaking. Added optional fields and
    new entity kinds MUST be classified as non-breaking. The system MUST emit
    a SchemaMigration record describing the changes. Breaking changes MUST
    only occur on major version increments. The previous schema version MUST be
    read from the most recent Graph Protocol export file in the output directory,
    or from a .specforge/schema-cache.json file maintained by the compiler. If
    no previous schema exists, all changes are treated as non-breaking (initial
    export). The `.specforge/schema-cache.json` file is authoritative. The
    output directory export is a fallback source used only when the cache
    file does not exist. When no previous schema is found and the project
    has been exported before (output directory contains prior exports), the
    system SHOULD emit an I008 info diagnostic indicating the schema cache
    was not found and breaking change detection was skipped.
  """

  verify unit "removed entity kind detected as breaking"
  verify unit "added optional field detected as non-breaking"
  verify unit "new required field detected as breaking"
  verify unit "SchemaMigration record emitted on version change"
  verify unit "no previous schema treats all changes as non-breaking"
  verify unit "missing cache with prior exports emits I008 info diagnostic"

}

behavior publish_schema_specification "Publish Schema Specification" {
  invariants [graph_schema_completeness, diagnostic_determinism, registry_api_openness]
  types      [GraphProtocolSchema, OutputFile]
  ports      [CompilerApi, FileSystem]
  consumes   [validation_complete]
  produces   [render_complete]

  contract """
    When specforge schema --publish is invoked, the system MUST export the
    current GraphProtocolSchema as a standalone JSON Schema document suitable
    for third-party consumers. The output MUST be a valid JSON Schema
    (draft 2020-12 or later) that describes the Graph Protocol format
    including all registered entity kinds, edge types, and field definitions.
    The published schema MUST be usable by any JSON Schema validator to
    validate Graph Protocol exports. The schema MUST include a $schema
    meta-reference identifying the JSON Schema draft version used.
  """

  verify unit "published schema is valid JSON Schema"
  verify unit "published schema describes all registered entity kinds"
  verify unit "published schema describes all edge types"
  verify unit "third-party validator can use published schema"
  verify unit "published schema validates known-good export"

}

// Entity embeddings moved to spec/extensions/embeddings/behaviors.spec
