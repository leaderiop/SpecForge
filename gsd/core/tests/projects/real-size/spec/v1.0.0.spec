milestone v1.0.0 "Graph Protocol 1.0" {
}

phase 02 "Graph Protocol Export" {
  milestone        v1.0.0
  slug             "graph-protocol-export"
  goal             "The compiled graph is exported as deterministic Graph Protocol JSON"
  requirements     [FEATURE-047]
  success_criteria [
    "specforge export prints every node and edge with a format version",
    "Two compilations of the same files export the same bytes",
  ]
}

phase 03 "Graph Protocol Schema Versioning" {
  milestone        v1.0.0
  slug             "graph-protocol-schema-versioning"
  depends_on       [02]
  requirements     [FEATURE-120, FEATURE-048]
  success_criteria [
    "The schema is embedded in graph exports by default (opt out with --no-schema; opt in for context, brief or budgeted exports with --with-schema)",
    "Schema version is computed by diffing every attribute against the last export's cached schema (major for breaking, minor for additions, patch for documentation and provenance; 1.0.0 with no cache)",
    "Breaking changes warn W053 with the versions they move between",
    "--schema-version admits a version whose readers read the current schema and refuses others; the export is never relabelled",
    "specforge schema --publish emits a JSON Schema (draft 2020-12) for graph, context and brief exports",
  ]
  goal             """
    Self-describing graph protocol schema embedded in exports with version auto-computation, breaking change detection, and schema negotiation.
  """
}
