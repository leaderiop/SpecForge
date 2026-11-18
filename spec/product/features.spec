requirement FEATURE-098 "Product Entity Registration" {
  category "Feature"
  tier     v1
  text     """
    The compiler has no knowledge of product planning concepts until
    @specforge/product registers its entity kinds, edge types, field
    definitions, and validation rules (counts live in the manifest —
    see extensions/product — not in prose).

    A comprehensive ExtensionDeclaration declaration provides all entity kinds with
    testability flags, LSP metadata, DOT shapes, typed field definitions
    with edge mappings, and declarative validation rules. Registration
    follows the zero-entity core protocol.
  """
}

requirement FEATURE-100 "Product Graph Queries" {
  category "Feature"
  tier     v1
  text     """
    Product entities form a rich graph but there is no way to query it
    for planning insights: milestone completion, deliverable traceability,
    journey coverage, feature ordering, or reverse traversals.

    The product query port (every method declared in the query surface
    manifest) traverses the product graph to compute planning metrics. Results are cached per build and atomically invalidated
    on graph rebuild. Entity-not-found errors include fuzzy-match
    suggestions.
  """
}

requirement FEATURE-104 "Product Surface Access" {
  category "Feature"
  tier     v1
  text     """
    Product queries and validation have no CLI commands or MCP tools
    to expose them to users and agents.

    CLI commands declared in the manifest surfaces field are
    auto-promoted to MCP tools, which always answer in JSON; one-hop
    navigation is core's query. All surfaces have typed input/output
    schemas (ADR 0011).
  """
}

requirement FEATURE-105 "Product Validation Rules" {
  category "Feature"
  tier     v1
  text     """
    Without domain-specific validation, the compiler cannot detect
    product-level quality issues like unreferenced entities, dependency
    cycles, lifecycle inconsistencies, or invalid field values.

    Declarative validation rules across the severity ladder detect
    structural and semantic quality issues (the rule inventory is the
    manifest's, not this prose's).
    Each rule uses the declarative pattern engine with full diagnostic
    code traceability.
  """
}

requirement FEATURE-102 "Product Health Metric" {
  category "Feature"
  tier     v1
  text     """
    Individual queries return focused metrics but there is no single
    composite metric for overall product health.

    A composite health score (0.0-1.0) from weighted sub-scores:
    completion (30%), coverage (25%), unreferenced ratio (20%), cycle count
    (15%), error ratio (10%). Maps to a grade for quick assessment.
  """
}

requirement FEATURE-103 "Impact Analysis and What-If" {
  category "Feature"
  tier     v1
  text     """
    Product managers cannot assess transitive impact of deferring or
    completing a feature without mentally tracing dependency chains.

    Feature impact analysis (transitive traversal) and what-if simulation
    (hypothetical action on a read-only graph clone). Both available as
    MCP resources.
  """
}

requirement FEATURE-099 "Graph Diff and Comparison" {
  category "Feature"
  tier     v2
  text     """
    No way to compare the product graph over time between builds or
    sprints.

    Graph diff query compares two compiler snapshots and returns
    structural changes plus status field changes. Snapshots stored
    with configurable retention (default 50).
  """
}

requirement FEATURE-101 "Product Graph Rendering" {
  category "Feature"
  tier     v1
  text     """
    Product entities have no specification for how they appear in the
    Graph Protocol JSON output produced by specforge export.

    Product entities render as standard graph nodes with kind, id, fields,
    and edges in all export formats (context, graph, brief). Rendering
    follows the core graph protocol schema with no product-specific
    renderer — the core emitter handles all entity kinds uniformly.
  """
}
