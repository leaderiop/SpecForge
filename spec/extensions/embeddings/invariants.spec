// Embeddings extension — invariants for embedding cache and provider behavior

use "extensions/embeddings/behaviors"
use "behaviors/incremental"
use "behaviors/graph"
invariant embedding_cache_consistency "Embedding Cache Consistency" {
  guarantee """
    The embedding cache MUST be invalidated when any entity's content or
    relationships change, ensuring semantic search results always reflect
    current graph state.
  """
  enforced_by [generate_entity_embeddings, search_entities_by_similarity, rebuild_affected_subgraph, compute_graph_delta]
  risk medium

  verify property "embedding cache is invalidated when entity content changes"
  verify unit "semantic search returns results consistent with current graph state"
  verify unit "stale embeddings are never served after graph mutation"

}

invariant embedding_provider_determinism "Embedding Provider Determinism" {
  guarantee """
    Given identical .spec files and identical EmbeddingProvider configuration,
    embedding generation and similarity search MUST produce identical results.
    Results MAY vary across different providers or model versions.
  """
  enforced_by [generate_entity_embeddings, search_entities_by_similarity]
  risk medium

  verify property "identical spec files and provider config produce identical embeddings"
  verify unit "different provider configurations may produce different results"

}
