decision D-016 "Federation as Extension" {
  settled true
  origin  project
  text    """
    Federation functionality extracted from core into @specforge/federation
    extension. Federation behaviors reference federation-local invariants
    (federated_graph_traversal_integrity, federated_schema_completeness,
    cross_project_reference_safety) instead of core invariants. Generic
    qualified-reference hooks remain in core for any extension to use.
    Core compiler has zero federation knowledge.
  """
}
