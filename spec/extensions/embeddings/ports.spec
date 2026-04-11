// Embeddings extension — outbound port for AI/ML embedding providers

use "types/errors"
port EmbeddingProvider {
  direction outbound
  category  "ai/embeddings"

  method generateEmbeddings(texts: string[]) -> Result<float[][], EmitterError>
  method computeSimilarity(query: string, candidates: string[]) -> Result<float[], EmitterError>
}
