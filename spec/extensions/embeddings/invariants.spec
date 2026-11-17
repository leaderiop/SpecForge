requirement INVARIANT-022 "Embedding Cache Consistency" {
  category "Invariant"
  tier     v2
  text     """
    The embedding cache MUST be invalidated when any entity's content or
    relationships change, ensuring semantic search results always reflect
    current graph state.
  """
}

requirement INVARIANT-023 "Embedding Provider Determinism" {
  category "Invariant"
  tier     v2
  text     """
    Given identical .spec files and identical EmbeddingProvider configuration,
    embedding generation and similarity search MUST produce identical results.
    Results MAY vary across different providers or model versions.
  """
}
