// Product features — self-hosting @specforge/product's own feature entities
//
// Closes the traceability chain: deliverable -> journey -> feature -> module.
// These are the product-level feature instances for the pe_* capabilities
// declared in spec/extensions/product/features.spec.

use "extensions/product/features"

feature product_entity_registration "Product Entity Registration" {
  problem   """
    The compiler has no knowledge of product planning concepts until
    @specforge/product registers its 9 entity kinds, 16 edge types,
    field definitions, and validation rules.
  """
  solution  """
    A comprehensive ManifestV2 declaration provides all entity kinds with
    testability flags, LSP metadata, DOT shapes, typed field definitions
    with edge mappings, and declarative validation rules. Registration
    follows the zero-entity core protocol.
  """
  acceptance [
    "All 9 entity kinds registered in KindRegistry after manifest load",
    "Each kind has correct testable, singleton, and supports_verify flags",
    "All 16 edge types registered with correct source/target kind constraints",
    "Field definitions include type, optionality, and edge mappings",
  ]
  effort       s
  owner        "specforge-team"
  contributors ["specforge-team"]
}

feature product_graph_queries "Product Graph Queries" {
  problem   """
    Product entities form a rich graph but there is no way to query it
    for planning insights: milestone completion, deliverable traceability,
    journey coverage, feature ordering, or reverse traversals.
  """
  solution  """
    26 query methods traverse the product graph to compute planning
    metrics. Results are cached per build and atomically invalidated
    on graph rebuild. Entity-not-found errors include fuzzy-match
    suggestions.
  """
  effort       l
  owner        "specforge-team"
  contributors ["specforge-team"]
}

feature product_surface_access "Product Surface Access" {
  problem   """
    Product queries and validation have no CLI commands or MCP tools
    to expose them to users and agents.
  """
  solution  """
    16 CLI commands (8 list + 6 query + bulk-status + health) are
    auto-promoted to MCP tools. 23 MCP resources expose remaining
    query-port methods. All surfaces have typed input/output schemas.
  """
  acceptance [
    "All 16 CLI commands respond to --help with usage and typed arguments",
    "All MCP resources return valid ProductSurfaceResponse JSON",
    "CLI commands auto-promoted to MCP tools with matching input schemas",
    "Surface contributions registered from manifest surfaces field without manual wiring",
  ]
  effort       l
  owner        "specforge-team"
  contributors ["specforge-team"]
}

feature product_validation "Product Validation Rules" {
  problem   """
    Without domain-specific validation, the compiler cannot detect
    product-level quality issues like orphan entities, dependency
    cycles, lifecycle inconsistencies, or invalid field values.
  """
  solution  """
    47 declarative validation rules across 3 severity levels (5 errors,
    22 warnings, 28 info) detect structural and semantic quality issues.
    Each rule uses the declarative pattern engine with full diagnostic
    code traceability.
  """
  acceptance [
    "All 55 diagnostic codes fire on their respective invalid inputs",
    "No false positives on valid specs with complete product graphs",
    "Status transition violations (W087-W091, W094) caught for all entity lifecycles",
    "Cycle detection (W092) reports all participants in the cycle",
  ]
  effort       xl
  owner        "specforge-team"
  contributors ["specforge-team"]
}

feature product_health_metric "Product Health Metric" {
  problem   """
    Individual queries return focused metrics but there is no single
    composite metric for overall product health.
  """
  solution  """
    A composite health score (0.0-1.0) from weighted sub-scores:
    completion (30%), coverage (25%), orphan ratio (20%), cycle count
    (15%), error ratio (10%). Maps to a grade for quick assessment.
  """
  acceptance [
    "Health score returns 0.0-1.0 float with letter grade mapping",
    "Sub-scores weighted correctly: completion 30%, coverage 25%, orphan 20%, cycle 15%, error 10%",
    "Perfect project scores 1.0; empty project scores 0.0",
  ]
  effort       m
  owner        "specforge-team"
  contributors ["specforge-team"]
}

feature product_impact_and_whatif "Impact Analysis and What-If" {
  problem   """
    Product managers cannot assess transitive impact of deferring or
    completing a feature without mentally tracing dependency chains.
  """
  solution  """
    Feature impact analysis (transitive traversal) and what-if simulation
    (hypothetical action on a read-only graph clone). Both available as
    MCP resources.
  """
  effort       l
  owner        "specforge-team"
  contributors ["specforge-team"]
}

feature product_graph_diff "Graph Diff and Comparison" {
  problem   """
    No way to compare the product graph over time between builds or
    sprints.
  """
  solution  """
    Graph diff query compares two compiler snapshots and returns
    structural changes plus status field changes. Snapshots stored
    with configurable retention (default 50).
  """
  effort       l
  owner        "specforge-team"
  contributors ["specforge-team"]
}

feature product_graph_rendering "Product Graph Rendering" {
  problem   """
    Product entities have no specification for how they appear in the
    Graph Protocol JSON output produced by specforge export.
  """
  solution  """
    Product entities render as standard graph nodes with kind, id, fields,
    and edges in all export formats (context, graph, brief). Rendering
    follows the core graph protocol schema with no product-specific
    renderer — the core emitter handles all entity kinds uniformly.
  """
  effort       s
  owner        "specforge-team"
  contributors ["specforge-team"]
}
