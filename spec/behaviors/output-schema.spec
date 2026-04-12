// Output behaviors — schema generation, versioning, MCP graph serving, embeddings

use "invariants/core"
use "invariants/extensions"
use "invariants/validation"
use "invariants/zero-entity-core"
use "types/graph"
use "types/output"
use "types/zero-entity-core"
use "ports/inbound"
use "ports/outbound"
use "events/compilation"
// ── Self-Describing Graph Protocol Schema ─────────────────────────────

behavior generate_schema_from_registries "Generate Schema From Registries" {
  invariants [graph_schema_completeness, diagnostic_determinism, registry_population_before_validation, zero_domain_knowledge_core]
  category   query
  types      [GraphProtocolSchema, SchemaEntityKind, SchemaEdgeType, SchemaField, KindRegistryEntry, FieldRegistryEntry, SchemaExtensionInfo]
  ports      [CompilerApi]
  consumes   [registries_populated]
  produces   [schema_generated]

  requires {
    registries_populated_fired "registries_populated event has fired, confirming KindRegistry and FieldRegistry are fully populated from all extension manifests"
  }

  ensures {
    all_kinds_in_schema "Every registered entity kind produces a SchemaEntityKind entry in the schema"
    all_edges_in_schema "Every registered edge type produces a SchemaEdgeType entry in the schema"
    schema_cached "Schema is generated once per compilation cycle and cached for reuse"
    schema_generated_emitted "schema_generated event is emitted after schema construction completes"
  }

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
  verify contract "requires/ensures consistency for schema generation from registries"

}

behavior embed_schema_in_export "Embed Schema in Export" {
  invariants [graph_traversal_integrity, graph_schema_completeness, diagnostic_determinism, zero_domain_knowledge_core, schema_version_backward_compatibility]
  category   query
  types      [Graph, GraphProtocolSchema, OutputFile]
  ports      [CompilerApi]
  // Barrier-join: schema pipeline must complete (schema_version_computed) AND
  // validation must complete (validation_complete) before export embedding.
  consumes   [schema_version_computed, validation_complete]

  requires {
    schema_version_computed_fired "schema_version_computed event has fired, confirming schema version is determined"
    validation_complete_fired "validation_complete event has fired, confirming the graph is ready for export"
  }

  ensures {
    schema_embedded "GraphProtocolSchema is embedded as a top-level schema key in the JSON output"
    format_version_set "format_version is set to 2.0 when schema is present, 1.0 when suppressed"
    full_project_schema "Embedded schema reflects the full project, not just a scoped subgraph"
  }

  contract """
    When exporting the graph as JSON (specforge export or specforge render json),
    the system MUST embed the GraphProtocolSchema as a top-level "schema" key
    in the output, placed before the "nodes" and "edges" keys. The
    format_version field MUST be set to "2.0" when schema is present. The
    --no-schema flag MUST suppress schema embedding for backward compatibility,
    in which case format_version remains "1.0". The schema MUST reflect
    the full project (all registered entity kinds and edge types), not
    just a scoped subgraph. This allows agents to understand the broader
    context even when operating on a scoped extraction.
  """

  verify unit "schema embedded as top-level key in JSON export"
  verify unit "format_version set to 2.0 with schema"
  verify unit "--no-schema suppresses schema and keeps format_version 1.0"
  verify contract "requires/ensures consistency for schema embedding in export"

}

behavior persist_schema_cache "Persist Schema Cache" {
  invariants [graph_schema_completeness, diagnostic_determinism, schema_version_backward_compatibility, zero_domain_knowledge_core]
  category   command
  types      [GraphProtocolSchema, SchemaCacheEntry]
  ports      [FileSystem]
  consumes   [schema_generated]
  produces   [schema_cache_persisted]

  requires {
    schema_generated_fired "schema_generated event has fired, confirming the GraphProtocolSchema is constructed and ready for persistence"
  }

  ensures {
    cache_written_atomically "Schema cache file is written atomically via temp file then rename"
    cache_always_updated "Cache is updated on every compilation regardless of whether a JSON export is performed"
    schema_cache_persisted_emitted "schema_cache_persisted event is emitted after successful cache write"
  }

  contract """
    After the GraphProtocolSchema is generated, the system MUST persist it
    to `.specforge/schema-cache.json` for use by detect_breaking_schema_changes
    in subsequent compilations. The cache file MUST be overwritten atomically
    (write to temp, then rename). This behavior runs independently of JSON
    export — the cache is updated on every compilation regardless of whether
    an export is performed, ensuring breaking change detection works for
    incremental rebuilds and watch mode.
  """

  verify unit "schema-cache.json written after schema generation"
  verify unit "cache file overwritten atomically via temp+rename"
  verify unit "cache updated even when no JSON export is performed"
  verify contract "requires/ensures consistency for schema cache persistence"

}

behavior serve_schema_resource "Serve Schema Resource" {
  invariants [graph_traversal_integrity, graph_schema_completeness, diagnostic_determinism, zero_domain_knowledge_core]
  category   command
  types      [GraphProtocolSchema, SchemaEntityKind]
  ports      [CompilerApi]
  consumes   [validation_complete]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming the schema reflects the current compilation state"
  }

  ensures {
    full_schema_output "specforge schema outputs the complete GraphProtocolSchema as JSON"
    kind_filter_supported "Optional --kind filter restricts output to a single entity kind"
    mcp_resource_available "In MCP server mode, schema is available as specforge://schema resource"
  }

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
  verify contract "requires/ensures consistency for schema resource serving"

}

// ── MCP Graph Resources (Principle 3: agents are first-class consumers) ──

// This behavior provides the data layer for MCP resource exposure. The MCP
// behaviors (expose_graph_as_mcp_resource, expose_context_as_mcp_resource,
// expose_brief_as_mcp_resource) delegate to this behavior for graph serialization.
// Forward-ref: MCP transport-level resource registration is in
// behaviors/mcp-server.spec (expose_graph_as_mcp_resource, etc.).
// Cross-feature: contributes to agent_export feature (features/output.spec).
behavior serve_graph_resource "Serve Graph Resource via MCP" {
  invariants [graph_traversal_integrity, graph_schema_completeness, diagnostic_determinism, zero_domain_knowledge_core]
  category   command
  types      [Graph, GraphProtocolSchema, AgentExportConfig, DiagnosticSummary]
  ports      [CompilerApi, McpProtocol]
  consumes   [validation_complete]
  produces   [graph_resource_served]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming graph data is available for MCP resource serving"
    mcp_server_available "MCP server mode is active and ready to serve resources"
  }

  ensures {
    three_formats_served "specforge://graph, specforge://context, and specforge://brief resources are all served"
    scope_parameter_supported "All three resources support a scope query parameter for subgraph extraction"
    schema_embedded_in_resources "All resources include the embedded GraphProtocolSchema and schema_version"
    error_resource_on_failure "Compilation failure returns an error resource with DiagnosticSummary instead of empty resource"
    graph_resource_served_emitted "graph_resource_served event is emitted after resources are served"
  }

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
  verify contract "requires/ensures consistency for MCP graph resource serving"

}

// ── Graph Protocol Versioning ─────────────────────────────────────

behavior negotiate_schema_version "Negotiate Schema Version" {
  invariants [graph_schema_completeness, diagnostic_determinism, schema_version_backward_compatibility, zero_domain_knowledge_core]
  category   command
  types      [SchemaVersion, SchemaCompatibility, GraphProtocolSchema, ExportResult]
  ports      [CompilerApi]
  consumes   [validation_complete, schema_breaking_change_detected]
  produces   [schema_version_negotiated]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming the graph is ready for version-negotiated export"
    schema_breaking_change_detected_fired "schema_breaking_change_detected event has fired, confirming breaking change classification is complete"
  }

  ensures {
    compatible_version_resolved "Requested version within supported range is resolved to nearest compatible version"
    incompatible_version_rejected "Incompatible version request produces E027 diagnostic with supported range"
    default_to_latest "When no version is requested, the latest supported version is used"
    schema_version_negotiated_emitted "schema_version_negotiated event is emitted after negotiation completes"
  }

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
  verify contract "requires/ensures consistency for schema version negotiation"

}

behavior detect_breaking_schema_changes "Detect Breaking Schema Changes" {
  invariants [graph_schema_completeness, diagnostic_determinism, schema_version_backward_compatibility, zero_domain_knowledge_core]
  category   validation
  types      [SchemaVersion, SchemaMigration, GraphProtocolSchema, SchemaCacheEntry, SchemaMigrationChange]
  ports      [CompilerApi, FileSystem]
  // Reads the previous schema from .specforge/schema-cache.json written by the
  // PRIOR compilation. Does NOT depend on persist_schema_cache in the current
  // compilation — it reads from disk, not from the current event stream.
  consumes   [schema_generated]
  produces   [schema_breaking_change_detected]

  requires {
    schema_generated_fired "schema_generated event has fired, confirming the current schema is available for comparison"
    filesystem_available "FileSystem port is available for reading .specforge/schema-cache.json from prior compilation"
  }

  ensures {
    breaking_changes_classified "Removed entity kinds, changed edge semantics, and new required fields are classified as breaking"
    nonbreaking_changes_classified "Added optional fields and new entity kinds are classified as non-breaking"
    migration_record_emitted "SchemaMigration record is emitted describing the changes"
    schema_breaking_change_detected_emitted "schema_breaking_change_detected event is emitted after classification completes"
  }

  contract """
    When the Graph Protocol schema version changes between compilations, the
    system MUST detect breaking changes by comparing the previous and current
    schemas. Removed entity kinds, changed edge type semantics, and new
    required fields MUST be classified as breaking. Added optional fields and
    new entity kinds MUST be classified as non-breaking. The system MUST emit
    a SchemaMigration record describing the changes. Breaking changes MUST
    only occur on major version increments. The previous schema version MUST be
    read from `.specforge/schema-cache.json` (maintained by persist_schema_cache),
    or from the most recent Graph Protocol export file in the output directory
    as a fallback. If no previous schema exists, all changes are treated as
    non-breaking (initial export). The `.specforge/schema-cache.json` file is
    authoritative. The output directory export is a fallback source used only
    when the cache file does not exist. When no previous schema is found and
    the project has been exported before (output directory contains prior
    exports), the system SHOULD emit an I016 info diagnostic indicating the
    schema cache was not found and breaking change detection was skipped.
  """

  verify unit "removed entity kind detected as breaking"
  verify unit "added optional field detected as non-breaking"
  verify unit "new required field detected as breaking"
  verify unit "SchemaMigration record emitted on version change"
  verify unit "no previous schema treats all changes as non-breaking"
  verify unit "missing cache with prior exports emits I016 info diagnostic"
  verify contract "requires/ensures consistency for breaking schema change detection"

}

behavior compute_schema_version "Compute Schema Version" {
  invariants [graph_schema_completeness, diagnostic_determinism, schema_version_backward_compatibility, zero_domain_knowledge_core]
  category   query
  types      [SchemaVersion, SchemaMigration, GraphProtocolSchema, SchemaCacheEntry]
  ports      [CompilerApi]
  // compute_schema_version depends on breaking change classification from
  // detect_breaking_schema_changes, not the raw schema_generated event.
  consumes   [schema_breaking_change_detected]
  produces   [schema_version_computed]

  requires {
    schema_breaking_change_detected_fired "schema_breaking_change_detected event has fired, confirming breaking change classification is available for version computation"
  }

  ensures {
    version_auto_computed "Version bump is computed automatically: major for breaking, minor for new kinds/edges, patch for metadata"
    first_compilation_baseline "First compilation without a cache produces version 1.0.0"
    version_attached "Computed version is attached to the GraphProtocolSchema before export"
    schema_version_computed_emitted "schema_version_computed event is emitted after version computation completes"
  }

  contract """
    After the schema is generated, the system MUST compare the current schema
    against the previous schema from .specforge/schema-cache.json. The version
    bump MUST be computed automatically: major for breaking changes (as
    classified by detect_breaking_schema_changes), minor for new entity kinds
    or edge types, patch for non-structural metadata changes. First compilation
    without a cache MUST produce version 1.0.0. The computed version MUST be
    attached to the GraphProtocolSchema before export.
  """

  verify unit "first compilation without cache produces version 1.0.0"
  verify unit "new entity kind triggers minor version bump"
  verify unit "removed entity kind triggers major version bump"
  verify unit "field metadata change triggers patch version bump"
  verify contract "requires/ensures consistency for schema version computation"

}

behavior publish_schema_specification "Publish Schema Specification" {
  invariants [graph_schema_completeness, diagnostic_determinism, registry_api_openness, zero_domain_knowledge_core]
  category   command
  types      [GraphProtocolSchema, OutputFile]
  ports      [CompilerApi, FileSystem]
  consumes   [schema_version_computed, validation_complete]
  produces   [render_complete]

  requires {
    schema_version_computed_fired "schema_version_computed event has fired, confirming the schema version is determined"
    validation_complete_fired "validation_complete event has fired, confirming all entity kinds and edge types are registered"
  }

  ensures {
    valid_json_schema_produced "Output is a valid JSON Schema document (draft 2020-12 or later)"
    all_kinds_described "Published schema describes all registered entity kinds and edge types"
    third_party_usable "Published schema is usable by any JSON Schema validator to validate Graph Protocol exports"
    render_complete_emitted "render_complete event is emitted after schema publication"
  }

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
  verify contract "requires/ensures consistency for schema specification publication"

}

// Entity embeddings moved to spec/extensions/embeddings/behaviors.spec
