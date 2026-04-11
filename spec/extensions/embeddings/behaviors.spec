// Embeddings extension — entity embedding generation and similarity search

use "invariants/core"
use "types/graph"
use "types/output"
use "ports/outbound"
use "events/compilation"
use "extensions/embeddings/ports"
use "extensions/embeddings/types"
use "extensions/embeddings/invariants"
use "extensions/embeddings/events"
behavior generate_entity_embeddings "Generate Entity Embeddings" {
  status     roadmap
  category   query
  invariants [graph_traversal_integrity, embedding_provider_determinism, embedding_cache_consistency]
  types      [Graph, AgentExportConfig, EntityEmbedding]
  ports      [EmbeddingProvider]
  consumes   [validation_complete]
  produces   [embeddings_generated]

  contract """
    When specforge embed is invoked, the system MUST generate vector
    embeddings for all entities in the graph based on their contracts,
    fields, and relationship context. The embeddings MUST be serializable
    for storage and retrieval. Embeddings MUST be persisted in
    .specforge/embeddings/ and invalidated when the graph content hash
    changes. The system MUST compute a content hash of the graph and
    compare it against the stored hash before regenerating embeddings.
    If the hash matches, cached embeddings MUST be reused.
  """

  verify unit "embeddings generated for all graph entities"
  verify unit "embedding includes contract and relationship context"
  verify unit "embeddings persisted in .specforge/embeddings/"
  verify unit "cached embeddings reused when graph content hash unchanged"
  verify unit "embeddings invalidated when graph content hash changes"

}

behavior search_entities_by_similarity "Search Entities by Similarity" {
  status     roadmap
  category   query
  invariants [graph_traversal_integrity, embedding_provider_determinism, embedding_cache_consistency]
  types      [Graph, AgentExportConfig, EntityEmbedding, SimilarityResult]
  ports      [EmbeddingProvider]
  consumes   [validation_complete]
  produces   [entity_search_performed]

  contract """
    When specforge query --semantic is invoked with a natural language query,
    the system MUST perform a similarity search over entity embeddings and
    return the top-k most relevant entities. Results MUST include similarity
    scores and conform to the Graph Protocol schema. Each result MUST be
    an annotated graph node with a _similarity score field in the Graph
    Protocol response. The _similarity field MUST be a float between 0.0
    and 1.0 representing cosine similarity. If no embeddings exist in
    .specforge/embeddings/ (specforge embed has never been run or cache
    was cleared), the system MUST emit an E028 diagnostic with guidance:
    "No entity embeddings found. Run specforge embed first." and return
    an empty result set.
  """

  verify unit "semantic query returns ranked entities by similarity"
  verify unit "results include similarity scores"
  verify unit "results are annotated nodes with _similarity score field"
  verify unit "_similarity is a float between 0.0 and 1.0"
  verify unit "missing embeddings produces E028 diagnostic with guidance"

}
