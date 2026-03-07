// Federation Architecture Decision Records

use extensions/federation/invariants

decision federation_in_core "Federation in Core" {
  status   superseded
  date     2026-03-04

  context """
    Federation operates on structural references (project::entity_id syntax)
    and graph merging — both are structural operations, not domain-specific.
    The cross-project reference syntax is analogous to use imports: it
    describes how graphs connect, not what the entities mean semantically.
    The question is whether federation belongs in core or in an extension.
  """

  decision """
    Federation stays in the core compiler because cross-project reference
    syntax (project::entity_id) is structural, like use imports. The core
    already owns reference resolution and graph construction — federation
    extends these with project-qualified references and graph merging.
    Remote graph loading is constrained to local filesystem paths only
    (published Graph Protocol JSON files). Network-based federation
    (registry lookups, remote fetching) is deferred to the CLI wrapper
    or a dedicated extension.
  """

  consequences [
    "Cross-project references use the same resolution pipeline as local references",
    "No network access in the compiler core — registry/network is a CLI-level concern",
    "Remote graph loading limited to local/published Graph Protocol JSON files",
    "Federation metadata is part of the Graph Protocol standard, not a compiler extension",
    "Extensions can build on federation (e.g., registry-based remote loading) without core changes",
  ]

  invariants [cross_project_reference_safety, graph_traversal_integrity]
}

decision federation_as_extension "Federation as Extension" {
  status   accepted
  date     2026-03-06

  context """
    Vision Principle 7 mandates "Extensions over built-ins, always." Federation
    functionality — cross-project references, graph merging, federated export —
    was originally placed in core (see superseded federation_in_core ADR).
    However, federation is domain-aware behavior that not all SpecForge users
    need. Keeping it in core contradicts the zero-entity-core architecture and
    adds complexity to the compiler that only multi-project setups require.
  """

  decision """
    Federation functionality extracted from core into @specforge/federation
    extension. Federation behaviors reference federation-local invariants
    (federated_graph_traversal_integrity, federated_schema_completeness,
    cross_project_reference_safety) instead of core invariants. Generic
    qualified-reference hooks remain in core for any extension to use.
    Core compiler has zero federation knowledge.
  """

  consequences [
    "Core compiler has zero federation knowledge — simpler and faster for single-project use",
    "Federation behaviors reference federation-local invariants instead of core invariants",
    "Generic qualified-reference hooks remain in core for any extension to use",
    "Federation config is extension-provided, not part of core CompilerConfig",
    "Supersedes federation_in_core ADR",
  ]

  invariants [federated_graph_traversal_integrity, federated_schema_completeness, cross_project_reference_safety]
}
