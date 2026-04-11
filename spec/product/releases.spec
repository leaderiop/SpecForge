// SpecForge product releases — coordinated multi-deliverable shipping

use "product/deliverables"
use "product/milestones"

release alpha "SpecForge Alpha" {
  description """
    First public release of the SpecForge compiler. Core compilation
    pipeline (parse, resolve, graph, validate, emit) with CLI binary
    and tree-sitter grammar.
  """
  version      "0.1.0"
  status       in_progress
  deliverables [tree_sitter_specforge_deliverable, specforge_core, specforge_cli_deliverable, specforge_rust_traceability_deliverable]
  milestones   [structural_parsing, resolution_and_graph, validation_and_errors, output_and_export, project_init]
  target_date  "2026-06-01"
  changelog    """
    Initial alpha with core compiler pipeline, CLI binary, and basic
    validation. Includes tree-sitter grammar for .spec files, import
    resolution, mutable entity graph, ariadne-powered diagnostics,
    multi-format export (JSON, DOT, context, brief), project
    initialization via specforge init, and Rust test traceability
    toolkit with proc macro and JUnit XML collector.
  """
  reason       "Establish the core compilation pipeline and validate the graph-first approach with early adopters."
  owner        "specforge-team"
  contributors ["specforge-team"]
  tags         ["core", "mvp"]
}

release beta "SpecForge Beta" {
  description """
    Feature-complete release adding LSP, MCP, incremental compilation,
    formatting, and the zero-entity core architecture. Enables full
    IDE integration and agent consumption.
  """
  version      "0.5.0"
  status       planned
  deliverables [specforge_cli_deliverable, specforge_lsp_deliverable, specforge_mcp_deliverable]
  milestones   [lsp_server, ms_code_formatting, zero_entity_core, ms_incremental_compilation, mcp_server]
  depends_on   [alpha]
  reason       "Deliver IDE-grade developer experience and agent-first MCP integration before stabilizing the extension runtime."
  owner        "specforge-team"
  contributors ["specforge-team"]
  tags         ["platform", "ide", "agent"]
}

release one_zero "SpecForge 1.0" {
  description """
    Production release with Wasm extension runtime, first-party
    extensions, and schema versioning. The graph protocol schema
    is stabilized.
  """
  version      "1.0.0"
  status       planned
  deliverables [
    specforge_cli_deliverable,
    specforge_lsp_deliverable,
    specforge_mcp_deliverable,
    specforge_wasm_runtime_deliverable,
    specforge_software,
    specforge_formal,
    specforge_product,
    specforge_governance,
    specforge_gh,
  ]
  milestones   [wasm_runtime, extension_ecosystem, schema_versioning]
  depends_on   [beta]
  reason       "Stabilize the graph protocol schema and extension runtime for production adoption. The standard is the moat."
  owner        "specforge-team"
  contributors ["specforge-team"]
  tags         ["stable", "extensions", "production"]
}
