// Output behaviors — serialization and export of the spec graph

use "events/compilation"
use "features/zero-entity-core"
use "invariants/core"
use "invariants/validation"
use "invariants/zero-entity-core"
use "ports/inbound"
use "ports/outbound"
use "types/diagnostics"
use "types/errors"
use "types/graph"
use "types/output"
use "types/zero-entity-core"

// render_markdown_documentation moved to spec/extensions/markdown-renderer/behaviors.spec

behavior serialize_json_graph "Serialize JSON Graph" {
  features   [json_and_dot_render]
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    graph_schema_completeness,
    zero_domain_knowledge_core,
  ]
  category   query
  types      [Graph, GraphProtocolSchema, OutputFile, EmitterError, OutputFormat]
  ports      [GraphSerializer, FileSystem]
  consumes   [validation_complete]
  produces   [render_complete]
  requires {
    validation_complete_fired "validation_complete event has fired, confirming all diagnostics are collected and the graph is ready for serialization"
  }
  ensures {
    all_nodes_serialized    "JSON output contains exactly one entry per graph node"
    all_edges_serialized    "JSON output contains exactly one entry per graph edge"
    schema_version_present  "Output includes a schema_version field identifying the Graph Protocol version"
    valid_json_produced     "Output is valid JSON parseable by standard tools"
    render_complete_emitted "render_complete event is emitted after successful serialization"
  }
  contract   """
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
  verify contract "Serialize JSON Graph: JSON graph serialization holds — validation_complete_fired, all_nodes_serialized, all_edges_serialized, schema_version_present, valid_json_produced, render_complete_emitted"
}

// P7 Justification: DOT serialization is domain-agnostic graph visualization.
// It walks generic nodes and edges, delegating all kind-aware rendering
// (shapes, styles) to extensions via render_extension_defined_dot_shapes
// and render_extension_defined_edge_styles. See features/output.spec for
// the full P7 rationale.
behavior serialize_dot_visualization "Serialize DOT Visualization" {
  features   [json_and_dot_render]
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
    valid_dot_produced      "Output is valid Graphviz DOT syntax"
    nodes_labeled           "All nodes are labeled with entity IDs and titles"
    edges_labeled           "All edges are labeled with edge types"
    render_complete_emitted "render_complete event is emitted after successful DOT serialization"
  }
  contract   """
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
  verify contract "Serialize DOT Visualization: DOT visualization holds — validation_complete_fired, valid_dot_produced, nodes_labeled, edges_labeled, render_complete_emitted"
  verify unit "clusters group by declaring extension"
  verify unit "kind filter drops other kinds"
  verify unit "labels toggle emits bare IDs"
}

behavior compute_traceability_chain "Compute Traceability Chain" {
  features   [traceability_serialization]
  invariants [graph_traversal_integrity, diagnostic_determinism, zero_domain_knowledge_core]
  category   query
  types      [Graph, TraceChain, TraceLink, TraceLinkStatus]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [trace_chain_computed]
  requires {
    validation_complete_fired "validation_complete event has fired, confirming entity references are resolved and the graph is traversable"
  }
  ensures {
    full_chain_traversed         "Trace covers both upstream and downstream connections from the target entity"
    missing_links_flagged        "Expected edges from extension manifests that are not instantiated are flagged with missing status"
    trace_chain_computed_emitted "trace_chain_computed event is emitted after successful traversal"
  }
  contract   """
    When specforge trace is invoked with an entity ID, the system MUST
    traverse the graph both upstream and downstream from that entity
    following all registered edge types. The trace MUST show the full
    chain of connected entities regardless of their kind. Missing links
    in expected edges (as declared by extension manifests) MUST be flagged.
    A TraceLink with status "missing" indicates an edge type registered
    in an extension manifest that is NOT instantiated between the two
    entities in the current graph. This distinguishes from broken
    references (E003), which are caught during resolution.
    The JSON trace output MUST carry the Graph Protocol schema_version.
    Tracing an entity the graph does not have MUST be E003, naming the
    closest entity when one is near.
  """
  verify unit "trace from entity shows upstream and downstream connections"
  verify unit "trace shows full chain depth"
  verify unit "missing link in chain is flagged"
  verify unit "trace output includes schema version"
  verify unit "tracing an entity the graph lacks is E003 naming the closest entity"
  verify contract "Compute Traceability Chain: traceability chain computation holds — validation_complete_fired, full_chain_traversed, missing_links_flagged, trace_chain_computed_emitted"
}

// CLI query command (specforge stats). Read-only: produces []. Output is terminal,
// not a pipeline signal.
behavior compute_project_statistics "Compute Project Statistics" {
  invariants [diagnostic_determinism, testable_entity_classification, zero_domain_knowledge_core]
  category   query
  types      [Graph, KindRegistryEntry, ProjectStatistics, DiagnosticSummary, EntityKindCount]
  consumes   [validation_complete]
  features   [extension_driven_coverage]
  requires {
    validation_complete_fired "validation_complete event has fired, confirming graph state is finalized for statistics computation"
  }
  ensures {
    entity_counts_produced "Statistics include entity counts grouped by kind"
    coverage_computed      "Coverage percentage is computed over testable entities only"
    zero_testable_safe     "Coverage is reported as 0% when testable_entity_count is zero, not as a division error"
  }
  contract   """
    When specforge stats is invoked, the system MUST compute and display:
    entity counts by kind, the declared percentage, the proof percentage
    when tests are recorded, orphan count, and diagnostic summary. Statistics MUST be derived from the current
    graph state. Coverage percentage MUST be computed only over entity
    kinds with testable=true in the KindRegistry, not over all entities,
    and without the entities W004 exempts that declare no obligations
    (union types, abstract entities, governance kinds): the coverage
    pass's testable count.
    An entity is "declared" when it has at least one verify statement.
    The declared percentage is declared testable entities /
    testable_entity_count; coverage percentage is its deprecated alias,
    kept for readers of the old name. When testable_entity_count is zero,
    it MUST be reported as 0%, not as a division error. When the project
    has recorded test results (specforge-report.json), stats also reports
    the proof percentage: the share of testable entities the coverage
    rule proves, the analyze coverage --min gate's figure. Without
    results it is absent; a report that exists but cannot be read is an
    error.
  """
  verify unit "stats reports correct entity counts"
  verify unit "stats reports coverage percentage"
  verify unit "stats reports orphan count"
  verify unit "stats reports diagnostic summary"
  verify unit "coverage is 0% when testable_entity_count is zero"
  verify unit "stats leaves the entities W004 exempts out of the testable count"
  verify integration "stats reports the declared and proof percentages"
  verify contract "Compute Project Statistics: project statistics computation holds — validation_complete_fired, entity_counts_produced, coverage_computed, zero_testable_safe"
}

// The read views are operations over the project view: one per view,
// shared by the CLI and MCP.
behavior read_views_over_the_project_view "Read Views over the Project View" {
  features   [mcp_core_tools, extension_driven_coverage, traceability_serialization]
  invariants [diagnostic_determinism, testable_entity_classification, zero_domain_knowledge_core]
  category   query
  types      [Graph, KindRegistryEntry, GraphProtocolSchema, TraceChain]
  ports      [CompilerApi, McpProtocol]
  requires {
    project_compiled "A compiled project or a project session supplies the project view"
  }
  ensures {
    one_report_rule        "The recorded test report and the schema cache are the view root's, never an ancestor's"
    one_coverage_per_state "Coverage is computed once per compile and per recorded report content"
    surfaces_agree         "The CLI and MCP report the same numbers, chains, schema version and diagrams for one project"
  }
  contract   """
    Stats, trace (one entity or every entity), the coverage view, the
    model and outline diagrams and the versioned Graph Protocol schema
    MUST each be one operation over the project view, shared by the CLI
    and MCP; a surface maps its arguments and renders the outcome. The
    view's root is the root the project was compiled from: its recorded
    test report is <root>/specforge-report.json and its schema cache
    <root>/.specforge/schema-cache.json, and no view looks in an ancestor
    directory. A report that exists but cannot be read is an error on
    every view (E045). Coverage is computed once per compiled project or
    session state and per content of the recorded report; a rewritten
    report is read again. An entity is unverified when it counts toward
    coverage and is not proven.
  """
  verify unit "the recorded test report is read at the view's root, never an ancestor's"
  verify unit "coverage is computed once per compile and report content, and again after the report changes"
  verify unit "an entity is unverified when it counts toward coverage and is not proven"
  verify integration "specforge stats and specforge.stats report the same numbers"
  verify integration "specforge trace and specforge.trace return the same chain for an entity"
}

behavior print_diagnostics_structured "Print Diagnostics Structured" {
  features   [diagnostic_reporting]
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
    color_coding_applied       "Errors are red, warnings yellow, info blue"
  }
  contract   """
    All diagnostics MUST be formatted in a structured style: file path, line
    number, column, source context snippet, suggestion, and color-coded
    severity. Errors MUST be red, warnings yellow, info blue. The format
    MUST be consistent across all output modes.
  """
  verify unit "error diagnostic is formatted with file:line:col"
  verify unit "diagnostic includes context snippet"
  verify unit "suggestion is displayed when available"
  verify contract "Print Diagnostics Structured: structured diagnostic printing holds — validation_complete_fired, structured_format_enforced, color_coding_applied"
  verify unit "spanless diagnostic uses code as fallback location"
  verify unit "spanless error diagnostic also uses code"
  verify unit "diagnostics are plain when output is not a terminal"
  verify unit "NO_COLOR disables diagnostic colour"
}

behavior exit_code_reflects_diagnostic_severity "Exit Code Reflects Diagnostic Severity" {
  features   [ci_integration]
  invariants [multi_error_collection, diagnostic_determinism, zero_domain_knowledge_core]
  category   command
  types      [DiagnosticBag]
  ports      [CompilerApi]
  consumes   [validation_complete]
  requires {
    validation_complete_fired "validation_complete event has fired, confirming all diagnostics are finalized"
  }
  ensures {
    exit_zero_on_clean   "Exit code is 0 when no error-level diagnostics exist"
    exit_one_on_errors   "Exit code is 1 when any error-level diagnostic exists"
    strict_mode_enforced "In --strict mode, warnings also cause exit code 1"
  }
  contract   """
    specforge check MUST exit with code 0 if no errors exist. It MUST
    exit with code 1 if any error-level diagnostic exists. With --strict,
    warnings MUST also cause exit code 1.
    A command-line value outside a flag's allowed set, such as an
    unknown --format, MUST be rejected while arguments are parsed, with
    exit code 2, before anything is compiled.
  """
  verify unit "exit 0 with no errors"
  verify unit "exit 1 with errors"
  verify unit "exit 1 with warnings in strict mode"
  verify unit "a typo'd --format fails with a clap error (exit 2), not a bespoke runtime error"
  verify contract "Exit Code Reflects Diagnostic Severity: exit code severity mapping holds — validation_complete_fired, exit_zero_on_clean, exit_one_on_errors, strict_mode_enforced"
}

behavior serialize_traceability_data "Serialize Traceability Data" {
  features   [traceability_serialization]
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
    full_trace_serialized      "Full traceability chain from root to leaf entities is serialized as structured JSON"
    gaps_included              "Missing links in the chain are included with a missing status"
    graph_protocol_conformance "Output conforms to the Graph Protocol schema"
    render_complete_emitted    "render_complete event is emitted after successful serialization"
  }
  contract   """
    When specforge trace is invoked without an entity ID, the system MUST
    compute and serialize the full traceability chain across all registered
    edge types from root entities to leaf entities as structured JSON graph
    traversal data. Gaps in the chain MUST be included in the output with
    a "missing" status. The output MUST conform to the Graph Protocol schema.
  """
  verify unit "full trace covers all root entities across registered edge types"
  verify unit "gaps in chain are highlighted"
  verify unit "output conforms to Graph Protocol schema"
  verify contract "Serialize Traceability Data: traceability data serialization holds — validation_complete_fired, full_trace_serialized, gaps_included, graph_protocol_conformance, render_complete_emitted"
}

behavior validate_agent_plan "Validate Agent Implementation Plan" {
  features   [traceability_serialization]
  invariants [graph_traversal_integrity, diagnostic_determinism, zero_domain_knowledge_core]
  category   validation
  types      [
    Graph,
    TraceChain,
    TraceLink,
    AgentPlan,
    AgentPlanEntry,
    PlanValidationResult,
    PlanValidationEntry,
  ]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [plan_validated]
  requires {
    validation_complete_fired "validation_complete event has fired, confirming the spec graph is finalized for plan comparison"
  }
  ensures {
    unresolvable_ids_diagnosed "Every plan entity ID that does not resolve to a declared entity produces an E003 diagnostic"
    missing_entries_warned     "Every testable entity missing from the plan produces a warning"
    ordering_validated         "Plan dependency order is validated against graph edge structure"
    structured_report_produced "Output is a structured JSON report listing validated entries, gaps, and ordering violations"
    plan_validated_emitted     "plan_validated event is emitted after validation completes"
  }
  contract   """
    When specforge trace --plan plan.json is invoked, the system MUST parse
    the plan file and validate it against the current spec graph. Every entity
    ID referenced in the plan MUST resolve to a declared entity in the graph;
    unresolvable IDs MUST produce an E003 diagnostic. Every testable entity
    in the graph MUST have a corresponding planned action in the plan; missing
    entries MUST be reported as warnings. Dependency order declared in the plan
    MUST be validated against the graph's edge structure; any ordering that
    contradicts the graph MUST produce a diagnostic. The output MUST be a
    structured JSON report listing validated entries, gaps, and ordering
    violations.
  """
  verify unit "plan with all valid entity IDs passes validation"
  verify unit "plan referencing nonexistent entity ID produces E003"
  verify unit "testable entity missing from plan produces warning"
  verify unit "plan dependency order contradicting graph produces diagnostic"
  verify unit "output is structured JSON"
  verify contract "Validate Agent Implementation Plan: agent plan validation holds — validation_complete_fired, unresolvable_ids_diagnosed, missing_entries_warned, ordering_validated, structured_report_produced, plan_validated_emitted"
}

// render_index_files and selective_render_by_entity_type moved to
// spec/extensions/markdown-renderer/behaviors.spec

behavior deterministic_output "Deterministic Output" {
  features   [ci_integration]
  invariants [diagnostic_determinism, zero_domain_knowledge_core, graph_traversal_integrity]
  category   command
  types      [OutputFile]
  ports      [CompilerApi]
  consumes   [validation_complete]
  requires {
    validation_complete_fired "validation_complete event has fired, confirming input graph state is finalized"
  }
  ensures {
    byte_identical_output      "Given identical input spec files, all output paths produce byte-for-byte identical output"
    no_nondeterministic_values "Output contains no timestamps, random values, or iteration-order-dependent content"
  }
  contract   """
    Given identical input .spec files, all output paths — file emitters
    (specforge render) and stdout emitters (specforge export, specforge query)
    — MUST produce byte-for-byte identical output. Output MUST NOT depend on
    filesystem iteration order, hashmap ordering, or timestamps.
  """
  verify property "same input produces identical output across runs"
  verify unit "entity ordering is independent of hashmap iteration"
  verify unit "file emission order is independent of filesystem readdir order"
  verify unit "output contains no timestamps or non-deterministic values"
  verify unit "edge ordering is independent of hashmap iteration"
  verify contract "Deterministic Output: deterministic output holds — validation_complete_fired, byte_identical_output, no_nondeterministic_values"
}

behavior check_mode_for_ci "Check Mode for CI" {
  features   [ci_integration]
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
    diagnostics_to_stderr    "All diagnostics are printed to stderr"
    appropriate_exit_code    "Exit code reflects diagnostic severity per exit_code_reflects_diagnostic_severity"
  }
  contract   """
    specforge check MUST parse, resolve, and validate all .spec files
    without producing any output files. It MUST print diagnostics to
    stderr and exit with an appropriate exit code. This is the primary
    CI integration point.
  """
  verify unit "check mode produces no output files"
  verify unit "check mode prints diagnostics to stderr"
  verify integration "check mode works in CI environment"
  verify contract "Check Mode for CI: CI check mode holds — validation_complete_fired, no_output_files_produced, diagnostics_to_stderr, appropriate_exit_code"
}

behavior export_diagnostics_as_json "Export Diagnostics as JSON" {
  features   [ci_integration, diagnostic_reporting]
  invariants [multi_error_collection, diagnostic_determinism, zero_domain_knowledge_core]
  category   query
  types      [DiagnosticBag, Diagnostic, DiagnosticFormat]
  ports      [CompilerApi]
  consumes   [validation_complete]
  requires {
    validation_complete_fired "validation_complete event has fired, confirming all diagnostics are collected"
  }
  ensures {
    json_array_produced        "All diagnostics are serialized as a JSON array to stdout"
    diagnostic_fields_complete "Each diagnostic includes code, severity, message, file path, line, column"
    exit_code_unaffected       "The --format=json flag does not alter exit code behavior"
  }
  contract   """
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
  verify contract "Export Diagnostics as JSON: JSON diagnostic export holds — validation_complete_fired, json_array_produced, diagnostic_fields_complete, exit_code_unaffected"
  verify unit "max diagnostics limit truncates output"
  verify unit "no truncation under limit"
}

// One JSON shape for diagnostics on every surface (ADR 0004 D1-c): a
// superset of the CLI's nested span and MCP's flat location, so no reader
// of either breaks.
behavior present_diagnostics_as_json "Present Diagnostics as JSON" {
  features   [ci_integration, diagnostic_reporting]
  invariants [diagnostic_determinism, zero_domain_knowledge_core]
  category   query
  types      [Diagnostic]
  ports      [CompilerApi]
  requires {
    diagnostics_collected "the diagnostics to present have been collected"
  }
  ensures {
    one_shape_everywhere "specforge check --format json, MCP validate, the specforge://diagnostics resource and tool _meta present diagnostics in one shape"
    location_both_ways   "each entry carries its span nested under span and the span's start flat as file, line and column"
    data_only_when_typed "an entry carries data, the diagnostic's typed payload, only when the diagnostic has one"
  }
  contract   """
    Wherever SpecForge prints diagnostics as JSON (specforge check
    --format json, specforge collect and specforge watch --json, the MCP
    specforge.validate tool, the specforge://diagnostics resource and the
    diagnostics in a tool result's _meta), it MUST present them in one
    shape: an array of
    entries with code, severity, message and suggestion (null when there
    is none), the span nested under span (file, start_line, start_col,
    end_line, end_col; null without a location), and the span's start
    flat as file, line and column (null without a location). The shape
    is a superset of the nested and the flat shapes these surfaces used
    before, so no reader of either breaks. A diagnostic that carries a
    typed payload (DiagnosticData, e.g. the target of an unresolved
    reference) MUST present it under data, tagged by kind; one without
    MUST NOT gain the key, so its entry is unchanged.
  """
  verify unit "diagnostics are presented as one JSON array"
  verify unit "each diagnostic carries code, severity, message, file, line and column"
  verify unit "suggestion is included when available"
  verify unit "the presented JSON is valid and parseable"
  verify unit "the span is nested beside the flat location, with its end positions"
  verify unit "a typed payload is presented under data, and its absence adds no key"
  verify integration "check and MCP validate present the same diagnostics identically"
  verify contract "Present Diagnostics as JSON: JSON diagnostic presentation holds — diagnostics_collected, one_shape_everywhere, location_both_ways, data_only_when_typed"
}

// ── Agent-Optimized Export (Principle 3: agents are first-class consumers) ──
// `specforge export` writes to stdout for agent/interactive consumption (context, brief, graph formats).
// `specforge render` writes files to disk for batch/CI output (json, dot, markdown via extensions).

behavior export_agent_context_format "Export Agent Context Format" {
  features   [agent_export]
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    graph_schema_completeness,
    zero_domain_knowledge_core,
  ]
  category   query
  types      [
    Graph,
    GraphProtocolSchema,
    OutputFile,
    AgentExportConfig,
    ProjectStatistics,
    GraphAnnotation,
  ]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [export_complete]
  requires {
    validation_complete_fired "validation_complete event has fired, confirming the graph is ready for agent export"
  }
  ensures {
    token_optimized_output  "Output omits verbose prose fields to minimize token consumption"
    schema_version_present  "Output includes a schema_version field identifying the Graph Protocol version"
    scope_enforced          "When --scope is specified, only the reachable subgraph is returned"
    invalid_scope_diagnosed "Non-existent scope entity produces E003 and exit code 1"
    export_complete_emitted "export_complete event is emitted after successful export"
  }
  contract   """
    When specforge export --format=context is invoked, the system MUST produce
    a token-optimized representation of the graph containing entity IDs,
    contracts, relationships, and coverage status. Each entity also carries
    the fields its extension declares normative (an invariant's guarantee,
    a decision's decision text), so every kind keeps the text that states
    what it promises; core reads the flag and knows no field by name. The
    fields an extension declares headline (a behavior's contract, a status)
    sit at the entity's top level instead; a field no extension declares
    headline is never lifted, whatever its name. The output MUST omit
    verbose fields (full descriptions, prose) to minimize token consumption.
    The format MUST be valid JSON conforming to the Graph Protocol schema.
    The output MUST include a schema_version field identifying the Graph
    Protocol version. An optional --scope parameter MUST allow scoping to
    a subgraph rooted at a specific entity. If --scope references a
    non-existent entity ID, the system MUST emit an E003 diagnostic
    and exit with code 1. When coverage metadata is available from
    compute_project_statistics, the context export MUST include
    coverage_pct and testable_entity_count in the graph metadata.
  """
  verify unit "context format includes entity IDs and contracts"
  verify unit "context format omits verbose prose fields"
  verify unit "context format keeps each entity's normative fields"
  verify integration "export --format context keeps an invariant's guarantee"
  verify unit "scoped export returns only reachable subgraph"
  verify unit "non-existent scope entity produces E003 and exit code 1"
  verify unit "output conforms to Graph Protocol schema"
  verify unit "output includes schema_version field"
  verify contract "Export Agent Context Format: agent context export holds — validation_complete_fired, token_optimized_output, schema_version_present, scope_enforced, invalid_scope_diagnosed, export_complete_emitted"
  verify unit "the context export leaves the schema out unless --with-schema is given"
}

behavior export_agent_brief_format "Export Agent Brief Format" {
  features   [agent_export]
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    graph_schema_completeness,
    zero_domain_knowledge_core,
  ]
  category   query
  types      [Graph, GraphProtocolSchema, OutputFile, AgentExportConfig]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [export_complete]
  requires {
    validation_complete_fired "validation_complete event has fired, confirming the graph is ready for agent export"
  }
  ensures {
    minimal_representation  "Output contains only entity IDs, kinds, titles, and direct relationships"
    schema_version_present  "Output includes a schema_version field identifying the Graph Protocol version"
    export_complete_emitted "export_complete event is emitted after successful export"
  }
  contract   """
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
  verify contract "Export Agent Brief Format: agent brief export holds — validation_complete_fired, minimal_representation, schema_version_present, export_complete_emitted"
  verify unit "the brief export leaves the schema out unless --with-schema is given"
}

behavior export_agent_graph_format "Export Agent Graph Format" {
  features   [agent_export]
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    graph_schema_completeness,
    zero_domain_knowledge_core,
  ]
  category   query
  types      [Graph, GraphProtocolSchema, OutputFile, AgentExportConfig]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [export_complete]
  requires {
    validation_complete_fired "validation_complete event has fired, confirming the graph is ready for full-fidelity export"
  }
  ensures {
    full_fidelity_output    "Output contains all nodes, edges, fields, and metadata"
    schema_version_present  "Output includes a schema_version field identifying the Graph Protocol version"
    scope_enforced          "When --scope is specified, only the reachable subgraph is returned"
    invalid_scope_diagnosed "Non-existent scope entity produces E003 and exit code 1"
    export_complete_emitted "export_complete event is emitted after successful export"
  }
  contract   """
    When specforge export --format=graph is invoked, the system MUST produce
    the complete entity graph as JSON conforming to the Graph Protocol schema.
    This is the full-fidelity format containing all nodes, edges, fields, and
    metadata. Unlike specforge render json (which writes files to disk), this
    command writes to stdout for agent consumption. An optional --scope parameter
    MUST allow scoping to a subgraph rooted at a specific entity. If --scope
    references a non-existent entity ID, the system MUST emit an E003
    diagnostic and exit with code 1. The output MUST include a schema_version
    field identifying the Graph Protocol version.
  """
  verify unit "graph format includes all nodes and edges"
  verify unit "graph format includes all fields and metadata"
  verify unit "scoped export returns only reachable subgraph"
  verify unit "non-existent scope entity produces E003 and exit code 1"
  verify unit "output conforms to Graph Protocol schema"
  verify unit "output includes schema_version field"
  verify integration "structural-only graph exports valid JSON with raw keyword strings as entity kinds"
  verify contract "Export Agent Graph Format: agent graph export holds — validation_complete_fired, full_fidelity_output, schema_version_present, scope_enforced, invalid_scope_diagnosed, export_complete_emitted"
}

behavior query_graph_multi_resolution "Query Graph at Multiple Resolutions" {
  features   [agent_export]
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    graph_schema_completeness,
    zero_domain_knowledge_core,
  ]
  category   query
  types      [Graph, GraphProtocolSchema, AgentExportConfig]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [graph_queried]
  requires {
    validation_complete_fired "validation_complete event has fired, confirming the graph is finalized and traversable"
  }
  ensures {
    depth_respected            "Subgraph returned contains only entities within the requested hop distance"
    kind_filter_applied        "When --kind is specified, results are restricted to the specified entity kinds"
    graph_protocol_conformance "Output is valid JSON conforming to the Graph Protocol schema with schema_version"
    graph_queried_emitted      "graph_queried event is emitted after query completes"
  }
  contract   """
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
    Filtering builds on a graph-level node filter that accepts any
    predicate over an entity's kind and fields.
  """
  verify unit "depth 0 returns only the target entity"
  verify unit "depth 1 returns direct neighbors"
  verify unit "depth N returns all entities within N hops"
  verify unit "kind filter restricts results to specified entity kinds"
  verify unit "multiple kind filters combine as union"
  verify unit "output conforms to Graph Protocol schema"
  verify unit "output includes schema_version field"
  verify property "querying same entity at same depth produces identical subgraph"
  verify unit "filter_nodes with field predicate"
  verify contract "Query Graph at Multiple Resolutions: multi-resolution graph query holds — validation_complete_fired, depth_respected, kind_filter_applied, graph_protocol_conformance, graph_queried_emitted"
}

// ── Token Economics (Principle 3: agents are first-class consumers) ────

behavior enforce_token_budget "Enforce Token Budget" {
  features   [agent_export]
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    token_budget_subgraph_consistency,
    zero_domain_knowledge_core,
  ]
  category   query
  types      [AgentExportConfig, TokenBudgetResult, Graph, OutputFile, ExportResult, TokenBudgetStrategy]
  ports      [CompilerApi]
  consumes   [validation_complete]
  produces   [token_budget_applied]
  requires {
    validation_complete_fired "validation_complete event has fired, confirming the graph is ready for token estimation"
  }
  ensures {
    budget_respected                "Output token count does not exceed the specified --max-tokens budget"
    truncation_metadata_produced    "TokenBudgetResult is included in output metadata when budget is applied"
    valid_subgraph_after_truncation "Remaining subgraph after truncation has no dangling edge references"
    token_budget_applied_emitted    "token_budget_applied event is emitted after budget enforcement completes"
  }
  contract   """
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
  verify integration "the graph export honours --max-tokens with the schema left out unless --with-schema is given"
  verify integration "an embedded schema counts toward the token budget"
  verify integration "a budget smaller than the embedded schema fails with E062 instead of truncating the schema"
  verify integration "a budget below one entity yields the envelope with no entities and the truncation marker"
  verify integration "a budget below the empty envelope fails with E062"
  verify contract "Enforce Token Budget: token budget enforcement holds — validation_complete_fired, budget_respected, truncation_metadata_produced, valid_subgraph_after_truncation, token_budget_applied_emitted"
}
