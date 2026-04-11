// Zero-entity core — manifest V2, dynamic registries, grammar, bootstrap, visualization, consistency

use "invariants/zero-entity-core"
use "invariants/core"
use "types/zero-entity-core"
use "types/core"
use "types/config"
use "types/diagnostics"
use "types/errors"
use "types/wasm"
use "ports/outbound"
use "events/compilation"
// -- Extension Manifest V2 ---------------------------------------------------

behavior validate_manifest_v2_schema "Validate Manifest V2 Schema" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   validation
  types      [ManifestV2, ExtensionError]

  requires {
    manifest_json_available "Manifest JSON has been parsed from disk and is available as a structured object"
  }

  ensures {
    schema_validated "Manifest passes all v2 schema checks — required fields present, manifest_version is 2, contributions structurally correct"
    malformed_diagnosed "Malformed JSON or missing required fields produce hard error diagnostics"
  }

  contract """
    The compiler MUST validate a manifest against the v2 schema. This
    behavior is a pure schema check — it does NOT handle v1 detection or
    migration. Required fields (name, version, manifest_version, wasm_path)
    MUST be present. The manifest_version MUST be 2. Unknown top-level
    fields MUST produce a warning. Grammar and body parser contribution
    arrays, when present, MUST be validated for structural correctness:
    entity_kinds MUST be non-empty arrays, grammar_wasm_path and
    export_name MUST be non-empty strings. Malformed JSON MUST produce a
    hard error. This behavior is called by validate_extension_manifest
    (behaviors/wasm-lifecycle.spec) after initial manifest parsing. Schema
    validation MUST complete for all manifests before registry population
    begins.
  """

  verify unit "valid v2 manifest passes schema validation"
  verify unit "missing required field produces hard error"
  verify unit "manifestVersion != 2 produces hard error"
  verify unit "unknown top-level field produces warning"
  verify contract "requires/ensures consistency for manifest v2 schema validation"

}

behavior register_entity_kinds_from_manifest "Register Entity Kinds From Manifest" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   command
  types      [ManifestV2, ManifestEntityKind, KindRegistryEntry]
  consumes   [extension_manifests_loaded]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
  }

  ensures {
    kinds_registered "Every entityKinds entry from the manifest is registered in the KindRegistry with full metadata"
    source_extension_recorded "Source extension name recorded on each KindRegistryEntry for diagnostics"
  }

  contract """
    For each entityKinds entry in a extension manifest, the compiler MUST
    register the kind in the KindRegistry with full metadata: testable flag,
    singleton flag, supportsVerify flag, semantic token classification,
    LSP icon for outline, and DOT shape for
    visualization. The source extension name MUST be recorded for diagnostics
    and doctor output.
  """

  verify unit "entity kind registered with testable flag"
  verify unit "entity kind registered with singleton flag"
  verify unit "entity kind registered with LSP metadata"
  verify unit "source extension recorded in registry entry"
  // Testability registration (formerly register_testability_from_manifest)
  verify unit "testable=true entity participates in coverage"
  verify unit "testable=false entity excluded from coverage"
  verify unit "no default testability assumed by core"
  verify contract "requires/ensures consistency for entity kind registration"

}

behavior register_edge_types_from_manifest "Register Edge Types From Manifest" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   command
  types      [ManifestV2, ManifestEdgeType]
  consumes   [extension_manifests_loaded]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
  }

  ensures {
    edge_types_registered "Every edgeTypes entry and field-to-edge mapping is registered in the edge type set"
    constraints_recorded "Source and target kind constraints recorded for each edge type"
    duplicates_warned "Duplicate edge labels across extensions produce W-level warnings"
  }

  contract """
    For each edgeTypes entry in a extension manifest, the compiler MUST
    register the edge label in the edge type set. The source and target
    kind constraints MUST be recorded for graph validation. Field-to-edge
    mappings from ManifestField entries MUST create corresponding edge
    type registrations.

    Duplicate edge labels across extensions MUST produce a W-level warning
    including both extension names. Resolution is deterministic:
    first-registered wins, where registration order follows topological
    sort of peer dependencies. Extensions without dependency relationships
    are ordered alphabetically by name. The winning registration's
    source_kind and target_kind constraints are used for graph validation;
    the duplicate's constraints are discarded.
  """

  verify unit "edge type registered with label and description"
  verify unit "source/target kind constraints recorded"
  verify unit "duplicate edge label across extensions produces W-level warning"
  verify unit "first-registered edge type wins on collision (topological order)"
  verify unit "field-to-edge mapping creates edge type"
  verify contract "requires/ensures consistency for edge type registration"

}

behavior register_validation_rules_from_manifest "Register Validation Rules From Manifest" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation, declarative_validation_determinism]
  category   command
  types      [ManifestV2, ValidationRulePattern]
  consumes   [extension_manifests_loaded]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
  }

  ensures {
    rules_registered "Every validationRules entry is parsed and stored with raw target_kind and edge_type strings"
    deferred_validation_complete "Post-registration cross-reference validation executed after registries_populated"
    invalid_refs_warned "Invalid target_kind or edge_type references produce warnings, not hard errors"
  }

  contract """
    For each validationRules entry in a extension manifest, the compiler
    MUST parse and register the declarative rule. Registration is a two-step
    process:

    Step 1 (during registration): Rules are parsed and stored with their
    raw target_kind and edge_type strings. No cross-reference validation
    occurs at this point because peer-dependency extensions may not have
    registered their kinds yet.

    Step 2 (post-registration, after registries_populated): The compiler
    MUST validate all registered rules for internal consistency —
    target_kind references MUST match registered entity kinds, edge_type
    references MUST match registered edge types. Invalid references MUST
    produce a warning, not a hard error, to allow partial loading. This
    deferred validation ensures that cross-extension references (e.g., a
    governance rule targeting a software entity kind) resolve correctly
    regardless of extension load order.
  """

  verify unit "validation rule registered from manifest"
  verify unit "target_kind validation deferred to post-registration phase"
  verify unit "target_kind reference validated against KindRegistry after registries_populated"
  verify unit "edge_type reference validated against edge type set after registries_populated"
  verify unit "invalid reference produces warning not error"
  verify contract "requires/ensures consistency for validation rule registration"

}

behavior register_verify_kinds_from_manifest "Register Verify Kinds From Manifest" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   validation
  types      [ManifestV2, ManifestEntityKind]
  consumes   [extension_manifests_loaded]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
  }

  ensures {
    verify_kinds_registered "All allowedVerifyKinds from manifest entityKinds are registered per entity kind"
    no_hardcoded_kinds "No built-in verify kind names exist — all come from extension manifests"
  }

  contract """
    When a manifest entityKind has an allowedVerifyKinds field, the compiler
    MUST register those verify kinds for entities of that kind. Verify kind
    names MUST NOT be hardcoded (no built-in verify kind names — all come from extension manifests).
    Extensions MAY define arbitrary verify kinds (e.g., "contract", "smoke",
    "chaos"). Unknown verify kinds in .spec files MUST produce a warning
    referencing the entity's extension.
  """

  verify unit "custom verify kinds registered from manifest"
  verify unit "no hardcoded verify kinds in core"
  verify unit "unknown verify kind in .spec produces W-level diagnostic in Phase 2"
  verify contract "requires/ensures consistency for verify kind registration"

}

// -- Dynamic Entity Registration ---------------------------------------------

behavior boot_empty_kind_registry "Boot Empty Kind Registry" {
  invariants [zero_domain_knowledge_core]
  category   command
  types      [KindRegistryEntry]

  requires {
    compiler_initializing "Compiler initialization has started and memory is allocated"
  }

  ensures {
    kind_registry_empty "KindRegistry contains zero entity kind entries"
    structural_keywords_ready "Parser recognizes only structural keywords (spec, ref, use, define)"
  }

  contract """
    When the compiler initializes, KindRegistry::new() MUST return an
    empty registry with zero entity kind entries. Only the structural
    keywords spec, ref, use, and define MUST be recognized by the
    parser (these have dedicated grammar rules). No extension-defined
    entity keywords MUST exist until extensions populate the registry.
  """

  verify unit "KindRegistry::new() has zero entries"
  verify unit "parser recognizes spec keyword without extensions"
  verify unit "parser recognizes ref keyword without extensions"
  verify unit "parser recognizes use keyword without extensions"
  verify unit "parser recognizes define keyword without extensions"
  verify contract "requires/ensures consistency for empty kind registry boot"

}

behavior boot_empty_field_registry "Boot Empty Field Registry" {
  invariants [zero_domain_knowledge_core]
  category   command
  types      [FieldRegistryEntry]

  requires {
    compiler_initializing "Compiler initialization has started and memory is allocated"
  }

  ensures {
    field_registry_empty "FieldRegistry contains zero field definitions"
    no_fields_recognized "No extension-defined field names are recognized before population"
  }

  contract """
    When the compiler initializes, FieldRegistry::new() MUST return an
    empty registry with zero field definitions. No extension-defined field
    names MUST be recognized until extensions populate the registry. The
    entity title (the string after the entity keyword and ID) is a
    grammar-level positional element parsed by the generic_entity_block
    rule — it is NOT a FieldRegistry entry and does not participate in
    field validation.
  """

  verify unit "FieldRegistry::new() has zero entries"
  verify unit "no field names recognized before extension loading"
  verify unit "entity title parsed by grammar, not FieldRegistry"
  verify contract "requires/ensures consistency for empty field registry boot"

}

behavior boot_empty_edge_registry "Boot Empty Edge Registry" {
  invariants [zero_domain_knowledge_core]
  category   command
  types      [ManifestEdgeType]

  requires {
    compiler_initializing "Compiler initialization has started and memory is allocated"
  }

  ensures {
    edge_registry_empty "Edge type set contains zero registered edge labels"
    no_edges_recognized "No extension-defined edge types exist before population"
  }

  contract """
    When the compiler initializes, the edge type set MUST start empty with
    zero registered edge labels. No extension-defined edge types MUST exist
    until extensions populate the set. This parallels boot_empty_kind_registry
    and boot_empty_field_registry — all three registries begin empty and are
    populated exclusively from extension manifests.
  """

  verify unit "edge type set starts with zero entries"
  verify unit "no edge labels recognized before extension loading"
  verify contract "requires/ensures consistency for empty edge registry boot"

}

behavior custom_entity_types_via_define "Custom Entity Types via Define" {
  invariants [reference_resolution_completeness, entity_id_uniqueness, zero_domain_knowledge_core, define_extension_kind_uniqueness, compilation_pipeline_ordering]
  category   command
  types      [CompilerConfig, DefineBlockConfig, KindRegistryEntry]
  consumes   [registries_populated]
  produces   [custom_entity_type_defined, define_blocks_registered]

  requires {
    registries_populated_fired "registries_populated event has fired, confirming all extension registries are fully populated"
  }

  ensures {
    custom_kinds_registered "Each define block is registered as a KindRegistryEntry with source_extension '<project>'"
    events_emitted "custom_entity_type_defined event emitted per define block, define_blocks_registered emitted after all"
    resolution_participation "Custom entities participate in reference resolution and orphan detection"
  }

  contract """
    When a define block exists in a .spec file, the compiler MUST
    register the custom entity type as a DefineBlockConfig with its
    id_prefix, required fields, optional fields, and reference targets.
    The custom kind MUST be added to the KindRegistry so that subsequent
    entity blocks using that keyword are recognized. Custom entities MUST
    participate in reference resolution and orphan detection like
    extension-defined entities.

    Define blocks are "project-scoped domain knowledge" — they allow
    projects to extend the entity vocabulary without writing an extension.
    Define blocks MUST be processed in Phase 2, after extension loading
    and registry population (registries_populated event). This ordering
    ensures that define blocks MAY reference extension-defined kinds in
    their reference_targets (e.g., a custom kind targeting an entity
    kind from an installed extension). After processing, the compiler
    MUST emit a custom_entity_type_defined event for each define block.
    Define-block kinds are registered with source_extension set to
    "<project>" to distinguish them from extension-provided kinds.
  """

  verify unit "custom entity type is registered in KindRegistry"
  verify unit "custom entity participates in reference resolution"
  verify unit "custom entity has orphan detection"
  verify unit "define block creates DefineBlockConfig with correct fields"
  verify unit "define blocks processed after registries_populated"
  verify unit "define block can reference extension-defined kinds"
  verify unit "define-block kind has source_extension '<project>'"
  verify unit "custom_entity_type_defined event emitted per define block"
  verify contract "requires/ensures consistency for custom entity type registration"

}

behavior populate_kind_registry_from_extensions "Populate Kind Registry From Extensions" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation, define_extension_kind_uniqueness, compilation_pipeline_ordering]
  category   command
  types      [ManifestV2, ManifestEntityKind, KindRegistryEntry]
  produces   [registries_populated]

  requires {
    manifests_loaded        "All extension manifests MUST be loaded and their entity_kinds arrays accessible"
    kind_registry_empty     "KindRegistry MUST be in its initial empty state (boot_empty_kind_registry completed)"
  }

  ensures {
    one_entry_per_kind      "KindRegistry contains exactly one entry per unique entity kind declared across all installed extension manifests"
    registries_event_fired  "registries_populated event emitted after all three registries are populated"
  }

  maintains {
    zero_domain_knowledge   "No domain-specific logic introduced during population — only structural metadata registered"
  }

  contract """
    After loading all extension manifests, the compiler MUST populate the
    KindRegistry by iterating extensions in topological order (peer deps
    first). For each extension, each entityKinds entry MUST be registered
    as a KindRegistryEntry. After population, the parser MUST recognize
    all registered keywords for subsequent files. Population MUST complete
    before any semantic validation begins.
  """

  verify unit "extensions iterated in topological order"
  verify unit "all entityKinds entries registered"
  verify unit "registered keywords available to parser"
  verify unit "population completes before validation"
  verify integration "two extensions register kinds without collision"
  verify contract "requires/ensures consistency for registry population"

}

behavior populate_field_registry_from_extensions "Populate Field Registry From Extensions" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   command
  types      [ManifestV2, ManifestField, FieldRegistryEntry]
  consumes   [extension_manifests_loaded]
  // Orchestrated by populate_kind_registry_from_extensions which fires registries_populated after all three complete.

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
    kind_registry_populated "KindRegistry is already populated so field registrations can reference valid entity kinds"
  }

  ensures {
    fields_registered "Every ManifestEntityKind's fields array is registered in the FieldRegistry"
    field_types_validated "All field type declarations validated against ManifestFieldType variants"
    fields_populated "FieldRegistry is fully populated and ready for downstream consumers"
  }

  contract """
    After populating the KindRegistry, the compiler MUST populate the
    FieldRegistry from all extension manifests. Each ManifestEntityKind's
    fields array MUST be registered as valid field definitions for that
    entity kind. Field types (string, string[], reference, reference[],
    block) MUST be validated against ManifestFieldType variants. Invalid
    field types MUST produce a warning. Note: verify is NOT a field type
    — it is a grammar-level construct governed by the supports_verify
    flag on ManifestEntityKind.
  """

  verify unit "fields registered per entity kind"
  verify unit "field types validated against known types"
  verify unit "invalid field type produces warning"
  verify contract "requires/ensures consistency for field registry population"

}

behavior populate_edge_registry_from_extensions "Populate Edge Registry From Extensions" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   command
  types      [ManifestV2, ManifestEdgeType, ManifestField, EdgeRegistryEntry]
  consumes   [extension_manifests_loaded]
  // Orchestrated by populate_kind_registry_from_extensions which fires registries_populated after all three complete.

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
  }

  ensures {
    edge_set_complete "Complete edge type set built from both explicit edgeTypes and implicit field-to-edge mappings"
    duplicates_warned "Duplicate edge labels produce warnings but are not rejected"
    edges_populated "EdgeRegistry is fully populated and ready for downstream consumers"
  }

  contract """
    The compiler MUST build the complete edge type set from two sources:
    explicit edgeTypes declarations in manifests, and implicit edge types
    derived from ManifestField entries with an edge property. Both sources
    MUST be merged into a single edge type set before graph construction.
    Duplicate edge labels MUST be warned about but not rejected.
  """

  verify unit "explicit edgeTypes merged into edge set"
  verify unit "implicit edges from field mappings merged"
  verify unit "duplicate edge labels produce warning"
  verify contract "requires/ensures consistency for edge registry population"

}

behavior validate_registered_entity_fields "Validate Registered Entity Fields" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   validation
  types      [ManifestV2, ManifestField, ManifestEntityKind, ManifestEdgeType, FieldRegistryEntry, KindRegistryEntry]

  consumes  [registries_populated]

  requires {
    registries_populated    "All three registries (KindRegistry, FieldRegistry, EdgeRegistry) are populated"
    populated_event_fired   "registries_populated event has fired"
  }

  ensures {
    target_kinds_resolved   "Every field target_kind reference resolves to a registered kind"
    unresolved_diagnosed    "Diagnostics emitted for unresolved target_kind references"
  }

  maintains {
    no_domain_logic         "Cross-validation uses only structural checks — no domain-specific logic"
  }

  contract """
    After all registries are populated, the compiler MUST cross-validate
    the registered data using only structural checks — no domain-specific
    logic. target_kind references in fields MUST resolve to registered
    entity kinds. Edge labels in field-to-edge mappings MUST resolve to
    registered edge types. Field type declarations MUST be internally
    consistent. Validation failures MUST produce warnings to allow partial
    loading, not hard errors.
  """

  verify unit "target_kind reference resolves to registered kind"
  verify unit "edge label resolves to registered edge type"
  verify unit "unresolved target_kind produces warning"
  verify unit "unresolved edge label produces warning"
  verify unit "cross-validation uses no domain-specific logic"
  verify contract "requires/ensures consistency for field cross-validation"

}

// -- Grammar Consolidation ---------------------------------------------------

behavior collapse_grammar_to_generic_entity_block "Collapse Grammar to Generic Entity Block" {
  invariants [zero_domain_knowledge_core]
  category   command
  types      [SpecFile, Entity]

  requires {
    grammar_source_available "Tree-sitter grammar source is available for compilation"
  }

  ensures {
    single_generic_rule "Grammar contains exactly one generic entity_block rule for all extension keywords"
    structural_rules_preserved "spec_block, ref_block, use_import, and define_block remain as separate grammar rules"
    no_keyword_validation_in_grammar "All keyword validation deferred to semantic phase"
  }

  contract """
    The tree-sitter grammar MUST have exactly ONE generic entity_block rule
    that parses any keyword id [title] { fields } structure. All per-keyword
    block rules that existed in the pre-zero-entity grammar MUST be collapsed
    into this single generic rule. spec_block, ref_block, use_import, and
    define_block MUST remain as separate grammar rules because they have unique
    structural syntax (ref has scheme:identifier ID format, spec has singleton
    semantics). All keyword validation MUST happen in the semantic phase, not
    the grammar.
  """

  verify unit "grammar has single generic entity_block rule"
  verify unit "no per-keyword block rules remain in grammar"
  verify unit "spec_block remains as separate grammar rule"
  verify unit "ref_block remains as separate grammar rule"
  verify unit "use_import remains as separate grammar rule"
  verify unit "define_block remains as separate grammar rule"
  verify contract "requires/ensures consistency for grammar collapse"

}

// -- Zero-Entity Bootstrap ---------------------------------------------------

behavior two_phase_parse_structural "Two-Phase Parse: Structural" {
  invariants [registry_population_before_validation, zero_domain_knowledge_core, compilation_pipeline_ordering]
  category   command
  types      [SpecFile, Entity]
  produces   [structural_parse_complete]

  requires {
    spec_files_available "All .spec files discovered and readable from the configured spec_root"
  }

  ensures {
    structural_parse_produced "SpecFile ASTs produced with generic entity blocks for every keyword name { } block"
    structural_parse_event_emitted "structural_parse_complete event emitted after all files are parsed"
    no_keyword_validation "No keyword validation performed — all keywords accepted structurally"
  }

  contract """
    In Phase 1 of the two-phase compilation, the parser MUST perform
    purely structural parsing of all .spec files. Every keyword name { }
    block MUST be parsed into a generic entity node regardless of whether
    the keyword is registered. No keyword validation MUST occur in Phase 1.
    The output MUST be a list of SpecFile ASTs with generic entity blocks.
    After structural parsing and registry population, Phase 1.5 MUST
    dispatch registered body parsers for entity kinds that have them,
    transforming raw body text into structured fields before Phase 2
    semantic validation.
  """

  verify unit "unknown keyword parsed into generic entity node"
  verify unit "no keyword validation in Phase 1"
  verify unit "all .spec files parsed before Phase 2"
  verify unit "parse errors collected without aborting"
  verify contract "requires/ensures consistency for structural parsing"

}

behavior two_phase_validate_semantic "Two-Phase Validate: Semantic" {
  invariants [registry_population_before_validation, zero_domain_knowledge_core, compilation_pipeline_ordering]
  category   validation
  types      [KindRegistryEntry, FieldRegistryEntry]
  consumes   [registries_populated, define_blocks_registered]
  // barrier: MUST wait for ALL consumed events (registries_populated AND define_blocks_registered) before executing

  requires {
    registries_populated      "registries_populated event MUST have fired, confirming KindRegistry, FieldRegistry, and edge type set are fully populated"
    define_blocks_registered  "define_blocks_registered event MUST have fired, confirming all project-defined entity kinds are registered"
  }

  ensures {
    all_blocks_checked        "Every parsed entity block checked against the KindRegistry"
    unknown_keywords_diagnosed "Unknown keywords have E024 diagnostics emitted"
    fields_validated          "Known keywords have their fields validated against the FieldRegistry"
  }

  contract """
    This behavior orchestrates the two-phase pipeline by invoking
    resolution and validation behaviors in sequence. It does not
    duplicate their logic.
    Phase 2 of the two-phase compilation includes resolution and semantic
    validation. First, the resolver MUST link references and build graph
    edges using the populated registries. Then, the validator MUST check
    all entity keywords against the populated KindRegistry. Unknown
    keywords MUST produce E024 diagnostics. Known keywords MUST have
    their fields validated against the FieldRegistry via
    detect_unknown_entity_fields. Phase 2 MUST NOT begin until all
    extension manifests are loaded and all registries are fully populated.
  """

  verify unit "known keyword passes semantic validation"
  verify unit "unknown keyword produces E024"
  verify unit "field validation uses FieldRegistry"
  verify unit "Phase 2 starts only after registries populated"
  verify integration "Phase 2 waits for both registries_populated AND define_blocks_registered"
  verify contract "requires/ensures consistency for semantic validation"

}

// The KeywordExtensionIndex is a static JSON file mapping known entity keywords
// to their providing extension names. It is generated from the extension registry
// at release time and shipped as a bundled data file. It does NOT require network
// access at runtime.
//
// Bridge: The bundled KeywordExtensionIndex data file is produced by
// generate_keyword_extension_index (behaviors/extensions.spec) at build/release
// time. That behavior belongs to extension_registry feature — this behavior
// consumes its output artifact at runtime.
//
// Cross-ref: The KeywordExtensionIndex JSON file is generated by
// generate_keyword_extension_index (behaviors/extensions.spec:523) at build
// time. If the index generation behavior changes, this consumer must be
// updated to match the new format.
// suggest_missing_extensions is invoked inline by the diagnostic pipeline when
// an E024 (unknown entity kind) is emitted — it is not event-driven. It enriches
// the diagnostic help text with extension suggestions from the bundled index.
behavior suggest_missing_extensions "Suggest Missing Extensions" {
  invariants [zero_domain_knowledge_core]
  category   command
  types      [KindRegistryEntry, UnknownKindError, KeywordExtensionIndex]

  requires {
    e024_diagnostic_emitted "An E024 (unknown entity kind) diagnostic has been emitted and requires help text enrichment"
  }

  ensures {
    suggestion_provided "Help text includes extension suggestion from bundled index or fallback to specforge search"
    lazy_loading_enforced "KeywordExtensionIndex loaded lazily on first E024, not at startup"
  }

  contract """
    When an E024 (unknown entity kind) diagnostic is emitted, the help
    text MUST suggest which extension provides the unknown keyword, if known.
    The suggestion MUST use a data-driven keyword-to-extension index shipped
    as a bundled data file (not hardcoded in compiler source). The index maps
    common keywords to their providing extensions. If no mapping exists for
    the unknown keyword, the help text MUST suggest running specforge search.
    Loading MUST be lazy — triggered on first E024 occurrence, not at startup.
    If the bundled file is missing or malformed, the behavior MUST silently
    fall back to suggesting specforge search for all unknown keywords.
  """

  verify unit "E024 for keyword in index suggests the providing extension"
  verify unit "E024 for keyword not in index suggests specforge search"
  verify unit "keyword-to-extension index is loaded from bundled data file"
  verify contract "requires/ensures consistency for missing extension suggestions"

}

// detect_unknown_entity_kinds and suggest_missing_extensions live in the
// registries file because they query the KindRegistry directly to determine
// whether a keyword is registered. They are validation behaviors that depend
// on registry state rather than on the validation rule engine.
behavior detect_unknown_entity_kinds "Detect Unknown Entity Kinds" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   validation
  types      [KindRegistryEntry, UnknownKindError]
  consumes   [registries_populated]

  requires {
    registries_populated_fired "registries_populated event has fired, confirming KindRegistry is fully populated"
    structural_parse_ready "All .spec files have been structurally parsed into generic entity blocks"
  }

  ensures {
    unknown_kinds_diagnosed "E024 diagnostic emitted for every keyword not present in the KindRegistry"
    registered_kinds_accepted "Registered keywords pass without E024 diagnostics"
  }

  contract """
    After Phase 1 parsing and registry population, the compiler MUST scan
    all parsed entity blocks and check each keyword against the KindRegistry.
    Every keyword not present in the registry MUST produce an E024 diagnostic
    with the entity's source span. The diagnostic MUST include the unrecognized
    keyword and the file location.
  """

  verify unit "unregistered keyword produces E024"
  verify unit "E024 includes keyword name and source span"
  verify unit "registered keyword does not produce E024"
  verify unit "define-block keywords not checked against KindRegistry"
  verify contract "requires/ensures consistency for unknown entity kind detection"

}

behavior graceful_degradation_without_extensions "Graceful Degradation Without Extensions" {
  invariants [zero_domain_knowledge_core]
  category   command
  types      [SpecFile, KindRegistryEntry]
  consumes   [registries_populated]

  requires {
    registries_populated_fired "registries_populated event has fired (with empty registries when no extensions are installed)"
  }

  ensures {
    i002_emitted "I002 info diagnostic emitted indicating no extensions are configured"
    structural_mode_operational "Compiler operates in structural-only mode with generic entity nodes"
    valid_export_produced "specforge export produces valid JSON from structural-only graph"
  }

  contract """
    When no extensions are installed, the compiler MUST still function. It
    MUST emit an I002 info diagnostic indicating that no extensions are
    configured. It MUST parse all .spec files structurally. The graph MUST
    be built with generic entity nodes but no kind-specific validation.
    The LSP MUST provide basic features (syntax highlighting, folding)
    without extension-driven completions.

    Reference edges are structural — they are implicit edges created by the
    parser when one entity's reference list mentions another entity's ID.
    These edges do not require registered edge types in the EdgeRegistry.
    The EdgeRegistry governs extension-defined semantic edge types only.

    In structural-only mode, the kind field of exported entity nodes MUST
    contain the raw keyword string as parsed from the .spec file.
  """

  verify unit "no extensions installed emits I002 info"
  verify unit "structural parsing works without extensions"
  verify unit "graph built with generic nodes"
  verify unit "LSP provides basic features without extensions"
  verify unit "specforge export produces valid JSON from structural-only graph"
  verify unit "generic entity nodes appear as nodes in exported graph"
  verify unit "references between generic entities produce edges"
  verify integration "specforge check with zero extensions exits cleanly with I002"
  verify contract "requires/ensures consistency for graceful degradation"

}

behavior handle_all_extensions_failed_to_load "Handle All Extensions Failed to Load" {
  invariants [zero_domain_knowledge_core, multi_error_collection]
  category   command
  types      [ExtensionError, Diagnostic]
  consumes   [extension_manifests_loaded]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming load attempts completed (even if all failed)"
  }

  ensures {
    per_extension_errors_emitted "E-level diagnostic emitted for each failed extension"
    structural_mode_fallback "System transitions to structural-only mode after all failures"
    no_crash_guaranteed "System does not crash or enter an undefined state"
  }

  contract """
    If all declared extensions in specforge.json fail to load (network errors,
    missing Wasm binaries, invalid manifests), the compiler MUST emit an E-level
    diagnostic for each failed extension, then transition to structural-only mode
    (identical to graceful_degradation_without_extensions). The system MUST NOT
    crash or enter an undefined state. An I002 info diagnostic MUST indicate
    the system is operating in structural-only mode due to load failures.
  """

  verify unit "all extensions failing produces per-extension E-level diagnostics"
  verify unit "system transitions to structural-only mode after all failures"
  verify integration "specforge check with all extensions unavailable exits cleanly"
  verify contract "requires/ensures consistency for all-extensions-failed handling"
}

// -- Extension-Driven Visualization ------------------------------------------
// These visualization behaviors are synchronous delegates called by
// serialize_dot_visualization (behaviors/output.spec) — they do not consume
// events or own ports directly.

behavior render_extension_defined_dot_shapes "Render Extension-Defined DOT Shapes" {
  invariants [zero_domain_knowledge_core]
  category   query
  types      [KindRegistryEntry]

  requires {
    kind_registry_available "KindRegistry is populated and queryable for dot_shape, dot_color, dot_fillcolor metadata"
    graph_available "Compiled graph with entity nodes is available for rendering"
  }

  ensures {
    shapes_applied "Every entity node rendered with its extension-defined DOT shape or default 'box'"
    colors_applied "dot_color and dot_fillcolor attributes set on nodes when specified in manifest"
  }

  contract """
    When rendering DOT visualization, the graph renderer MUST query the
    KindRegistry for each entity's dot_shape field. Entity nodes MUST use
    the extension-defined DOT shape (box, ellipse, diamond, hexagon, etc.).
    If no dot_shape is specified in the manifest, the default shape MUST
    be "box". The shape MUST appear in the DOT node attribute list.
    When dot_color is specified in the manifest, the renderer MUST set
    the DOT node color attribute. When dot_fillcolor is specified, the
    renderer MUST set the DOT node fillcolor attribute AND add
    style=filled to the node attributes.
  """

  verify unit "entity uses extension-defined dot_shape"
  verify unit "default shape is box when dot_shape not specified"
  verify unit "dot_shape appears in DOT node attributes"
  verify unit "dot_color sets DOT node color attribute"
  verify unit "dot_fillcolor sets DOT node fillcolor and style=filled attributes"
  verify integration "multi-extension graph renders each kind with its declared shape"
  verify contract "requires/ensures consistency for DOT shape rendering"

}

behavior render_extension_defined_edge_styles "Render Extension-Defined Edge Styles" {
  invariants [zero_domain_knowledge_core]
  category   query
  types      [ManifestEdgeType, EdgeRegistryEntry]

  requires {
    edge_registry_available "Edge type registry is populated and queryable for style metadata"
    graph_available "Compiled graph with typed edges is available for rendering"
  }

  ensures {
    styles_applied "Every edge rendered with its extension-defined style or default 'solid'"
    edge_colors_applied "edge_color and edge_arrowhead attributes set on edges when specified in manifest"
  }

  contract """
    When rendering graph visualization, the compiler MUST query the edge
    type registry for style metadata on each edge. Edge styles MUST be
    one of: solid, dashed, or dotted. If no edge_style is defined for an
    edge type in the extension manifest, the compiler MUST default to a
    solid line. The style MUST appear in the DOT edge attribute list as
    the style property. When edge_color is specified, the renderer MUST
    set the DOT edge color attribute (default: "black"). When
    edge_arrowhead is specified, the renderer MUST set the DOT edge
    arrowhead attribute (default: "normal").
  """

  verify unit "edge uses extension-defined edge_style"
  verify unit "default style is solid when edge_style not specified"
  verify unit "edge_style appears in DOT edge attributes"
  verify unit "edge_color sets DOT edge color attribute"
  verify unit "edge_arrowhead sets DOT edge arrowhead attribute"
  verify integration "multi-extension graph renders each edge type with its declared style"
  verify contract "requires/ensures consistency for DOT edge style rendering"

}

// -- Grammar and Body Parser Registration ------------------------------------

behavior register_grammar_contributions "Register Grammar Contributions" {
  invariants [zero_domain_knowledge_core, grammar_composition_determinism]
  category   command
  types      [GrammarContribution, ManifestV2, KindRegistryEntry]
  consumes   [extension_manifests_loaded]
  produces   [grammar_contribution_registered]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
    kind_registry_available "KindRegistry is populated so entity_kinds references can be validated"
  }

  ensures {
    grammar_contributions_registered "Valid grammar contributions stored for LSP injection and Phase 1.5 dispatch"
    grammar_event_emitted "grammar_contribution_registered event emitted for each successful registration"
    conflicts_resolved "Grammar conflicts for the same entity kind resolved per grammar_policy"
  }

  contract """
    For each grammar_contributions entry in a loaded extension manifest,
    the compiler MUST validate that the declared entity_kinds reference
    registered kinds in the KindRegistry. The grammar .wasm path MUST be
    validated as accessible. When multiple extensions declare grammar
    contributions for the same entity kind, the grammar_policy from
    CompilerConfig MUST be applied. Successfully registered grammar
    contributions MUST be stored for LSP grammar injection and Phase 1.5
    body parser dispatch.
  """

  verify unit "grammar contribution registered for valid entity kind"
  verify unit "grammar contribution for unregistered kind produces warning"
  verify unit "grammar conflict detected and policy applied"
  verify unit "grammar .wasm path validated as accessible"
  verify contract "requires/ensures consistency for grammar contribution registration"

}

behavior register_body_parser_contributions "Register Body Parser Contributions" {
  invariants [zero_domain_knowledge_core, body_parser_output_conformance]
  category   command
  types      [BodyParserContribution, ManifestV2, KindRegistryEntry]
  consumes   [extension_manifests_loaded]
  produces   [body_parser_contribution_registered]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
    kind_registry_available "KindRegistry is populated so entity_kinds references can be validated"
  }

  ensures {
    body_parsers_registered "Valid body parser contributions stored for Phase 1.5 dispatch"
    body_parser_event_emitted "body_parser_contribution_registered event emitted for each successful registration"
    one_parser_per_kind_enforced "At most one body parser per entity kind — duplicates produce errors"
  }

  contract """
    For each body_parser_contributions entry in a loaded extension manifest,
    the compiler MUST validate that the declared entity_kinds reference
    registered kinds in the KindRegistry. The export_name MUST be verified
    to exist in the extension's .wasm binary. At most one body parser
    per entity kind is allowed — duplicates MUST produce an error. The
    output_schema (if declared) MUST be stored for Phase 1.5 output
    validation.
  """

  verify unit "body parser contribution registered for valid entity kind"
  verify unit "body parser for unregistered kind produces warning"
  verify unit "duplicate body parser for same kind produces error"
  verify unit "export_name verified against Wasm binary"
  verify contract "requires/ensures consistency for body parser contribution registration"

}

// -- Extension Manifest Consistency ------------------------------------------

behavior validate_extension_manifest_consistency "Validate Extension Manifest Consistency" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   validation
  types      [ManifestV2, ManifestEntityKind, ManifestEdgeType, ManifestField]

  requires {
    manifest_parsed "Extension manifest has been parsed and its entity kinds, fields, and edge types are accessible"
    peer_dependencies_known "Peer dependency manifests are available for cross-manifest reference validation"
  }

  ensures {
    self_consistency_validated "All internal target_kind and edge_type references checked for self-consistency"
    authoring_errors_diagnosed "Self-contradictory references produce E-level errors; undeclared cross-extension references produce W-level warnings"
  }

  contract """
    When an extension is loaded, the compiler MUST validate that the manifest
    is self-consistent: all target_kind references in fields MUST reference
    entity kinds declared in the same manifest or in a peer dependency.
    All edge labels in field-to-edge mappings MUST have corresponding
    edgeType declarations. This validation is domain-agnostic — it checks
    structural consistency of the manifest without knowledge of what the
    entity kinds or edge types represent.

    Self-contradictory references within the same manifest (target_kind or
    edge_type that references a name not declared in the manifest itself
    and not in any peer dependency) MUST produce an E-level error, not a
    warning. These are authoring errors in the extension manifest that
    indicate broken internal contracts. Cross-extension references to kinds
    from non-peer extensions produce W-level warnings (the kind may exist
    but the dependency is undeclared).
  """

  verify unit "target_kind referencing own manifest kind passes"
  verify unit "target_kind referencing peer dependency kind passes"
  verify unit "self-contradictory target_kind produces E-level error"
  verify unit "target_kind referencing non-peer extension kind produces W-level warning"
  verify unit "self-contradictory edge label produces E-level error"
  verify contract "requires/ensures consistency for manifest self-consistency validation"

}
