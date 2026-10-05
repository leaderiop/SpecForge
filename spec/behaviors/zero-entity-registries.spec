// Zero-entity core — manifest V2, dynamic registries, grammar, bootstrap, visualization, consistency

use "events/compilation"
use "invariants/core"
use "invariants/zero-entity-core"
use "ports/outbound"
use "types/config"
use "types/core"
use "types/diagnostics"
use "types/errors"
use "types/wasm"
use "types/zero-entity-core"

// -- Extension Manifest V2 ---------------------------------------------------

behavior validate_manifest_v2_schema "Validate Manifest V2 Schema" {
  features   [extension_manifest]
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   validation
  types      [ManifestV2, ExtensionError]
  requires {
    manifest_json_available "Manifest JSON has been parsed from disk and is available as a structured object"
  }
  ensures {
    schema_validated    "Manifest passes all v2 schema checks — required fields present, manifest_version is 2, contributions structurally correct"
    malformed_diagnosed "Malformed JSON or missing required fields produce hard error diagnostics"
  }
  contract   """
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
  verify contract "Validate Manifest V2 Schema: manifest v2 schema validation holds — manifest_json_available, schema_validated, malformed_diagnosed"
}

behavior register_entity_kinds_from_manifest "Register Entity Kinds From Manifest" {
  features   [dynamic_entity_registration]
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   command
  types      [ManifestV2, ManifestEntityKind, KindRegistryEntry]
  consumes   [extension_manifests_loaded]
  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
  }
  ensures {
    kinds_registered          "Every entityKinds entry from the manifest is registered in the KindRegistry with full metadata"
    source_extension_recorded "Source extension name recorded on each KindRegistryEntry for diagnostics"
  }
  contract   """
    For each entityKinds entry in a extension manifest, the compiler MUST
    register the kind in the KindRegistry with full metadata: testable flag,
    singleton flag, supportsVerify flag, semantic token classification,
    LSP icon for outline, and DOT shape for
    visualization. The source extension name MUST be recorded for diagnostics
    and doctor output. A kind's lifecycle_field MUST be recorded when it
    names one of the kind's own or the extension's shared fields, and
    refused with a warning (W021) otherwise.
  """
  verify unit "entity kind registered with testable flag"
  verify unit "entity kind registered with singleton flag"
  verify unit "entity kind registered with LSP metadata"
  verify unit "source extension recorded in registry entry"
  // Testability registration (formerly register_testability_from_manifest)
  verify unit "testable=true entity participates in coverage"
  verify unit "testable=false entity excluded from coverage"
  verify unit "no default testability assumed by core"
  verify unit "a kind's lifecycle_field must name a field it declares"
  verify contract "Register Entity Kinds From Manifest: entity kind registration holds — extension_manifests_loaded_fired, kinds_registered, source_extension_recorded"
}

behavior register_edge_types_from_manifest "Register Edge Types From Manifest" {
  features   [dynamic_entity_registration]
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   command
  types      [ManifestV2, ManifestEdgeType]
  consumes   [extension_manifests_loaded]
  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
  }
  ensures {
    edge_types_registered "Every edgeTypes entry and field-to-edge mapping is registered in the edge type set"
    constraints_recorded  "Source and target kind constraints recorded for each edge type"
    duplicates_warned     "Duplicate edge labels across extensions produce W-level warnings"
  }
  contract   """
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
  verify contract "Register Edge Types From Manifest: edge type registration holds — extension_manifests_loaded_fired, edge_types_registered, constraints_recorded, duplicates_warned"
}

behavior register_validation_rules_from_manifest "Register Validation Rules From Manifest" {
  features   [declarative_validation_rules]
  invariants [
    zero_domain_knowledge_core,
    registry_population_before_validation,
    declarative_validation_determinism,
  ]
  category   command
  types      [ManifestV2, ValidationRulePattern]
  consumes   [extension_manifests_loaded]
  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
  }
  ensures {
    rules_registered       "Every validationRules entry is parsed and stored with raw target_kind and edge_type strings"
    unloaded_targets_inert "A rule whose target kind or edge type no loaded extension declares reports nothing"
  }
  contract   """
    For each validationRules entry in a extension manifest, the compiler
    MUST parse and register the declarative rule. Registration is a two-step
    process:

    Step 1 (during registration): Rules are parsed and stored with their
    raw target_kind and edge_type strings. No cross-reference validation
    occurs at this point because peer-dependency extensions may not have
    registered their kinds yet.

    Step 2 (after registries_populated): target_kind and edge_type are
    resolved against the loaded registries when rules run. A rule whose
    target kind or edge type no loaded extension declares reports nothing:
    it belongs to an optional peer that is not installed, so a project
    without that kind is not held to it. A manifest that references a kind
    no declared peer provides is the extension author's mistake, reported
    as W021 when the extensions load.
  """
  verify unit "validation rule registered from manifest"
  verify unit "target_kind validation deferred to post-registration phase"
  verify unit "a rule targeting a kind no loaded extension declares reports nothing"
  verify contract "Register Validation Rules From Manifest: validation rule registration holds — extension_manifests_loaded_fired, rules_registered, unloaded_targets_inert"
}

behavior register_verify_kinds_from_manifest "Register Verify Kinds From Manifest" {
  features   [extension_manifest]
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   validation
  types      [ManifestV2, ManifestEntityKind]
  consumes   [extension_manifests_loaded]
  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
  }
  ensures {
    verify_kinds_registered "All allowedVerifyKinds from manifest entityKinds are registered per entity kind"
    no_hardcoded_kinds      "No built-in verify kind names exist — all come from extension manifests"
  }
  contract   """
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
  verify contract "Register Verify Kinds From Manifest: verify kind registration holds — extension_manifests_loaded_fired, verify_kinds_registered, no_hardcoded_kinds"
}

// -- Dynamic Entity Registration ---------------------------------------------

behavior boot_empty_kind_registry "Boot Empty Kind Registry" {
  features   [dynamic_entity_registration]
  invariants [zero_domain_knowledge_core]
  category   command
  types      [KindRegistryEntry]
  requires {
    compiler_initializing "Compiler initialization has started and memory is allocated"
  }
  ensures {
    kind_registry_empty       "KindRegistry contains zero entity kind entries"
    structural_keywords_ready "Parser recognizes only structural keywords (spec, ref, use, define)"
  }
  contract   """
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
  verify contract "Boot Empty Kind Registry: empty kind registry boot holds — compiler_initializing, kind_registry_empty, structural_keywords_ready"
}

behavior boot_empty_field_registry "Boot Empty Field Registry" {
  features   [dynamic_entity_registration]
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
  contract   """
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
  verify contract "Boot Empty Field Registry: empty field registry boot holds — compiler_initializing, field_registry_empty, no_fields_recognized"
}

behavior boot_empty_edge_registry "Boot Empty Edge Registry" {
  features   [dynamic_entity_registration]
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
  contract   """
    When the compiler initializes, the edge type set MUST start empty with
    zero registered edge labels. No extension-defined edge types MUST exist
    until extensions populate the set. This parallels boot_empty_kind_registry
    and boot_empty_field_registry — all three registries begin empty and are
    populated exclusively from extension manifests.
  """
  verify unit "edge type set starts with zero entries"
  verify unit "no edge labels recognized before extension loading"
  verify contract "Boot Empty Edge Registry: empty edge registry boot holds — compiler_initializing, edge_registry_empty, no_edges_recognized"
}

behavior report_define_blocks "Report Unsupported Define Blocks" {
  features   [zero_entity_bootstrap]
  invariants [zero_domain_knowledge_core]
  category   validation
  types      [SpecFile, Diagnostic]
  ensures {
    define_warned     "Each define block produces one W143 warning naming it"
    no_entity_created "A define block adds no entity to the graph, so its body is not read as references and its name is not checked as an ID"
  }
  contract   """
    Custom entity kinds come from extensions, not from .spec files: a
    project's kinds are a function of specforge.json and its loaded
    extensions only (ADR 0005). The grammar still parses
    define <name> { fields }, and define stays a reserved word, but the
    compiler MUST report each define block with one W143 warning that
    suggests writing an extension, and MUST NOT add it to the graph. An
    incremental rebuild MUST report a define block as a fresh compile
    does.
  """
  verify unit "a define block produces one W143 and adds no node to the graph"
  verify integration "an incremental rebuild reports a define block as a fresh compile does"
}

behavior populate_kind_registry_from_extensions "Populate Kind Registry From Extensions" {
  features   [dynamic_entity_registration]
  invariants [
    zero_domain_knowledge_core,
    registry_population_before_validation,
    compilation_pipeline_ordering,
  ]
  category   command
  types      [ManifestV2, ManifestEntityKind, KindRegistryEntry]
  produces   [registries_populated]
  requires {
    manifests_loaded    "All extension manifests MUST be loaded and their entity_kinds arrays accessible"
    kind_registry_empty "KindRegistry MUST be in its initial empty state (boot_empty_kind_registry completed)"
  }
  ensures {
    one_entry_per_kind     "KindRegistry contains exactly one entry per unique entity kind declared across all installed extension manifests"
    registries_event_fired "registries_populated event emitted after all three registries are populated"
  }
  maintains {
    zero_domain_knowledge "No domain-specific logic introduced during population — only structural metadata registered"
  }
  contract   """
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
  verify contract "Populate Kind Registry From Extensions: registry population holds for the declared obligations"
}

behavior populate_field_registry_from_extensions "Populate Field Registry From Extensions" {
  features   [dynamic_entity_registration]
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   command
  types      [ManifestV2, ManifestField, FieldRegistryEntry, ManifestFieldType]
  consumes   [extension_manifests_loaded]
  // Orchestrated by populate_kind_registry_from_extensions which fires registries_populated after all three complete.
  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
    kind_registry_populated          "KindRegistry is already populated so field registrations can reference valid entity kinds"
  }
  ensures {
    fields_registered     "Every ManifestEntityKind's fields array is registered in the FieldRegistry"
    field_types_validated "All field type declarations validated against ManifestFieldType variants"
    fields_populated      "FieldRegistry is fully populated and ready for downstream consumers"
  }
  contract   """
    After populating the KindRegistry, the compiler MUST populate the
    FieldRegistry from all extension manifests. Each ManifestEntityKind's
    fields array MUST be registered as valid field definitions for that
    entity kind. Field types (string, string[], reference, reference[],
    block) MUST be validated against ManifestFieldType variants. Invalid
    field types MUST produce a warning. Note: verify is NOT a field type
    — it is a grammar-level construct governed by the supports_verify
    flag on ManifestEntityKind. A field's normative flag MUST be kept in
    its registry entry, so exports can tell the text that states what an
    entity promises from prose without core knowing any field's name.
    Likewise a field's proof_role (bound or claim) MUST reach its registry
    entry, so the prove pass reads declared roles and no field name; any
    other role value MUST be refused with a warning (W021).
  """
  verify unit "fields registered per entity kind"
  verify unit "a field's normative flag reaches its registry entry"
  verify unit "a field's proof_role reaches the field registry"
  verify unit "a proof_role other than bound or claim is refused"
  verify unit "field types validated against known types"
  verify unit "invalid field type produces warning"
  verify contract "Populate Field Registry From Extensions: field registry population holds — extension_manifests_loaded_fired, kind_registry_populated, fields_registered, field_types_validated, fields_populated"
}

behavior populate_edge_registry_from_extensions "Populate Edge Registry From Extensions" {
  features   [dynamic_entity_registration]
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
    edges_populated   "EdgeRegistry is fully populated and ready for downstream consumers"
  }
  contract   """
    The compiler MUST build the complete edge type set from two sources:
    explicit edgeTypes declarations in manifests, and implicit edge types
    derived from ManifestField entries with an edge property. Both sources
    MUST be merged into a single edge type set before graph construction.
    Duplicate edge labels MUST be warned about but not rejected.
  """
  verify unit "explicit edgeTypes merged into edge set"
  verify unit "implicit edges from field mappings merged"
  verify unit "duplicate edge labels produce warning"
  verify contract "Populate Edge Registry From Extensions: edge registry population holds — extension_manifests_loaded_fired, edge_set_complete, duplicates_warned, edges_populated"
}

behavior validate_registered_entity_fields "Validate Registered Entity Fields" {
  features   [dynamic_entity_registration]
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   validation
  types      [
    ManifestV2,
    ManifestField,
    ManifestEntityKind,
    ManifestEdgeType,
    FieldRegistryEntry,
    KindRegistryEntry,
  ]
  consumes   [extension_manifests_loaded]
  requires {
    extensions_loaded                "Every configured extension's manifest is loaded, so each one's kinds and edge types are known"
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired"
  }
  ensures {
    target_kinds_resolved "Every field target_kind reference resolves to a registered kind"
    unresolved_diagnosed  "Diagnostics emitted for unresolved target_kind references"
  }
  maintains {
    no_domain_logic "Cross-validation uses only structural checks — no domain-specific logic"
  }
  contract   """
    Once every configured extension is loaded, before the registries are
    built from them, the compiler MUST cross-validate what they will
    register using only structural checks — no domain-specific logic.
    target_kind references in fields MUST resolve to entity kinds the
    extension or a loaded peer declares. Edge labels in field-to-edge
    mappings MUST resolve to edge types the extension declares. Field type
    declarations MUST be internally consistent. Validation failures MUST
    produce warnings (W021, from validate_extension_manifest_consistency)
    to allow partial loading, not hard errors.
  """
  verify unit "target_kind reference resolves to registered kind"
  verify unit "edge label resolves to registered edge type"
  verify unit "unresolved target_kind produces warning"
  verify unit "unresolved edge label produces warning"
  verify unit "cross-validation uses no domain-specific logic"
  verify contract "Validate Registered Entity Fields: field cross-validation holds for the declared obligations"
}

// -- Grammar Consolidation ---------------------------------------------------

behavior collapse_grammar_to_generic_entity_block "Collapse Grammar to Generic Entity Block" {
  features   [zero_entity_bootstrap, spec_file_parsing]
  invariants [zero_domain_knowledge_core]
  category   command
  types      [SpecFile, Entity]
  requires {
    grammar_source_available "Tree-sitter grammar source is available for compilation"
  }
  ensures {
    single_generic_rule              "Grammar contains exactly one generic entity_block rule for all extension keywords"
    structural_rules_preserved       "spec_block, ref_block, use_import, and define_block remain as separate grammar rules"
    no_keyword_validation_in_grammar "All keyword validation deferred to semantic phase"
  }
  contract   """
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
  verify contract "Collapse Grammar to Generic Entity Block: grammar collapse holds — grammar_source_available, single_generic_rule, structural_rules_preserved, no_keyword_validation_in_grammar"
}

// -- Zero-Entity Bootstrap ---------------------------------------------------

behavior two_phase_parse_structural "Two-Phase Parse: Structural" {
  features   [zero_entity_bootstrap]
  invariants [
    registry_population_before_validation,
    zero_domain_knowledge_core,
    compilation_pipeline_ordering,
  ]
  category   command
  types      [SpecFile, Entity]
  produces   [structural_parse_complete]
  requires {
    spec_files_available "All .spec files discovered and readable from the configured spec_root"
  }
  ensures {
    structural_parse_produced      "SpecFile ASTs produced with generic entity blocks for every keyword name { } block"
    structural_parse_event_emitted "structural_parse_complete event emitted after all files are parsed"
    no_keyword_validation          "No keyword validation performed — all keywords accepted structurally"
  }
  contract   """
    In Phase 1 of the two-phase compilation, the parser MUST perform
    purely structural parsing of all .spec files. Every keyword name { }
    block MUST be parsed into a generic entity node regardless of whether
    the keyword is registered. No keyword validation MUST occur in Phase 1.
    The output MUST be a list of SpecFile ASTs with generic entity blocks.
  """
  verify unit "unknown keyword parsed into generic entity node"
  verify unit "no keyword validation in Phase 1"
  verify unit "all .spec files parsed before Phase 2"
  verify unit "parse errors collected without aborting"
  verify contract "Two-Phase Parse: Structural: structural parsing holds — spec_files_available, structural_parse_produced, structural_parse_event_emitted, no_keyword_validation"
}

behavior two_phase_validate_semantic "Two-Phase Validate: Semantic" {
  features   [zero_entity_validation, zero_entity_bootstrap]
  invariants [
    registry_population_before_validation,
    zero_domain_knowledge_core,
    compilation_pipeline_ordering,
  ]
  category   validation
  types      [KindRegistryEntry, FieldRegistryEntry]
  consumes   [registries_populated]
  requires {
    registries_populated "registries_populated event MUST have fired, confirming KindRegistry, FieldRegistry, and edge type set are fully populated"
  }
  ensures {
    all_blocks_checked         "Every parsed entity block checked against the KindRegistry"
    unknown_keywords_diagnosed "Unknown keywords have E024 diagnostics emitted"
    fields_validated           "Known keywords have their fields validated against the FieldRegistry"
  }
  contract   """
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
  verify contract "Two-Phase Validate: Semantic: semantic validation holds — unknown_keywords_diagnosed"
}

// The KeywordExtensionIndex is a static JSON file (data/keyword-index.json)
// mapping known entity keywords to their providing extension names, shipped
// as a bundled data file. It does NOT require network access at runtime. It
// is maintained by hand; a test checks it against every builtin extension's
// declared entity kinds.
// suggest_missing_extensions is invoked inline by the diagnostic pipeline when
// an E024 (unknown entity kind) is emitted — it is not event-driven. It enriches
// the diagnostic help text with extension suggestions from the bundled index.
behavior suggest_missing_extensions "Suggest Missing Extensions" {
  features   [zero_entity_validation, zero_entity_bootstrap]
  invariants [zero_domain_knowledge_core]
  category   command
  types      [KindRegistryEntry, UnknownKindError, KeywordExtensionIndex, KeywordExtensionMapping]
  requires {
    e024_diagnostic_emitted "An E024 (unknown entity kind) diagnostic has been emitted and requires help text enrichment"
  }
  ensures {
    suggestion_provided   "Help text includes extension suggestion from bundled index or fallback to specforge search"
    lazy_loading_enforced "KeywordExtensionIndex loaded lazily on first E024, not at startup"
  }
  contract   """
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
  verify contract "Suggest Missing Extensions: missing extension suggestions holds — e024_diagnostic_emitted, suggestion_provided, lazy_loading_enforced"
}

// detect_unknown_entity_kinds and suggest_missing_extensions live in the
// registries file because they query the KindRegistry directly to determine
// whether a keyword is registered. They are validation behaviors that depend
// on registry state rather than on the validation rule engine.
behavior detect_unknown_entity_kinds "Detect Unknown Entity Kinds" {
  features   [zero_entity_validation, zero_entity_bootstrap]
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   validation
  types      [KindRegistryEntry, UnknownKindError]
  consumes   [registries_populated]
  requires {
    registries_populated_fired "registries_populated event has fired, confirming KindRegistry is fully populated"
    structural_parse_ready     "All .spec files have been structurally parsed into generic entity blocks"
  }
  ensures {
    unknown_kinds_diagnosed   "E024 diagnostic emitted for every keyword not present in the KindRegistry"
    registered_kinds_accepted "Registered keywords pass without E024 diagnostics"
  }
  contract   """
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
  verify contract "Detect Unknown Entity Kinds: unknown entity kind detection holds — registries_populated_fired, structural_parse_ready, unknown_kinds_diagnosed, registered_kinds_accepted"
}

behavior graceful_degradation_without_extensions "Graceful Degradation Without Extensions" {
  features   [zero_entity_bootstrap]
  invariants [zero_domain_knowledge_core]
  category   command
  types      [SpecFile, KindRegistryEntry]
  consumes   [registries_populated]
  requires {
    registries_populated_fired "registries_populated event has fired (with empty registries when no extensions are installed)"
  }
  ensures {
    i002_emitted                "I002 info diagnostic emitted indicating no extensions are configured"
    structural_mode_operational "Compiler operates in structural-only mode with generic entity nodes"
    valid_export_produced       "specforge export produces valid JSON from structural-only graph"
  }
  contract   """
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
  verify contract "Graceful Degradation Without Extensions: graceful degradation holds — registries_populated_fired, i002_emitted, structural_mode_operational, valid_export_produced"
}

behavior handle_all_extensions_failed_to_load "Handle All Extensions Failed to Load" {
  features   [zero_entity_bootstrap]
  invariants [zero_domain_knowledge_core, multi_error_collection]
  category   command
  types      [ExtensionError, Diagnostic]
  consumes   [extension_manifests_loaded]
  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming load attempts completed (even if all failed)"
  }
  ensures {
    per_extension_errors_emitted "E-level diagnostic emitted for each failed extension"
    structural_mode_fallback     "System transitions to structural-only mode after all failures"
    no_crash_guaranteed          "System does not crash or enter an undefined state"
  }
  contract   """
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
  verify contract "Handle All Extensions Failed to Load: all-extensions-failed handling holds — extension_manifests_loaded_fired, per_extension_errors_emitted, structural_mode_fallback, no_crash_guaranteed"
}

// -- Extension-Driven Visualization ------------------------------------------
// These visualization behaviors are synchronous delegates called by
// serialize_dot_visualization (behaviors/output.spec) — they do not consume
// events or own ports directly.

behavior render_extension_defined_dot_shapes "Render Extension-Defined DOT Shapes" {
  features   [extension_driven_visualization]
  invariants [zero_domain_knowledge_core]
  category   query
  types      [KindRegistryEntry]
  requires {
    kind_registry_available "KindRegistry is populated and queryable for dot_shape, dot_color, dot_fillcolor metadata"
    graph_available         "Compiled graph with entity nodes is available for rendering"
  }
  ensures {
    shapes_applied "Every entity node rendered with its extension-defined DOT shape or default 'box'"
    colors_applied "dot_color and dot_fillcolor attributes set on nodes when specified in manifest"
  }
  contract   """
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
  verify contract "Render Extension-Defined DOT Shapes: DOT shape rendering holds — kind_registry_available, graph_available, shapes_applied, colors_applied, be"
}

behavior render_extension_defined_edge_styles "Render Extension-Defined Edge Styles" {
  features   [extension_driven_visualization]
  invariants [zero_domain_knowledge_core]
  category   query
  types      [ManifestEdgeType, EdgeRegistryEntry]
  requires {
    edge_registry_available "Edge type registry is populated and queryable for style metadata"
    graph_available         "Compiled graph with typed edges is available for rendering"
  }
  ensures {
    styles_applied      "Every edge rendered with its extension-defined style or default 'solid'"
    edge_colors_applied "edge_color and edge_arrowhead attributes set on edges when specified in manifest"
  }
  contract   """
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
  verify contract "Render Extension-Defined Edge Styles: DOT edge style rendering holds — edge_registry_available, graph_available, styles_applied, edge_colors_applied"
}

// -- Extension Manifest Consistency ------------------------------------------

behavior validate_extension_manifest_consistency "Validate Extension Manifest Consistency" {
  features   [extension_manifest]
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   validation
  types      [ManifestV2, ManifestEntityKind, ManifestEdgeType, ManifestField]
  requires {
    manifest_parsed         "Extension manifest has been parsed and its entity kinds, fields, and edge types are accessible"
    peer_dependencies_known "Peer dependency manifests are available for cross-manifest reference validation"
  }
  ensures {
    self_consistency_validated "All internal target_kind and edge_type references checked for self-consistency"
    authoring_errors_diagnosed "Self-contradictory and undeclared cross-extension references produce W021 warnings"
  }
  contract   """
    When an extension is loaded, the compiler MUST validate that the manifest
    is self-consistent: all target_kind references in fields MUST reference
    entity kinds declared in the same manifest or in a peer dependency.
    All edge labels in field-to-edge mappings MUST have corresponding
    edgeType declarations. This validation is domain-agnostic — it checks
    structural consistency of the manifest without knowledge of what the
    entity kinds or edge types represent.

    Self-contradictory references within the same manifest (target_kind or
    edge_type that references a name not declared in the manifest itself
    and not in any peer dependency) MUST produce a W021 warning naming the
    reference. They are authoring errors in the extension, not in the
    user's spec, so they MUST NOT fail the user's compile. Cross-extension
    references to kinds from non-peer extensions produce W021 as well (the
    kind may exist but the dependency is undeclared).

    A field's `derived_from` MUST name `type_expressions` or
    `method_signatures` and sit on a reference or reference_list field with
    a target_kind; any other one derives nothing and produces a W021
    warning naming the field.
  """
  verify unit "target_kind referencing own manifest kind passes"
  verify unit "target_kind referencing peer dependency kind passes"
  verify unit "self-contradictory target_kind produces a W021 warning"
  verify unit "target_kind referencing non-peer extension kind produces W-level warning"
  verify unit "self-contradictory edge label produces a W021 warning"
  verify unit "a derived_from the host can't apply produces a W021 warning"
  verify contract "Validate Extension Manifest Consistency: manifest self-consistency validation holds — manifest_parsed, peer_dependencies_known, self_consistency_validated, authoring_errors_diagnosed"
}
