// Embeddings extension — entity embedding search feature

use "extensions/embeddings/behaviors"
feature entity_embedding_search "Entity Embedding Search" {
  status     roadmap
  behaviors  [generate_entity_embeddings, search_entities_by_similarity]

  problem """
    Agents need semantic search over spec entities to find relevant context
    by meaning rather than exact ID matching. Current graph queries require
    knowing entity IDs or kinds upfront, limiting discoverability for agents
    working with unfamiliar specifications.
  """

  solution """
    Entity embedding generation and vector-based semantic search. Each entity's
    contract, title, and relationships are embedded into a vector space.
    specforge search --semantic queries entities by natural language similarity.
    Embedding generation is delegated to an external embedding provider via the
    extension contribution system — SpecForge does not bundle or invoke any AI/ML
    model. The EmbeddingProvider port abstracts the embedding backend (OpenAI,
    local model, etc.). This ensures SpecForge remains deterministic infrastructure
    per the vision ('not an AI agent itself').
    Future integration with agent memory systems for persistent context.
  """
}
