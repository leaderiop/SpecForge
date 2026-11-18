milestone v0.1.0 "SpecForge Alpha" {
  name      "SpecForge Alpha"
  delivered """
    Initial alpha with core compiler pipeline, CLI binary, and basic
    validation. Includes tree-sitter grammar for .spec files, import
    resolution, mutable entity graph, ariadne-powered diagnostics,
    multi-format export (JSON, DOT, context, brief), project
    initialization via specforge init, and Rust test traceability
    toolkit with proc macro and test result collection.
  """
  goal      """
    First public release of the SpecForge compiler. Core compilation
    pipeline (parse, resolve, graph, validate, emit) with CLI binary
    and tree-sitter grammar.
  """
}

milestone v0.5.0 "SpecForge Beta" {
  name "SpecForge Beta"
  goal """
    Feature-complete release adding LSP, MCP, incremental compilation,
    formatting, and the zero-entity core architecture. Enables full
    IDE integration and agent consumption.
  """
}

milestone v1.0.0 "SpecForge 1.0" {
  name "SpecForge 1.0"
  goal """
    Production release with Wasm extension runtime, first-party
    extensions, and schema versioning. The graph protocol schema
    is stabilized.
  """
}
