// Output behaviors — serialization and export of the spec graph

use invariants/core
use invariants/validation
use invariants/zero-entity-core
use types/graph
use types/output
use types/diagnostics
use types/errors
use types/zero-entity-core
use ports/outbound
use ports/inbound
use events/compilation

// render_markdown_documentation moved to spec/extensions/markdown-renderer/behaviors.spec

behavior serialize_json_graph "Serialize JSON Graph" {
  invariants [graph_traversal_integrity, diagnostic_determinism, graph_schema_completeness]
  types      [Graph, GraphProtocolSchema, OutputFile, EmitterError]
  ports      [GraphSerializer, FileSystem]
  consumes   [validation_complete]
  produces   [render_complete]

  contract """
    When specforge render json is invoked, the system MUST serialize
    the entire in-memory graph to JSON conforming to the Graph Protocol
    schema. The JSON MUST contain all nodes, edges, and metadata. The
    output MUST include a schema_version field identifying the Graph
    Protocol version. The output MUST be valid JSON parseable by
    standard tools.
  """

  verify unit "JSON output contains all nodes"
  verify unit "JSON output contains all edges"
  verify unit "output is valid JSON"
  verify unit "output includes schema_version field"
  verify unit "empty graph produces valid JSON with empty nodes and edges arrays"
  verify unit "schema is included even for empty graph"
  verify integration "structural-only graph (zero extensions) produces valid Graph Protocol JSON with raw keywords in kind field"

}

// P7 Justification: DOT serialization is domain-agnostic graph visualization.
// It walks generic nodes and edges, delegating all kind-aware rendering
// (shapes, styles) to extensions via render_extension_defined_dot_shapes
// and render_extension_defined_edge_styles. See features/output.spec for
// the full P7 rationale.
behavior serialize_dot_visualization "Serialize DOT Visualization" {
  invariants [graph_traversal_integrity, diagnostic_determinism]
  types      [Graph, OutputFile, EmitterError]
  ports      [GraphSerializer, FileSystem]
  consumes   [validation_complete]
  produces   [render_complete]

  contract """
    When specforge render dot is invoked, the system MUST emit a DOT format
    graph compatible with Graphviz. Nodes MUST be labeled with entity
    IDs and titles. Edges MUST be labeled with edge types. Node shape
    rendering MUST delegate to render_extension_defined_dot_shapes
    (behaviors/zero-entity-core.spec) which reads the dot_shape field
    from the KindRegistry entry for each entity kind. If no dot_shape
    is specified, the default shape MUST be "box".
  """

  verify unit "DOT output is valid Graphviz syntax"
  verify unit "nodes are labeled with IDs"
  verify unit "edges are labeled with types"
  verify unit "node shapes use extension-defined dot_shape"

}

behavior compute_traceability_chain "Compute Traceability Chain" {
  invariants [graph_traversal_integrity, diagnostic_determinism]
  types      [Graph, TraceChain, TraceLink]
  ports      [CompilerApi]
  consumes   [validation_complete]

  contract """
    When specforge trace is invoked with an entity ID, the system MUST
    traverse the graph both upstream and downstream from that entity
    following all registered edge types. The trace MUST show the full
    chain of connected entities regardless of their kind. Missing links
    in expected edges (as declared by extension manifests) MUST be flagged.
    A TraceLink with status "missing" indicates an edge type registered
    in an extension manifest that is NOT instantiated between the two
    entities in the current graph. This distinguishes from broken
    references (E001), which are caught during resolution.
  """

  verify unit "trace from entity shows upstream and downstream connections"
  verify unit "trace shows full chain depth"
  verify unit "missing link in chain is flagged"

}

// CLI query command (specforge stats) — no event produced. Output is terminal,
// not a pipeline signal.
behavior compute_project_statistics "Compute Project Statistics" {
  invariants [diagnostic_determinism, testable_entity_classification, zero_domain_knowledge_core]
  types      [Graph, KindRegistryEntry, ProjectStatistics, DiagnosticSummary]
  consumes   [validation_complete]

  contract """
    When specforge stats is invoked, the system MUST compute and display:
    entity counts by kind, coverage percentage, orphan count, and
    diagnostic summary. Statistics MUST be derived from the current
    graph state. Coverage percentage MUST be computed only over entity
    kinds with testable=true in the KindRegistry, not over all entities.
  """

  verify unit "stats reports correct entity counts"
  verify unit "stats reports coverage percentage"
  verify unit "stats reports orphan count"
  verify unit "stats reports diagnostic summary"

}

behavior print_diagnostics_structured "Print Diagnostics Structured" {
  invariants [multi_error_collection, diagnostic_determinism]
  types      [Diagnostic, DiagnosticBag]
  ports      [CompilerApi]
  consumes   [validation_complete]

  contract """
    All diagnostics MUST be formatted in a structured style: file path, line
    number, column, source context snippet, suggestion, and color-coded
    severity. Errors MUST be red, warnings yellow, info blue. The format
    MUST be consistent across all output modes.
  """

  verify unit "error diagnostic is formatted with file:line:col"
  verify unit "diagnostic includes context snippet"
  verify unit "suggestion is displayed when available"

}

behavior exit_code_reflects_diagnostic_severity "Exit Code Reflects Diagnostic Severity" {
  invariants [multi_error_collection]
  types      [DiagnosticBag]
  ports      [CompilerApi]
  consumes   [validation_complete]

  contract """
    specforge check MUST exit with code 0 if no errors exist. It MUST
    exit with code 1 if any error-level diagnostic exists. With --strict,
    warnings MUST also cause exit code 1.
  """

  verify unit "exit 0 with no errors"
  verify unit "exit 1 with errors"
  verify unit "exit 1 with warnings in strict mode"

}

behavior serialize_traceability_data "Serialize Traceability Data" {
  invariants [graph_traversal_integrity, diagnostic_determinism]
  types      [Graph, TraceChain, TraceLink, OutputFile]
  ports      [GraphSerializer, FileSystem]
  consumes   [validation_complete]
  produces   [render_complete]

  contract """
    When specforge trace is invoked without an entity ID, the system MUST
    compute and serialize the full traceability chain across all registered
    edge types from root entities to leaf entities as structured JSON graph
    traversal data. Gaps in the chain MUST be included in the output with
    a "missing" status. The output MUST conform to the Graph Protocol schema.
  """

  verify unit "full trace covers all root entities across registered edge types"
  verify unit "gaps in chain are highlighted"
  verify unit "output conforms to Graph Protocol schema"

}

behavior validate_agent_plan "Validate Agent Implementation Plan" {
  invariants [graph_traversal_integrity, diagnostic_determinism]
  types      [Graph, TraceChain, TraceLink, AgentPlan, AgentPlanEntry, PlanValidationResult, PlanValidationEntry]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [plan_validated]

  contract """
    When specforge trace --plan plan.json is invoked, the system MUST parse
    the plan file and validate it against the current spec graph. Every entity
    ID referenced in the plan MUST resolve to a declared entity in the graph;
    unresolvable IDs MUST produce an E001 diagnostic. Every testable entity
    in the graph MUST have a corresponding planned action in the plan; missing
    entries MUST be reported as warnings. Dependency order declared in the plan
    MUST be validated against the graph's edge structure; any ordering that
    contradicts the graph MUST produce a diagnostic. The output MUST be a
    structured JSON report listing validated entries, gaps, and ordering
    violations.
  """

  verify unit "plan with all valid entity IDs passes validation"
  verify unit "plan referencing nonexistent entity ID produces E001"
  verify unit "testable entity missing from plan produces warning"
  verify unit "plan dependency order contradicting graph produces diagnostic"
  verify unit "output is structured JSON"

}

// render_index_files and selective_render_by_entity_type moved to
// spec/extensions/markdown-renderer/behaviors.spec

behavior deterministic_output "Deterministic Output" {
  invariants [diagnostic_determinism]
  types      [OutputFile]
  ports      [CompilerApi]
  consumes   [validation_complete]

  contract """
    Given identical input .spec files, all output paths — file emitters
    (specforge render) and stdout emitters (specforge export, specforge query)
    — MUST produce byte-for-byte identical output. Output MUST NOT depend on
    filesystem iteration order, hashmap ordering, or timestamps.
  """

  verify property "same input produces identical output across runs"

}

behavior check_mode_for_ci "Check Mode for CI" {
  invariants [multi_error_collection, diagnostic_determinism]
  types      [DiagnosticBag]
  ports      [CompilerApi]
  consumes   [validation_complete]

  contract """
    specforge check MUST parse, resolve, and validate all .spec files
    without producing any output files. It MUST print diagnostics to
    stderr and exit with an appropriate exit code. This is the primary
    CI integration point.
  """

  verify unit        "check mode produces no output files"
  verify unit        "check mode prints diagnostics to stderr"
  verify integration "check mode works in CI environment"

}

behavior export_diagnostics_as_json "Export Diagnostics as JSON" {
  invariants [multi_error_collection, diagnostic_determinism]
  types      [DiagnosticBag, Diagnostic, DiagnosticFormat]
  ports      [CompilerApi]
  consumes   [validation_complete]

  contract """
    When specforge check --format=json is invoked, the system MUST output
    all diagnostics as a JSON array to stdout. Each diagnostic MUST include
    code, severity, message, file path, line, column, and optional suggestion.
    The JSON output MUST conform to a stable schema suitable for consumption
    by CI tools and AI agents. This format is an alternative to the default
    structured text output (file:line:col format). The --format=json flag MUST NOT affect exit
    code behavior — exit codes remain governed by
    exit_code_reflects_diagnostic_severity.
  """

  verify unit "diagnostics serialized as JSON array to stdout"
  verify unit "each diagnostic includes code, severity, message, file, line, column"
  verify unit "JSON output is valid and parseable"
  verify unit "exit code unaffected by format flag"
  verify unit "suggestion field included when available"

}

// ── Agent-Optimized Export (Principle 3: agents are first-class consumers) ──
// `specforge export` writes to stdout for agent/interactive consumption (context, brief, graph formats).
// `specforge render` writes files to disk for batch/CI output (json, dot, markdown via extensions).

behavior export_agent_context_format "Export Agent Context Format" {
  invariants [graph_traversal_integrity, diagnostic_determinism, graph_schema_completeness]
  types      [Graph, GraphProtocolSchema, OutputFile, AgentExportConfig, ProjectStatistics]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [export_complete]

  contract """
    When specforge export --format=context is invoked, the system MUST produce
    a token-optimized representation of the graph containing entity IDs,
    contracts, relationships, and coverage status. The output MUST omit
    verbose fields (full descriptions, prose) to minimize token consumption.
    The format MUST be valid JSON conforming to the Graph Protocol schema.
    The output MUST include a schema_version field identifying the Graph
    Protocol version. An optional --scope parameter MUST allow scoping to
    a subgraph rooted at a specific entity. If --scope references a
    non-existent entity ID, the system MUST emit an E001 diagnostic
    and exit with code 1. When coverage metadata is available from
    compute_project_statistics, the context export MUST include
    coverage_pct and testable_entity_count in the graph metadata.
  """

  verify unit "context format includes entity IDs and contracts"
  verify unit "context format omits verbose prose fields"
  verify unit "scoped export returns only reachable subgraph"
  verify unit "non-existent scope entity produces E001 and exit code 1"
  verify unit "output conforms to Graph Protocol schema"
  verify unit "output includes schema_version field"

}

behavior export_agent_brief_format "Export Agent Brief Format" {
  invariants [graph_traversal_integrity, diagnostic_determinism, graph_schema_completeness]
  types      [Graph, GraphProtocolSchema, OutputFile, AgentExportConfig]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [export_complete]

  contract """
    When specforge export --format=brief is invoked, the system MUST produce
    a minimal representation containing only entity IDs, kinds, titles, and
    their direct relationships. This is the lowest-token-cost format for
    agent discovery tasks. The output MUST be valid JSON conforming to the
    Graph Protocol schema. The output MUST include a schema_version field
    identifying the Graph Protocol version.
  """

  verify unit "brief format includes only IDs, kinds, titles, and edges"
  verify unit "brief format is smaller than context format"
  verify unit "output conforms to Graph Protocol schema"
  verify unit "output includes schema_version field"

}

behavior export_agent_graph_format "Export Agent Graph Format" {
  invariants [graph_traversal_integrity, diagnostic_determinism, graph_schema_completeness]
  types      [Graph, GraphProtocolSchema, OutputFile, AgentExportConfig]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [export_complete]

  contract """
    When specforge export --format=graph is invoked, the system MUST produce
    the complete entity graph as JSON conforming to the Graph Protocol schema.
    This is the full-fidelity format containing all nodes, edges, fields, and
    metadata. Unlike specforge render json (which writes files to disk), this
    command writes to stdout for agent consumption. An optional --scope parameter
    MUST allow scoping to a subgraph rooted at a specific entity. If --scope
    references a non-existent entity ID, the system MUST emit an E001
    diagnostic and exit with code 1. The output MUST include a schema_version
    field identifying the Graph Protocol version.
  """

  verify unit "graph format includes all nodes and edges"
  verify unit "graph format includes all fields and metadata"
  verify unit "scoped export returns only reachable subgraph"
  verify unit "non-existent scope entity produces E001 and exit code 1"
  verify unit "output conforms to Graph Protocol schema"
  verify unit "output includes schema_version field"
  verify integration "structural-only graph exports valid JSON with raw keyword strings as entity kinds"
}

behavior query_graph_multi_resolution "Query Graph at Multiple Resolutions" {
  invariants [graph_traversal_integrity, diagnostic_determinism, graph_schema_completeness]
  types      [Graph, GraphProtocolSchema, AgentExportConfig]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [graph_queried]

  contract """
    When specforge query is invoked with an entity ID and a --depth parameter,
    the system MUST return the subgraph at the requested resolution level.
    Depth 0 returns only the entity itself. Depth 1 returns direct neighbors.
    Depth N returns all entities within N hops. An optional --kind parameter
    MUST filter results to only include entities of the specified kind(s),
    while preserving edges that connect through filtered-out nodes. Multiple
    --kind values MAY be specified (e.g., --kind=behavior --kind=invariant).
    The output MUST be valid JSON conforming to the Graph Protocol schema
    with a schema_version field. This enables agents to request exactly the
    context slice they need without consuming the full graph.
  """

  verify unit "depth 0 returns only the target entity"
  verify unit "depth 1 returns direct neighbors"
  verify unit "depth N returns all entities within N hops"
  verify unit "kind filter restricts results to specified entity kinds"
  verify unit "multiple kind filters combine as union"
  verify unit "output conforms to Graph Protocol schema"
  verify unit "output includes schema_version field"
  verify property "querying same entity at same depth produces identical subgraph"

}

// ── Token Economics (Principle 3: agents are first-class consumers) ────

behavior enforce_token_budget "Enforce Token Budget" {
  invariants [graph_traversal_integrity, diagnostic_determinism, token_budget_subgraph_consistency]
  types      [AgentExportConfig, TokenBudgetResult, Graph, OutputFile, ExportResult]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [token_budget_applied]

  contract """
    When specforge export is invoked with --max-tokens, the system MUST
    estimate the output token count before serialization. If the estimate
    exceeds the budget, the system MUST apply a truncation strategy:
    prioritize entities by graph centrality, truncate low-priority entities,
    and include a TokenBudgetResult in the output metadata. The strategy
    field MUST indicate which approach was used (truncate, prioritize, or
    error). If no --max-tokens is specified, this behavior MUST be skipped.
    The TokenBudgetResult MUST list any truncated entity IDs so agents can
    request them individually via specforge query. The default strategy MUST
    be `prioritize`. The default centrality metric MUST be degree centrality
    (count of incoming + outgoing edges). Both MUST be overridable via
    AgentExportConfig.
  """

  verify unit "output within budget includes all entities"
  verify unit "output exceeding budget truncates low-priority entities"
  verify unit "TokenBudgetResult included in metadata when budget applied"
  verify unit "truncated_entities lists omitted entity IDs"
  verify unit "no --max-tokens skips budget enforcement"
  verify integration "export with max_tokens produces output within budget and includes metadata"

}
