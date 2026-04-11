// Output behaviors — serialization and export of the spec graph

use "invariants/core"
use "invariants/validation"
use "invariants/zero-entity-core"
use "types/graph"
use "types/output"
use "types/diagnostics"
use "types/errors"
use "types/zero-entity-core"
use "ports/outbound"
use "ports/inbound"
use "events/compilation"
// render_markdown_documentation moved to spec/extensions/markdown-renderer/behaviors.spec

behavior serialize_json_graph "Serialize JSON Graph" {
  invariants [graph_traversal_integrity, diagnostic_determinism, graph_schema_completeness, zero_domain_knowledge_core]
  category   query
  types      [Graph, GraphProtocolSchema, OutputFile, EmitterError]
  ports      [GraphSerializer, FileSystem]
  consumes   [validation_complete]
  produces   [render_complete]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming all diagnostics are collected and the graph is ready for serialization"
  }

  ensures {
    all_nodes_serialized "JSON output contains exactly one entry per graph node"
    all_edges_serialized "JSON output contains exactly one entry per graph edge"
    schema_version_present "Output includes a schema_version field identifying the Graph Protocol version"
    valid_json_produced "Output is valid JSON parseable by standard tools"
    render_complete_emitted "render_complete event is emitted after successful serialization"
  }

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
  verify contract "requires/ensures consistency for JSON graph serialization"

}

// P7 Justification: DOT serialization is domain-agnostic graph visualization.
// It walks generic nodes and edges, delegating all kind-aware rendering
// (shapes, styles) to extensions via render_extension_defined_dot_shapes
// and render_extension_defined_edge_styles. See features/output.spec for
// the full P7 rationale.
behavior serialize_dot_visualization "Serialize DOT Visualization" {
  invariants [graph_traversal_integrity, diagnostic_determinism, zero_domain_knowledge_core]
  category   query
  types      [Graph, OutputFile, EmitterError]
  ports      [GraphSerializer, FileSystem]
  consumes   [validation_complete]
  produces   [render_complete]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming the graph is finalized and ready for visualization"
  }

  ensures {
    valid_dot_produced "Output is valid Graphviz DOT syntax"
    nodes_labeled "All nodes are labeled with entity IDs and titles"
    edges_labeled "All edges are labeled with edge types"
    render_complete_emitted "render_complete event is emitted after successful DOT serialization"
  }

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
  verify contract "requires/ensures consistency for DOT visualization"

}

behavior compute_traceability_chain "Compute Traceability Chain" {
  invariants [graph_traversal_integrity, diagnostic_determinism, zero_domain_knowledge_core]
  category   query
  types      [Graph, TraceChain, TraceLink]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [trace_chain_computed]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming entity references are resolved and the graph is traversable"
  }

  ensures {
    full_chain_traversed "Trace covers both upstream and downstream connections from the target entity"
    missing_links_flagged "Expected edges from extension manifests that are not instantiated are flagged with missing status"
    trace_chain_computed_emitted "trace_chain_computed event is emitted after successful traversal"
  }

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
  verify contract "requires/ensures consistency for traceability chain computation"

}

// CLI query command (specforge stats) — no event produced. Output is terminal,
// not a pipeline signal.
behavior compute_project_statistics "Compute Project Statistics" {
  invariants [diagnostic_determinism, testable_entity_classification, zero_domain_knowledge_core]
  category   query
  types      [Graph, KindRegistryEntry, ProjectStatistics, DiagnosticSummary]
  consumes   [validation_complete]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming graph state is finalized for statistics computation"
  }

  ensures {
    entity_counts_produced "Statistics include entity counts grouped by kind"
    coverage_computed "Coverage percentage is computed over testable entities only"
    zero_testable_safe "Coverage is reported as 0% when testable_entity_count is zero, not as a division error"
  }

  contract """
    When specforge stats is invoked, the system MUST compute and display:
    entity counts by kind, coverage percentage, orphan count, and
    diagnostic summary. Statistics MUST be derived from the current
    graph state. Coverage percentage MUST be computed only over entity
    kinds with testable=true in the KindRegistry, not over all entities.
    An entity is "verified" if it has at least one verify declaration OR
    at least one file-reference field value. Coverage percentage is
    verified_entity_count / testable_entity_count. When testable_entity_count
    is zero, coverage MUST be reported as 0%, not as a division error.
  """

  verify unit "stats reports correct entity counts"
  verify unit "stats reports coverage percentage"
  verify unit "stats reports orphan count"
  verify unit "stats reports diagnostic summary"
  verify unit "coverage is 0% when testable_entity_count is zero"
  verify contract "requires/ensures consistency for project statistics computation"

}

behavior print_diagnostics_structured "Print Diagnostics Structured" {
  invariants [multi_error_collection, diagnostic_determinism, zero_domain_knowledge_core]
  category   command
  types      [Diagnostic, DiagnosticBag]
  ports      [CompilerApi]
  consumes   [validation_complete]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming all diagnostics are collected in the DiagnosticBag"
  }

  ensures {
    structured_format_enforced "Every diagnostic is formatted with file path, line number, column, source snippet, and color-coded severity"
    color_coding_applied "Errors are red, warnings yellow, info blue"
  }

  contract """
    All diagnostics MUST be formatted in a structured style: file path, line
    number, column, source context snippet, suggestion, and color-coded
    severity. Errors MUST be red, warnings yellow, info blue. The format
    MUST be consistent across all output modes.
  """

  verify unit "error diagnostic is formatted with file:line:col"
  verify unit "diagnostic includes context snippet"
  verify unit "suggestion is displayed when available"
  verify contract "requires/ensures consistency for structured diagnostic printing"

}

behavior exit_code_reflects_diagnostic_severity "Exit Code Reflects Diagnostic Severity" {
  invariants [multi_error_collection, diagnostic_determinism, zero_domain_knowledge_core]
  category   command
  types      [DiagnosticBag]
  ports      [CompilerApi]
  consumes   [validation_complete]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming all diagnostics are finalized"
  }

  ensures {
    exit_zero_on_clean "Exit code is 0 when no error-level diagnostics exist"
    exit_one_on_errors "Exit code is 1 when any error-level diagnostic exists"
    strict_mode_enforced "In --strict mode, warnings also cause exit code 1"
  }

  contract """
    specforge check MUST exit with code 0 if no errors exist. It MUST
    exit with code 1 if any error-level diagnostic exists. With --strict,
    warnings MUST also cause exit code 1.
  """

  verify unit "exit 0 with no errors"
  verify unit "exit 1 with errors"
  verify unit "exit 1 with warnings in strict mode"
  verify contract "requires/ensures consistency for exit code severity mapping"

}

behavior serialize_traceability_data "Serialize Traceability Data" {
  invariants [graph_traversal_integrity, diagnostic_determinism, zero_domain_knowledge_core]
  category   query
  types      [Graph, TraceChain, TraceLink, OutputFile]
  ports      [GraphSerializer, FileSystem]
  consumes   [validation_complete]
  produces   [render_complete]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming the graph is finalized and all edge types are registered"
  }

  ensures {
    full_trace_serialized "Full traceability chain from root to leaf entities is serialized as structured JSON"
    gaps_included "Missing links in the chain are included with a missing status"
    graph_protocol_conformance "Output conforms to the Graph Protocol schema"
    render_complete_emitted "render_complete event is emitted after successful serialization"
  }

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
  verify contract "requires/ensures consistency for traceability data serialization"

}

behavior validate_agent_plan "Validate Agent Implementation Plan" {
  invariants [graph_traversal_integrity, diagnostic_determinism, zero_domain_knowledge_core]
  category   validation
  types      [Graph, TraceChain, TraceLink, AgentPlan, AgentPlanEntry, PlanValidationResult, PlanValidationEntry]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [plan_validated]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming the spec graph is finalized for plan comparison"
  }

  ensures {
    unresolvable_ids_diagnosed "Every plan entity ID that does not resolve to a declared entity produces an E001 diagnostic"
    missing_entries_warned "Every testable entity missing from the plan produces a warning"
    ordering_validated "Plan dependency order is validated against graph edge structure"
    structured_report_produced "Output is a structured JSON report listing validated entries, gaps, and ordering violations"
    plan_validated_emitted "plan_validated event is emitted after validation completes"
  }

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
  verify contract "requires/ensures consistency for agent plan validation"

}

// render_index_files and selective_render_by_entity_type moved to
// spec/extensions/markdown-renderer/behaviors.spec

behavior deterministic_output "Deterministic Output" {
  invariants [diagnostic_determinism, zero_domain_knowledge_core, graph_traversal_integrity]
  category   command
  types      [OutputFile]
  ports      [CompilerApi]
  consumes   [validation_complete]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming input graph state is finalized"
  }

  ensures {
    byte_identical_output "Given identical input spec files, all output paths produce byte-for-byte identical output"
    no_nondeterministic_values "Output contains no timestamps, random values, or iteration-order-dependent content"
  }

  contract """
    Given identical input .spec files, all output paths — file emitters
    (specforge render) and stdout emitters (specforge export, specforge query)
    — MUST produce byte-for-byte identical output. Output MUST NOT depend on
    filesystem iteration order, hashmap ordering, or timestamps.
  """

  verify property "same input produces identical output across runs"
  verify unit "entity ordering is independent of hashmap iteration"
  verify unit "file emission order is independent of filesystem readdir order"
  verify unit "output contains no timestamps or non-deterministic values"
  verify contract "requires/ensures consistency for deterministic output"

}

behavior check_mode_for_ci "Check Mode for CI" {
  invariants [multi_error_collection, diagnostic_determinism, zero_domain_knowledge_core]
  category   validation
  types      [DiagnosticBag]
  ports      [CompilerApi]
  consumes   [validation_complete]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming all spec files have been parsed, resolved, and validated"
  }

  ensures {
    no_output_files_produced "Check mode produces zero output files on disk"
    diagnostics_to_stderr "All diagnostics are printed to stderr"
    appropriate_exit_code "Exit code reflects diagnostic severity per exit_code_reflects_diagnostic_severity"
  }

  contract """
    specforge check MUST parse, resolve, and validate all .spec files
    without producing any output files. It MUST print diagnostics to
    stderr and exit with an appropriate exit code. This is the primary
    CI integration point.
  """

  verify unit        "check mode produces no output files"
  verify unit        "check mode prints diagnostics to stderr"
  verify integration "check mode works in CI environment"
  verify contract "requires/ensures consistency for CI check mode"

}

behavior export_diagnostics_as_json "Export Diagnostics as JSON" {
  invariants [multi_error_collection, diagnostic_determinism, zero_domain_knowledge_core]
  category   query
  types      [DiagnosticBag, Diagnostic, DiagnosticFormat]
  ports      [CompilerApi]
  consumes   [validation_complete]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming all diagnostics are collected"
  }

  ensures {
    json_array_produced "All diagnostics are serialized as a JSON array to stdout"
    diagnostic_fields_complete "Each diagnostic includes code, severity, message, file path, line, column"
    exit_code_unaffected "The --format=json flag does not alter exit code behavior"
  }

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
  verify contract "requires/ensures consistency for JSON diagnostic export"

}

// ── Agent-Optimized Export (Principle 3: agents are first-class consumers) ──
// `specforge export` writes to stdout for agent/interactive consumption (context, brief, graph formats).
// `specforge render` writes files to disk for batch/CI output (json, dot, markdown via extensions).

behavior export_agent_context_format "Export Agent Context Format" {
  invariants [graph_traversal_integrity, diagnostic_determinism, graph_schema_completeness, zero_domain_knowledge_core]
  category   query
  types      [Graph, GraphProtocolSchema, OutputFile, AgentExportConfig, ProjectStatistics]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [export_complete]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming the graph is ready for agent export"
  }

  ensures {
    token_optimized_output "Output omits verbose prose fields to minimize token consumption"
    schema_version_present "Output includes a schema_version field identifying the Graph Protocol version"
    scope_enforced "When --scope is specified, only the reachable subgraph is returned"
    invalid_scope_diagnosed "Non-existent scope entity produces E001 and exit code 1"
    export_complete_emitted "export_complete event is emitted after successful export"
  }

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
  verify contract "requires/ensures consistency for agent context export"

}

behavior export_agent_brief_format "Export Agent Brief Format" {
  invariants [graph_traversal_integrity, diagnostic_determinism, graph_schema_completeness, zero_domain_knowledge_core]
  category   query
  types      [Graph, GraphProtocolSchema, OutputFile, AgentExportConfig]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [export_complete]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming the graph is ready for agent export"
  }

  ensures {
    minimal_representation "Output contains only entity IDs, kinds, titles, and direct relationships"
    schema_version_present "Output includes a schema_version field identifying the Graph Protocol version"
    export_complete_emitted "export_complete event is emitted after successful export"
  }

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
  verify contract "requires/ensures consistency for agent brief export"

}

behavior export_agent_graph_format "Export Agent Graph Format" {
  invariants [graph_traversal_integrity, diagnostic_determinism, graph_schema_completeness, zero_domain_knowledge_core]
  category   query
  types      [Graph, GraphProtocolSchema, OutputFile, AgentExportConfig]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [export_complete]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming the graph is ready for full-fidelity export"
  }

  ensures {
    full_fidelity_output "Output contains all nodes, edges, fields, and metadata"
    schema_version_present "Output includes a schema_version field identifying the Graph Protocol version"
    scope_enforced "When --scope is specified, only the reachable subgraph is returned"
    invalid_scope_diagnosed "Non-existent scope entity produces E001 and exit code 1"
    export_complete_emitted "export_complete event is emitted after successful export"
  }

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
  verify contract "requires/ensures consistency for agent graph export"
}

behavior query_graph_multi_resolution "Query Graph at Multiple Resolutions" {
  invariants [graph_traversal_integrity, diagnostic_determinism, graph_schema_completeness, zero_domain_knowledge_core]
  category   query
  types      [Graph, GraphProtocolSchema, AgentExportConfig]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [graph_queried]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming the graph is finalized and traversable"
  }

  ensures {
    depth_respected "Subgraph returned contains only entities within the requested hop distance"
    kind_filter_applied "When --kind is specified, results are restricted to the specified entity kinds"
    graph_protocol_conformance "Output is valid JSON conforming to the Graph Protocol schema with schema_version"
    graph_queried_emitted "graph_queried event is emitted after query completes"
  }

  contract """
    When specforge query is invoked with an entity ID and a --depth parameter,
    the system MUST return the subgraph at the requested resolution level.
    Depth 0 returns only the entity itself. Depth 1 returns direct neighbors.
    Depth N returns all entities within N hops. An optional --kind parameter
    MUST filter results to only include entities of the specified kind(s),
    while preserving edges that connect through filtered-out nodes. Multiple
    --kind values MAY be specified (e.g., --kind=alpha --kind=beta).
    Kind names are extension-defined; examples use placeholders.
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
  verify contract "requires/ensures consistency for multi-resolution graph query"

}

// ── Token Economics (Principle 3: agents are first-class consumers) ────

behavior enforce_token_budget "Enforce Token Budget" {
  invariants [graph_traversal_integrity, diagnostic_determinism, token_budget_subgraph_consistency, zero_domain_knowledge_core]
  category   query
  types      [AgentExportConfig, TokenBudgetResult, Graph, OutputFile, ExportResult]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [token_budget_applied]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming the graph is ready for token estimation"
  }

  ensures {
    budget_respected "Output token count does not exceed the specified --max-tokens budget"
    truncation_metadata_produced "TokenBudgetResult is included in output metadata when budget is applied"
    valid_subgraph_after_truncation "Remaining subgraph after truncation has no dangling edge references"
    token_budget_applied_emitted "token_budget_applied event is emitted after budget enforcement completes"
  }

  contract """
    When specforge export is invoked with --max-tokens, the system MUST
    estimate the output token count before serialization. If the estimate
    exceeds the budget, the system MUST apply a truncation strategy:
    prioritize entities by graph centrality, truncate low-priority entities,
    and include a TokenBudgetResult in the output metadata. The strategy
    field MUST indicate which approach was used (truncate, prioritize, or
    error). If no --max-tokens is specified, this behavior MUST be skipped.
    The TokenBudgetResult MUST list any truncated entity IDs so agents can
    request them individually via specforge query. When truncating entities
    from the budget, the system MUST remove all edges to and from truncated
    entities before serialization. The remaining subgraph MUST be a valid
    graph with no dangling edge references. The truncated_entities list in
    TokenBudgetResult records which entities were removed. The default
    strategy MUST be `prioritize`. The default centrality metric MUST be
    degree centrality (count of incoming + outgoing edges). Both MUST be
    overridable via AgentExportConfig.
  """

  verify unit "output within budget includes all entities"
  verify unit "output exceeding budget truncates low-priority entities"
  verify unit "TokenBudgetResult included in metadata when budget applied"
  verify unit "truncated_entities lists omitted entity IDs"
  verify unit "no --max-tokens skips budget enforcement"
  verify integration "export with max_tokens produces output within budget and includes metadata"
  verify unit "error strategy rejects export exceeding budget"
  verify contract "requires/ensures consistency for token budget enforcement"

}
