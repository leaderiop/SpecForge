// @specforge/product extension invariants
//
// Runtime guarantees specific to the product entity model:
// capability, deliverable, roadmap, library, glossary.

invariant library_dag "Library DAG" {
  guarantee """
    The depends_on edges between library nodes MUST form a directed acyclic
    graph. The compiler MUST detect and reject circular library dependencies
    with an E007 diagnostic.
  """
  enforced_by [detect_library_cycles]
  risk medium

  verify property "an acyclic library dependency graph is accepted"
  verify unit "a circular library dependency produces E007"

}
