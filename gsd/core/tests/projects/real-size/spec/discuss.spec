context 02-CONTEXT "Graph Protocol Export context" {
  phase          v1.0.0/02
  gathered       2026-11-02
  boundary       "The graph is exported as Graph Protocol JSON, nodes and edges in a stable order. A schema in the export is phase 3."
  canonical_refs [
    { topic "The export envelope" paths ["docs/graph-protocol.md"] }
  ]
}

decision D-01 "The schema cache lives under .specforge" {
  phase  v1.0.0/02
  area   "Caches"
  text   "Every cache an export keeps lives under .specforge/ in the project root, ignored by git, one JSON file per cache, rewritten whole after each export"
  origin context
}

decision D-02 "Nodes are sorted by id" {
  phase  v1.0.0/02
  area   "Order"
  text   "Nodes are sorted by id and edges by source, label and target, so two exports of one graph are byte-identical"
  origin context
}

context 03-CONTEXT "Graph Protocol Schema Versioning context" {
  phase              v1.0.0/03
  gathered           2026-11-12
  boundary           "The schema is embedded in graph exports, versioned against the last export's cached schema, and a consumer can name the version it reads. Publishing the schema to a registry is out of scope."
  discretion         ["The order of kinds inside the schema", "The wording of W053's message"]
  specifics          "The version is computed, never written by hand: a person never edits a schema version"
  canonical_refs     [
    { topic "The Graph Protocol envelope and its format_version" paths ["docs/graph-protocol.md"] }
    { topic "Diagnostic codes and their catalog" paths ["docs/diagnostics.md"] }
  ]
  reusable_assets    [
    "`crates/pipeline/emitter/src/schema.rs`: GraphProtocolSchema and generate_schema",
    "`crates/pipeline/emitter/src/emit.rs`: the export shapes and their envelope",
  ]
  patterns           ["One reader per format: the cache is read with serde_json, never a hand parser"]
  integration_points [
    "`specforge export` writes the cache after a graph export",
    "the MCP export tool takes schema_version beside format",
  ]
}

decision D-01 "The version is derived from the cached schema" {
  phase  v1.0.0/03
  area   "Versioning"
  text   "Each export versions its schema against the schema the last export recorded in .specforge/schema-cache.json: major for anything a reader of the recorded schema relies on that is removed or reshaped, minor for additions, patch for documentation and provenance; 1.0.0 when no cache exists"
  origin context
}

decision D-02 "A breaking change warns, it never fails" {
  phase  v1.0.0/03
  area   "Versioning"
  text   "A major bump is W053, a warning naming the two versions and the first change that broke compatibility; the export is written either way"
  origin context
}

decision D-03 "The schema is embedded by default in graph exports" {
  phase  v1.0.0/03
  area   "Embedding"
  text   "A graph export embeds the schema unless --no-schema; context, brief and budgeted exports leave it out unless --with-schema, since the schema can be larger than the entities they keep"
  origin context
}

decision D-04 "A named version is admitted or refused, never relabelled" {
  phase  v1.0.0/03
  area   "Negotiation"
  text   "--schema-version and the export tool's schema_version admit a version whose readers read the current schema and refuse any other; an admitted export still states the current version"
  origin context
}

deferred_idea DI1 "Publish the schema to the registry" {
  phase  v1.0.0/03
  origin discussion
  text   "Publishing each schema version to the extension registry so consumers can fetch old versions"
  why    "Needs the registry's artifact store first"
}
