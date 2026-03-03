// Output behaviors — rendering and emitting artifacts

use invariants/core
use invariants/validation
use types/graph
use types/diagnostics
use types/codegen
use ports/outbound
use ports/inbound

behavior render_markdown_documentation "Render Markdown Documentation" {
  types      [Graph, GeneratedFile, EmitterError]
  ports      [OutputRenderer, FileSystem]

  contract """
    When specforge render markdown is invoked, the system MUST traverse
    the graph and produce one .md file per entity group. Generated
    markdown MUST include entity titles, descriptions, cross-reference
    links, and traceability chains. Output MUST be written to the
    specified output directory.
  """

  verify unit        "each entity group produces a markdown file"
  verify unit        "cross-references render as links"
  verify integration "output directory is created if missing"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior render_json_graph "Render JSON Graph" {
  types      [Graph, GeneratedFile]
  ports      [OutputRenderer, FileSystem]

  contract """
    When specforge render json is invoked, the system MUST serialize
    the entire in-memory graph to JSON. The JSON MUST contain all nodes,
    edges, and metadata. The output MUST be valid JSON parseable by
    standard tools.
  """

  verify unit "JSON output contains all nodes"
  verify unit "JSON output contains all edges"
  verify unit "output is valid JSON"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior render_dot_visualization "Render DOT Visualization" {
  types      [Graph, GeneratedFile]
  ports      [OutputRenderer, FileSystem]

  contract """
    When specforge graph is invoked, the system MUST emit a DOT format
    graph compatible with Graphviz. Nodes MUST be labeled with entity
    IDs and titles. Edges MUST be labeled with edge types.
  """

  verify unit "DOT output is valid Graphviz syntax"
  verify unit "nodes are labeled with IDs"
  verify unit "edges are labeled with types"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior compute_traceability_chain "Compute Traceability Chain" {
  types      [Graph, TraceChain, TraceLink]
  ports      [CompilerApi]

  contract """
    When specforge trace is invoked with an entity ID, the system MUST
    traverse the graph both upstream and downstream from that entity.
    The trace MUST show the full chain: deliverable -> capability ->
    feature -> behavior -> invariant. Missing links MUST be flagged.
  """

  verify unit "trace from behavior shows upstream features and downstream invariants"
  verify unit "trace shows full chain depth"
  verify unit "missing link in chain is flagged"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior compute_project_statistics "Compute Project Statistics" {
  types      [Graph]

  contract """
    When specforge stats is invoked, the system MUST compute and display:
    entity counts by type, coverage percentage, orphan count, and
    diagnostic summary. Statistics MUST be derived from the current
    graph state.
  """

  verify unit "stats reports correct entity counts"
  verify unit "stats reports coverage percentage"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior print_diagnostics_in_rustc_style "Print Diagnostics in Rustc Style" {
  invariants [multi_error_collection, diagnostic_determinism]
  types      [Diagnostic, DiagnosticBag]

  contract """
    All diagnostics MUST be formatted in the rustc style: file path, line
    number, column, context snippet, and suggestion. Errors MUST be red,
    warnings yellow, info blue. The format MUST be consistent across all
    output modes.
  """

  verify unit "error diagnostic is formatted with file:line:col"
  verify unit "diagnostic includes context snippet"
  verify unit "suggestion is displayed when available"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior exit_code_reflects_diagnostic_severity "Exit Code Reflects Diagnostic Severity" {
  types      [DiagnosticBag]

  contract """
    specforge check MUST exit with code 0 if no errors exist. It MUST
    exit with code 1 if any error-level diagnostic exists. With --strict,
    warnings MUST also cause exit code 1.
  """

  verify unit "exit 0 with no errors"
  verify unit "exit 1 with errors"
  verify unit "exit 1 with warnings in strict mode"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior render_traceability_report "Render Traceability Report" {
  types      [Graph, TraceChain, GeneratedFile]
  ports      [OutputRenderer, FileSystem]

  contract """
    When specforge trace is invoked without an entity ID, the system MUST
    produce a full traceability report showing every chain from deliverable
    down to invariant. Gaps in the chain MUST be highlighted.
  """

  verify unit "full trace covers all deliverables"
  verify unit "gaps in chain are highlighted"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior render_index_files "Render Index Files" {
  types      [Graph, GeneratedFile]
  ports      [OutputRenderer, FileSystem]

  contract """
    The compiler MUST auto-generate index files listing all entities
    grouped by type. Index files MUST NOT be hand-written. They MUST be
    regenerated on every compilation.
  """

  verify unit "index file lists all entities by type"
  verify unit "index is regenerated on recompilation"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior selective_render_by_entity_type "Selective Render by Entity Type" {
  types      [Graph, GeneratedFile]
  ports      [OutputRenderer]

  contract """
    The render command SHOULD support filtering by entity type
    (e.g., specforge render markdown --only behaviors). When a filter
    is specified, only matching entities MUST appear in the output.
  """

  verify unit "filter renders only matching entity types"
  verify unit "no filter renders all entity types"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior deterministic_output "Deterministic Output" {
  invariants [diagnostic_determinism]
  types      [GeneratedFile]

  contract """
    Given identical input .spec files, all emitters MUST produce byte-for-byte
    identical output. Output MUST NOT depend on filesystem iteration order,
    hashmap ordering, or timestamps.
  """

  verify property "same input produces identical output across runs"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior check_mode_for_ci "Check Mode for CI" {
  invariants [multi_error_collection]
  types      [DiagnosticBag]

  contract """
    specforge check MUST parse, resolve, and validate all .spec files
    without producing any output files. It MUST print diagnostics to
    stderr and exit with an appropriate exit code. This is the primary
    CI integration point.
  """

  verify unit        "check mode produces no output files"
  verify unit        "check mode prints diagnostics to stderr"
  verify integration "check mode works in CI environment"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}
