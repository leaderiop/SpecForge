plan 02-01 "Deterministic export" {
  phase          v1.0.0/02
  type           execute
  files_modified ["crates/pipeline/emitter/src/json.rs"]
  requirements   [FEATURE-047]
  objective      "The graph is exported as Graph Protocol JSON in a stable order (D-02)"
}

task T1 "Sort the export" {
  plan   v1.0.0/02/02-01
  type   auto
  files  ["crates/pipeline/emitter/src/json.rs"]
  action "Sort nodes by id and edges by source, label and target before serializing"
  verify [
    {
      automated  "cargo nextest run -p specforge-emitter json"
      fails_when "a test fails or none runs"
    }
  ]
  done   "Two exports of one graph are byte-identical"
}

truth TR1 "Two exports of one graph are byte-identical" {
  plan v1.0.0/02/02-01
  text "Two exports of one graph are byte-identical"
}

artifact AR1 "The JSON export" {
  plan     v1.0.0/02/02-01
  path     "crates/pipeline/emitter/src/json.rs"
  provides "the sorted Graph Protocol JSON export"
}

plan 03-01 "Embedded, versioned and negotiated schema" {
  phase            v1.0.0/03
  type             execute
  files_modified   [
    "crates/pipeline/emitter/src/schema.rs",
    "crates/pipeline/emitter/src/schema/version.rs",
    "crates/pipeline/emitter/src/schema/cache.rs",
    "crates/pipeline/emitter/src/emit.rs",
    "crates/surfaces/cli/src/export.rs",
    "crates/surfaces/cli/src/schema.rs",
    "crates/surfaces/mcp/src/tools/export.rs",
    "crates/pipeline/emitter/tests/schema_version.rs",
    "crates/surfaces/cli/tests/export.rs",
  ]
  requirements     [FEATURE-120, FEATURE-048]
  objective        "Every graph export carries the schema the registries describe, versioned against the last export's cached schema, and a consumer can name the version it reads (D-01, D-02, D-03, D-04)"
  purpose          "An agent reads the graph's shape from the graph itself, and a consumer pinned to a schema version learns when the shape moved under it"
  output           "The schema module, its version and cache, the export's schema flags, specforge schema --publish, and their tests"
  verification     [
    "cargo nextest run -p specforge-emitter schema",
    "cargo nextest run -p specforge-cli export",
    "cargo gate full --only rustfmt,layers,links",
  ]
  success_criteria [
    "A graph export embeds the schema and states its version",
    "A removed field moves the major version and warns W053",
    "A named version a reader of the current schema reads is admitted, any other refused",
  ]
}

task T1 "Generate and embed the schema" {
  plan         v1.0.0/03/03-01
  type         tdd
  files        [
    "crates/pipeline/emitter/src/schema.rs",
    "crates/pipeline/emitter/src/emit.rs",
    "crates/surfaces/cli/src/export.rs",
  ]
  read_first   [
    "crates/pipeline/registry/src/registries/kind.rs",
    "crates/pipeline/registry/src/registries/field.rs",
    "docs/graph-protocol.md",
  ]
  precondition "Every extension's kinds, fields and edge types are registered before the first export runs: the registry build completes before any emitter call"
  action       """
    Write generate_schema(registries) in crates/pipeline/emitter/src/schema.rs: one SchemaEntityKind per
    registered kind (its name, description, owning extension, fields in declared order with type, required flag,
    enum values and target kinds) and one SchemaEdgeType per edge type (label, source kinds, target kinds,
    cardinality), each map a BTreeMap keyed by name so the document prints in one order. Call it once per
    compilation, after every extension is loaded, and keep the value on the compiled project so every export reads
    the same schema. In emit.rs, embed it as the top-level "schema" key of a graph export, beside format_version
    2.0 and schema_version; leave it out of context, brief and budgeted exports unless --with-schema is given, and
    out of a graph export when --no-schema is. Add --with-schema and --no-schema to specforge export
    (crates/surfaces/cli/src/export.rs), conflicting with each other, and with_schema to the MCP export tool. Write
    the tests first: a graph export of the taskboard twin holds a schema whose kinds are exactly the registered
    kinds; a context export holds none; --with-schema adds it; --no-schema removes it; two compilations print the
    same schema bytes.
  """
  acceptance   [
    "generate_schema lists every registered kind and edge type, sorted by name",
    "a graph export embeds the schema unless --no-schema",
    "a context, brief or budgeted export embeds it only with --with-schema",
    "--with-schema and --no-schema conflict, and the MCP export tool takes with_schema with the same meaning",
    "two compilations of the taskboard twin print byte-identical schemas, kinds, fields and edge types in name order",
    "a kind an optional peer would declare, with that peer not loaded, is absent from the schema, not listed empty",
  ]
  verify       [
    {
      automated  "cargo nextest run -p specforge-emitter schema"
      fails_when "a test fails or none runs"
    }
    { automated "cargo nextest run -p specforge-cli export" fails_when "a test fails or none runs" }
  ]
  done         "Every graph export carries the schema the registries describe"
}

task T2 "Version the schema against the cache" {
  plan         v1.0.0/03/03-01
  type         tdd
  files        [
    "crates/pipeline/emitter/src/schema/version.rs",
    "crates/pipeline/emitter/src/schema/cache.rs",
    "crates/pipeline/emitter/tests/schema_version.rs",
  ]
  read_first   ["crates/pipeline/emitter/src/schema.rs", "docs/diagnostics.md"]
  reversibility {
    rating     costly
    why        "the cache format is read by every later export"
    settled_by v1.0.0/02/D-01
  }
  precondition "generate_schema answers one GraphProtocolSchema per compilation, its maps sorted by name, so two schemas compare attribute by attribute"
  action       """
    Write the cache in crates/pipeline/emitter/src/schema/cache.rs: .specforge/schema-cache.json holds the last
    export's schema and the version it had, read with serde_json and rewritten whole, only after the export's bytes
    are written (a failed export leaves the cache as it was). Write the version in schema/version.rs: compare the
    current schema with the cached one attribute by attribute and classify each difference. A removed kind, edge
    type or field, a new required field, a changed field type, required flag, edge type or target kind, a lost enum
    value or a lost edge end is major; an added kind, field, edge type, enum value or edge end is minor; a changed
    description, doc or provenance is patch. The highest class bumps the cached version; no cache is 1.0.0; no
    difference keeps the cached version. A major bump is W053, a warning naming both versions and the first
    breaking change in path order; the export is written either way. Write the tests first, one per class, each
    over two schemas that differ by one attribute, plus the no-cache and no-change cases and a failed export that
    leaves the cache untouched. A cache that does not parse is reported once, as a warning naming its path, and
    replaced: the export versions its schema as if no cache existed and writes a fresh cache after its bytes. Keep the
    diff a pure function of the two schema documents, with no clock and no file read inside it, so the tests build
    both schemas in memory and never touch the disk; the cache module is the only code that reads or writes the
    file, through the project's file port, never std::fs directly.
  """
  acceptance   [
    "each class of change bumps its part of the version and resets the parts below it",
    "a major bump warns W053 naming both versions and the first breaking change",
    "the cache is written only after the export succeeds",
    "a cache that does not parse is reported and replaced, and the export versions its schema as if no cache existed",
    "an enum value added and another removed in one change is major, the removal deciding the class",
    "W053's message names the old and the new version and the path of the first breaking change, kind then field",
  ]
  verify       [
    {
      automated  "cargo nextest run -p specforge-emitter schema_version"
      fails_when "a test fails or none runs"
    }
  ]
  done         "The schema version is computed from the cached schema and a breaking change warns W053"
}

task T3 "Negotiate and publish the schema" {
  plan         v1.0.0/03/03-01
  type         auto
  files        [
    "crates/surfaces/cli/src/export.rs",
    "crates/surfaces/cli/src/schema.rs",
    "crates/surfaces/mcp/src/tools/export.rs",
    "crates/surfaces/cli/tests/export.rs",
  ]
  read_first   [
    "crates/pipeline/emitter/src/schema/version.rs",
    "crates/surfaces/mcp/src/tools/export.rs",
  ]
  precondition "The version of the current schema is known before any byte of the export is written, so a refused version writes nothing"
  action       """
    Add --schema-version <X.Y.Z> to specforge export and schema_version to the MCP export tool. A named version is
    admitted when a reader of that version reads the current schema: same major, and a minor at or below the
    current one. Anything else is refused with an invalid-input error naming the current version and the oldest
    version it still admits; an admitted export still states the current version, never the named one. Add
    specforge schema --publish, which prints the schema as a standalone JSON Schema (draft 2020-12): one
    definition per kind, its fields as properties with their types, required fields listed, enum values as enum,
    references as strings with a pattern and x-specforge-target, and one top-level schema each for graph, context
    and brief exports. Cover each in crates/surfaces/cli/tests/export.rs: an admitted version, a refused newer
    minor, a refused other major, and a published schema that validates against the draft 2020-12 meta-schema.
    Decide the version before any byte of the export is written, so a refused version writes no file and leaves
    the cache as it was; the refusal is the surface's invalid-input error, with the argument it is about
    (--schema-version on the CLI, schema_version on the MCP tool) and the oldest version the current schema still
    admits, so a consumer pinned to an old major learns which version to move to.
  """
  acceptance   [
    "a version a reader of the current schema reads is admitted and the export states the current version",
    "any other version is refused naming the current version",
    "specforge schema --publish prints a schema that validates against the draft 2020-12 meta-schema",
    "a refused version writes no export file and leaves the schema cache untouched",
    "the MCP export tool refuses a version as invalid input naming the argument schema_version",
    "the published schema defines one top-level schema each for graph, context and brief exports",
  ]
  verify       [
    { automated "cargo nextest run -p specforge-cli export" fails_when "a test fails or none runs" }
    { automated "cargo nextest run -p specforge-cli schema" fails_when "a test fails or none runs" }
  ]
  done         "A consumer can name the version it reads, and the schema is published as JSON Schema"
}

truth TR1 "A graph export holds the schema of every loaded extension" {
  plan         v1.0.0/03/03-01
  text         "Exporting the graph of a project prints a schema whose kinds are exactly the kinds its loaded extensions register, with their fields and edge types"
  verification explicit
}

truth TR2 "A context export holds no schema unless asked" {
  plan v1.0.0/03/03-01
  text "A context, brief or budgeted export prints no schema, and prints it when --with-schema is given"
}

truth TR3 "--no-schema leaves the schema out of a graph export" {
  plan v1.0.0/03/03-01
  text "A graph export with --no-schema prints the nodes and edges and no schema key"
}

truth TR4 "Two compilations print the same schema" {
  plan v1.0.0/03/03-01
  text "Exporting the same project twice prints byte-identical schemas"
}

truth TR5 "The first export is version 1.0.0" {
  plan v1.0.0/03/03-01
  text "A project with no schema cache exports schema version 1.0.0 and writes the cache"
}

truth TR6 "An added field bumps the minor version" {
  plan v1.0.0/03/03-01
  text "After an extension adds a field, the next export states the cached version with its minor part raised and its patch reset"
}

truth TR7 "A removed field bumps the major version and warns" {
  plan v1.0.0/03/03-01
  text "After an extension removes a field, the next export states the next major version and warns W053 naming the old and new versions and the removed field"
}

truth TR8 "A changed description bumps the patch version" {
  plan v1.0.0/03/03-01
  text "After a kind's description changes and nothing else, the next export states the cached version with its patch part raised"
}

truth TR9 "A failed export leaves the cache as it was" {
  plan v1.0.0/03/03-01
  text "When an export fails after computing its version, the schema cache still holds the previous schema and version"
}

truth TR10 "A named version is admitted or refused" {
  plan v1.0.0/03/03-01
  text "An export with --schema-version naming an older minor of the current major succeeds and states the current version; a newer minor or another major is refused naming the current version"
}

truth TR11 "The published schema is valid JSON Schema" {
  plan v1.0.0/03/03-01
  text "specforge schema --publish prints a document that validates against the draft 2020-12 meta-schema and defines graph, context and brief exports"
}

artifact AR1 "Schema generation" {
  plan     v1.0.0/03/03-01
  path     "crates/pipeline/emitter/src/schema.rs"
  provides "generate_schema and the GraphProtocolSchema document"
  exports  ["generate_schema", "GraphProtocolSchema", "SchemaEntityKind", "SchemaEdgeType"]
}

artifact AR2 "Schema version" {
  plan     v1.0.0/03/03-01
  path     "crates/pipeline/emitter/src/schema/version.rs"
  provides "the attribute diff and its classification into major, minor and patch"
  exports  ["version_against", "Change", "Bump"]
}

artifact AR3 "Schema cache" {
  plan     v1.0.0/03/03-01
  path     "crates/pipeline/emitter/src/schema/cache.rs"
  provides "reading and writing .specforge/schema-cache.json"
  contains "schema-cache.json"
}

artifact AR4 "The export's schema embedding" {
  plan     v1.0.0/03/03-01
  path     "crates/pipeline/emitter/src/emit.rs"
  provides "the schema key of a graph export and the rule for context, brief and budgeted exports"
}

artifact AR5 "The export flags" {
  plan     v1.0.0/03/03-01
  path     "crates/surfaces/cli/src/export.rs"
  provides "--with-schema, --no-schema and --schema-version"
}

artifact AR6 "The schema command" {
  plan     v1.0.0/03/03-01
  path     "crates/surfaces/cli/src/schema.rs"
  provides "specforge schema --publish"
}

artifact AR7 "The version tests" {
  plan      v1.0.0/03/03-01
  path      "crates/pipeline/emitter/tests/schema_version.rs"
  provides  "one test per class of change, the no-cache and no-change cases"
  min_lines 120
}

key_link KL1 "The export embeds the generated schema" {
  plan    v1.0.0/03/03-01
  from    "crates/pipeline/emitter/src/emit.rs"
  to      "crates/pipeline/emitter/src/schema.rs"
  via     "generate_schema's value on the compiled project"
  pattern "GraphProtocolSchema"
}

key_link KL2 "The version reads the cache" {
  plan    v1.0.0/03/03-01
  from    "crates/pipeline/emitter/src/schema/version.rs"
  to      "crates/pipeline/emitter/src/schema/cache.rs"
  via     "the cached schema and version"
  pattern "cache::read"
}

key_link KL3 "The CLI negotiates through the version" {
  plan    v1.0.0/03/03-01
  from    "crates/surfaces/cli/src/export.rs"
  to      "crates/pipeline/emitter/src/schema/version.rs"
  via     "the admitted-version check"
  pattern "admits"
}

prohibition PR1 "No hand-written schema" {
  plan         v1.0.0/03/03-01
  statement    "No code keeps a hand-written list of kinds, fields or edge types: the schema is generated from the registries"
  verification judgment
}

prohibition PR2 "No relabelled export" {
  plan         v1.0.0/03/03-01
  statement    "An export never states the version a consumer named instead of the current one"
  verification test
  test         "export::a_named_version_is_admitted_and_the_export_states_the_current_one"
}

prohibition PR3 "No cache before the export" {
  plan         v1.0.0/03/03-01
  statement    "The schema cache is never written before the export's bytes are written"
  verification test
  test         "schema_version::a_failed_export_leaves_the_cache_as_it_was"
}

prohibition PR4 "No hand-rolled JSON reader" {
  plan         v1.0.0/03/03-01
  statement    "The cache is read and written with serde_json only, never a hand parser"
  verification judgment
}

prohibition PR5 "No unordered map in the schema" {
  plan         v1.0.0/03/03-01
  statement    "No HashMap or HashSet decides the order the schema prints in"
  verification judgment
}

plan 03-02 "Schema documentation" {
  phase          v1.0.0/03
  type           execute
  depends_on     [03-01]
  files_modified ["docs/graph-protocol.md"]
  requirements   [FEATURE-048]
  objective      "The Graph Protocol document describes the schema, its version and negotiation"
}

task T1 "Document the schema" {
  plan   v1.0.0/03/03-02
  type   auto
  files  ["docs/graph-protocol.md"]
  action "Describe the schema key, how its version is computed, W053 and --schema-version in docs/graph-protocol.md"
  verify [
    { automated "cargo gate full --only links,doc-links" fails_when "the gate exits non-zero" }
  ]
  done   "The document describes the schema and its version"
}

truth TR1 "The document describes the schema version" {
  plan v1.0.0/03/03-02
  text "A reader of docs/graph-protocol.md learns how the schema version is computed and negotiated"
}

artifact AR1 "Graph Protocol document" {
  plan     v1.0.0/03/03-02
  path     "docs/graph-protocol.md"
  provides "the schema, its version and negotiation"
}
