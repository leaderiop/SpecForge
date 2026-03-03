// Output and rendering features

use behaviors/output

feature markdown_documentation_generation "Markdown Documentation Generation" {
  behaviors [render_markdown_documentation, render_index_files, selective_render_by_entity_type]

  problem """
    Stakeholders who don't work with .spec files need readable
    documentation. Generated docs must stay in sync with the spec
    automatically without manual maintenance.
  """

  solution """
    specforge render markdown traverses the graph and produces .md files
    grouped by entity type, with cross-reference links, index files,
    and support for entity type filtering.
  """
}

feature json_and_dot_export "JSON and DOT Export" {
  behaviors [render_json_graph, render_dot_visualization]

  problem """
    External tools (dashboards, analyzers, visualizers) need machine-readable
    access to the spec graph. The graph must be exportable in standard formats.
  """

  solution """
    specforge render json exports the full graph as JSON. specforge graph
    exports a DOT format graph compatible with Graphviz for visualization.
  """
}

feature traceability_reports "Traceability Reports" {
  behaviors [compute_traceability_chain, render_traceability_report]

  problem """
    Architects and auditors need to trace requirements from high-level
    deliverables down to individual invariants and back. Manual traceability
    matrices are error-prone and always stale.
  """

  solution """
    specforge trace auto-generates traceability reports by traversing the
    graph. Single-entity trace shows upstream and downstream links.
    Full trace shows every chain from deliverable to invariant with gap
    detection.
  """
}

feature ci_integration "CI Integration" {
  behaviors [compute_project_statistics, print_diagnostics_in_rustc_style, exit_code_reflects_diagnostic_severity, deterministic_output, check_mode_for_ci]

  problem """
    CI pipelines need a single command that validates all .spec files,
    exits with an appropriate code, and produces deterministic output
    suitable for automated checks.
  """

  solution """
    specforge check parses, resolves, and validates without writing files.
    Exit code 0 for clean, 1 for errors. --strict treats warnings as errors.
    Output is deterministic for reproducible CI runs.
  """
}
