research 03-RESEARCH "Graph Protocol Schema Versioning research" {
  phase              v1.0.0/03
  domain             "Versioning a self-describing JSON schema that WebAssembly extensions extend at load time"
  researched         2026-11-13
  confidence         high
  summary            "The registries already hold every kind, field and edge type after load, so the schema is a projection of them. Versioning is a structural diff of two schema documents; serde_json reads the cache, and the JSON Schema draft 2020-12 vocabulary covers every field type the registries declare."
  user_constraints   [D-01, D-02, D-03, D-04]
  responsibility_map [
    { capability "Generate the schema" tier "Compiler" }
    { capability "Version the schema" tier "Compiler" }
    { capability "Negotiate a version" tier "CLI and MCP surfaces" }
  ]
}

finding F1 "The registries hold everything the schema needs" {
  research   v1.0.0/03/03-RESEARCH
  topic      architecture
  text       "After every extension is loaded, KindRegistry, FieldRegistry and the edge types are complete and immutable for the compilation, so generate_schema can run once per compilation and every export reads the same value"
  confidence high
}

finding F2 "A structural diff classifies every change" {
  research   v1.0.0/03/03-RESEARCH
  topic      architecture
  text       "Comparing two schema documents attribute by attribute (kinds, fields, field types, required flags, enum values, edge types and their ends, docs, provenance) gives one class per change; the highest class decides the bump"
  confidence high
}

finding F3 "Draft 2020-12 covers the field types" {
  research   v1.0.0/03/03-RESEARCH
  topic      stack
  text       "String, integer, number, bool, date, enum, reference, list and block fields each map to a draft 2020-12 construct; a reference is a string with a pattern and an x-specforge-target keyword"
  confidence medium
}

stack_choice SC1 "serde_json" {
  research v1.0.0/03/03-RESEARCH
  name     "serde_json"
  version  "1"
  purpose  "Read and write the schema cache and the published JSON Schema"
  tier     core
}

pattern P1 "Projection, not maintenance" {
  research v1.0.0/03/03-RESEARCH
  name     "Projection, not maintenance"
  what     "Generate the schema from the registries at every compilation instead of keeping a hand-written copy in step with them"
  when     "Every compilation that exports"
}

pattern P2 "Version stored beside the document" {
  research v1.0.0/03/03-RESEARCH
  name     "Version stored beside the document"
  what     "Write the version a document had into the cache with the document, so the next export compares against exactly what was read"
  when     "After each graph export"
}

pitfall PF1 "A cache written before the export succeeds" {
  research      v1.0.0/03/03-RESEARCH
  name          "A cache written before the export succeeds"
  goes_wrong    "An export that fails after the cache is written leaves a cache whose version no consumer ever saw"
  why           "The cache write and the export are two writes"
  avoid         "Write the cache only after the export's bytes are written"
  warning_signs "A minor bump with no change in the export"
}

pitfall PF2 "Unordered maps in the schema" {
  research      v1.0.0/03/03-RESEARCH
  name          "Unordered maps in the schema"
  goes_wrong    "Two compilations print the schema's kinds in different orders, so the diff sees changes that are not there"
  why           "HashMap iteration order is not stable"
  avoid         "Keep kinds, fields and edge types in BTreeMaps, sorted by name"
  warning_signs "A patch bump on an unchanged project"
}

dont_hand_roll DH1 "JSON Schema validation" {
  research v1.0.0/03/03-RESEARCH
  problem  "Checking that the published schema is valid draft 2020-12"
  instead  "Validate it in the tests with the jsonschema crate's meta-schema"
  why      "The meta-schema has corner cases a hand check misses"
}

open_question OQ1 "Where does the schema cache live?" {
  research       v1.0.0/03/03-RESEARCH
  question       "Does the schema cache live beside the export or under .specforge?"
  known          "Exports may be written anywhere, or to stdout"
  unclear        "Whether two exports to different files share one cache"
  recommendation "One cache per project, under .specforge, as every other cache"
  resolved_by    v1.0.0/02/D-01
}

open_question OQ2 "Does a breaking change fail the export?" {
  research       v1.0.0/03/03-RESEARCH
  question       "Does a major bump fail the export or warn?"
  known          "CI runs export on every change"
  unclear        "Whether a consumer would rather see a failed build"
  recommendation "Warn, and let --strict fail it"
  resolved_by    v1.0.0/03/D-02
}
