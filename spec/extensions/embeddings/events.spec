// Embeddings extension — events for embedding generation and search

use "types/output"
event embeddings_generated "Embeddings Generated" {
  /// Emitted when entity embeddings have been generated or updated.
  channel   "embeddings.generated"

  payload {
    entityCount     integer
    dimensions      integer
    timestamp       timestamp
  }


  verify integration "emits embeddings_generated with correct entityCount and dimensions after embedding generation"

}

event entity_search_performed "Entity Search Performed" {
  channel   "embeddings.search"

  payload {
    query           string
    resultCount     integer
    threshold       float
    timestamp       timestamp
  }


  verify integration "emits entity_search_performed with correct query and resultCount"
  verify integration "payload includes similarity threshold used for the search"

}
