// Zero-entity core — declarative validation engine and field validation

use invariants/zero-entity-core
use invariants/core
use types/zero-entity-core
use types/core
use types/config
use types/diagnostics
use types/errors
use types/wasm
use ports/outbound
use events/compilation

// -- Declarative Validation --------------------------------------------------

// No consumes — synchronous helper called by register_validation_rules_from_manifest
behavior parse_validation_rule_pattern "Parse Validation Rule Pattern" {
  invariants [zero_domain_knowledge_core, declarative_validation_determinism]
  types      [ValidationRulePattern, ValidationPatternKind]

  requires {
    manifest_rules_available "Extension manifest's validationRules array is accessible as structured data"
  }

  ensures {
    patterns_parsed "Each validationRules entry parsed into a well-formed ValidationRulePattern"
    unrecognized_warned "Unrecognized pattern kinds produce warning diagnostics with extension name"
  }

  contract """
    When the compiler reads a extension manifest's validationRules array,
    it MUST parse each entry into a ValidationRulePattern. The check
    field MUST be one of the recognized pattern kinds: no_incoming_edges,
    no_outgoing_edges, missing_field_when_flag_set, field_value_constraint,
    cycle_detection, file_exists. Unrecognized pattern kinds MUST produce
    a warning diagnostic with the extension name and invalid kind.
  """

  verify unit "parses no_incoming_edges pattern from manifest"
  verify unit "parses missing_field_when_flag_set pattern from manifest"
  verify unit "unrecognized pattern kind produces warning"
  verify unit "all required fields validated on each rule"
  verify contract "requires/ensures consistency for validation rule parsing"

}

behavior execute_validation_pattern "Execute Validation Pattern" {
  invariants [zero_domain_knowledge_core, declarative_validation_determinism]
  types      [ValidationRulePattern, ValidationPatternKind, Diagnostic]
  ports      [WasmRuntime]
  consumes   [graph_built]
  produces   [declarative_validation_executed]

  requires {
    patterns_parsed     "ValidationRulePattern is parsed and well-formed"
    graph_available     "Compiled graph is available for querying"
  }

  ensures {
    all_entities_matched "Pattern matched against all applicable entities in the graph"
    violations_diagnosed "Diagnostics emitted for every pattern violation"
  }

  maintains {
    deterministic_order "Patterns executed in code-sorted order producing identical diagnostics across runs"
  }

  contract """
    The declarative validation engine MUST execute each registered pattern
    against the compiled graph. no_incoming_edges MUST check that every
    entity of the target kind has at least one incoming edge. no_outgoing_edges
    MUST check outgoing edges. missing_field_when_flag_set MUST check that
    entities whose kind has the specified flag set to true have the specified field. field_value_constraint MUST
    check that a named field on entities of the target kind satisfies a
    value predicate (non-empty, matches regex, or is one of an allowed set).
    cycle_detection MUST check for cycles among the specified edge type.
    file_exists MUST check that file-reference fields point to existing
    files. custom MUST dispatch to the Wasm function registered by
    register_custom_validation_patterns. Each pattern violation MUST
    produce a diagnostic with the configured code and severity.
  """

  verify unit "no_incoming_edges detects orphan entities"
  verify unit "no_outgoing_edges detects entities with zero outgoing edges"
  verify unit "missing_field_when_flag_set detects missing specified field on flagged entity"
  verify unit "field_value_constraint rejects invalid field value"
  verify unit "cycle_detection finds cycles in edge type"
  verify unit "file_exists reports missing file-reference field targets"
  verify unit "custom pattern dispatches to registered Wasm function"
  verify unit "pattern violation produces diagnostic with configured code and severity"
  verify contract "requires/ensures consistency for declarative validation"

}

behavior emit_diagnostic_from_pattern "Emit Diagnostic From Pattern" {
  invariants [zero_domain_knowledge_core, declarative_validation_determinism]
  types      [ValidationRulePattern, Diagnostic]

  requires {
    violation_detected "A declarative validation pattern has detected a violation for an entity"
    pattern_configured "The pattern's messageTemplate, code, and severity fields are available"
  }

  ensures {
    diagnostic_emitted "Diagnostic emitted with interpolated message, configured code, and correct severity"
    template_interpolated "All interpolation variables ({id}, {kind}, {field}, {value}) resolved in messageTemplate"
  }

  contract """
    When a declarative validation pattern detects a violation, the engine
    MUST emit a diagnostic using the pattern's messageTemplate with
    interpolation variables: {id} for the entity ID, {kind} for the
    entity kind, {field} for the field name, {value} for the field value.
    The diagnostic code MUST be the pattern's code field. The severity
    MUST match the pattern's severity field (error, warning, info).
  """

  verify unit "message template interpolates {id} and {kind}"
  verify unit "message template interpolates {field} and {value}"
  verify unit "diagnostic code matches pattern code"
  verify unit "diagnostic severity matches pattern severity"
  verify contract "requires/ensures consistency for pattern diagnostic emission"

}

behavior register_extension_validation_rules "Register Extension Validation Rules" {
  invariants [zero_domain_knowledge_core, declarative_validation_determinism, registry_population_before_validation]
  types      [ValidationRulePattern, ManifestV2]
  consumes   [extension_manifests_loaded]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
    individual_rules_parsed "Per-extension validation rules already parsed by register_validation_rules_from_manifest"
  }

  ensures {
    unified_rule_set_produced "Single validation rule set aggregated from all installed extensions"
    deterministic_order_enforced "Rules sorted by code for deterministic execution order"
    duplicate_codes_warned "Duplicate diagnostic codes across extensions produce warnings"
  }

  contract """
    During extension loading, the compiler MUST collect all validationRules
    from all installed extensions into a single validation rule set. This
    behavior operates at the cross-extension level — it aggregates rules
    already parsed by register_validation_rules_from_manifest into the
    final rule set used by execute_validation_pattern. Duplicate diagnostic
    codes across extensions MUST produce a warning listing both extensions.
    Rules MUST be sorted by code for deterministic execution order.
    Collection MUST complete before any declarative validation begins.
  """

  verify unit "rules from multiple extensions are collected"
  verify unit "duplicate codes across extensions produce warning"
  verify unit "rules sorted by code for deterministic order"
  verify contract "requires/ensures consistency for cross-extension rule aggregation"

}

behavior register_custom_validation_patterns "Register Custom Validation Patterns" {
  invariants [zero_domain_knowledge_core, declarative_validation_determinism]
  types      [ValidationRulePattern, CustomValidationPattern, ManifestV2]
  refs       [provide_host_function_query_graph]
  ports      [WasmRuntime]
  consumes   [extension_manifests_loaded]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
    wasm_runtime_available "WasmRuntime port is available for resolving and dispatching Wasm exports"
  }

  ensures {
    custom_patterns_registered "Custom validation patterns registered alongside declarative patterns"
    wasm_functions_resolved "wasm_function fields resolved to Wasm exports or warnings emitted for unresolvable names"
  }

  contract """
    When an extension declares validation rules with check kind "custom",
    the compiler MUST resolve the wasm_function field to a Wasm export in
    the extension's module. The custom pattern MUST be registered alongside
    declarative patterns. During validation, custom patterns MUST be
    dispatched to the Wasm runtime via the extension's exported function.
    The Wasm function receives the entity ID and returns a boolean (pass/fail).
    The function body MAY call the `specforge.query_graph` host function
    (see provide_host_function_query_graph) to access the compiled graph
    for cross-entity semantic checks and custom graph traversals.
    On failure, the engine MUST emit a diagnostic using the pattern's
    configured code, severity, and message template. Unresolvable
    wasm_function names MUST produce a warning at registration time.
  """

  verify unit "custom pattern registered with wasm_function reference"
  verify unit "unresolvable wasm_function produces warning"
  verify unit "custom pattern dispatched to Wasm runtime during validation"
  verify unit "custom pattern failure emits configured diagnostic"
  verify contract "requires/ensures consistency for custom validation pattern registration"

}

// -- Field Validation --------------------------------------------------------

behavior detect_unknown_entity_fields "Detect Unknown Entity Fields" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  types      [FieldRegistryEntry, KindRegistryEntry, Diagnostic]
  consumes   [registries_populated, define_blocks_registered]

  requires {
    registries_populated_fired "registries_populated event has fired, confirming FieldRegistry and KindRegistry are fully populated"
    define_blocks_registered_fired "define_blocks_registered event has fired, confirming project-defined kinds are registered"
  }

  ensures {
    unknown_fields_diagnosed "W020 warning emitted for every unrecognized field name on registered entity kinds"
    cascading_avoided "Field validation skipped for entities with unregistered kinds (already E024)"
  }

  contract """
    During Phase 2 semantic validation, the compiler MUST scan all parsed
    entity blocks whose kind is registered in the KindRegistry and check
    each field name against the FieldRegistry for that kind. Every field
    name not present in the FieldRegistry for the entity's kind MUST
    produce a W020 warning diagnostic with the entity's source span, the
    unrecognized field name, and the entity kind name. Structural fields
    parsed by dedicated grammar rules (title, verify) MUST NOT
    be checked against the FieldRegistry — they are grammar-level
    constructs, not extension-defined fields. When the entity's kind
    itself is unregistered (already reported as E024), field validation
    MUST be skipped for that entity to avoid cascading diagnostics.
  """

  verify unit "unregistered field name produces W020"
  verify unit "W020 includes field name, entity kind, and source span"
  verify unit "registered field name does not produce W020"
  verify unit "structural fields (title, verify) not checked against FieldRegistry"
  verify unit "field validation skipped when entity kind is unregistered"
  verify contract "requires/ensures consistency for unknown field detection"

}

// Registry-level collision detection during manifest loading. Called by
// detect_entity_kind_collision (behaviors/wasm-extensions.spec) as part of its
// orchestration — focuses exclusively on inter-extension kind collisions (E026).
behavior detect_duplicate_entity_kinds "Detect Duplicate Entity Kinds" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  types      [ManifestV2, ManifestEntityKind, KindRegistryEntry, Diagnostic]

  requires {
    manifests_loading "Extension manifests are being loaded and entity kinds are being registered into KindRegistry"
  }

  ensures {
    collisions_detected "E026 diagnostic emitted when two extensions register the same entity kind keyword"
    first_wins_enforced "First extension in topological order owns the kind on collision"
  }

  contract """
    When two extensions register the same entity kind keyword, the compiler
    MUST detect the collision during registry population. The first extension
    in topological order MUST own the kind. The second registration MUST
    produce an E026 diagnostic naming both extensions. This collision
    detection is distinct from detect_entity_kind_collision (behaviors/wasm-extensions.spec)
    which handles the user-facing resolution — this behavior handles the
    registry-level detection during manifest loading.
  """

  verify unit "duplicate kind from two extensions produces E026"
  verify unit "first extension in topological order owns the kind"
  verify unit "single extension registering a kind produces no diagnostic"
  verify contract "requires/ensures consistency for duplicate entity kind detection"

}

behavior validate_peer_dependencies "Validate Peer Dependencies" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  types      [ManifestV2, PeerDependency, ExtensionError]
  produces   [extension_loading_failed]

  requires {
    manifests_available "All declared extension manifests have been loaded and their peer_dependencies fields are accessible"
  }

  ensures {
    dependencies_validated "Every peer dependency checked against installed extensions for semver compatibility"
    unsatisfied_blocked "Unsatisfied peer dependencies produce hard error diagnostics and prevent registry population"
    loading_failed_emitted "extension_loading_failed event emitted for extensions with unmet dependencies"
  }

  contract """
    During extension loading, the compiler MUST validate that every
    peer dependency declared in an extension's manifest is satisfied by
    an installed extension at a compatible semver version. Unsatisfied
    peer dependencies MUST produce a hard error diagnostic naming the
    missing extension and required version range. This check MUST occur
    before registry population to prevent partial registration from
    extensions with unmet dependencies.
  """

  verify unit "satisfied peer dependency passes validation"
  verify unit "missing peer dependency produces hard error"
  verify unit "incompatible version produces hard error with required range"
  verify contract "requires/ensures consistency for peer dependency validation"

}

// Moved from behaviors/validation.spec — belongs with zero-entity core validation
behavior validate_extension_testability "Validate Extension Testability" {
  invariants [testable_entity_classification, zero_domain_knowledge_core]
  types      [Diagnostic, KindRegistryEntry]
  consumes  [registries_populated]

  requires {
    registries_populated_fired "registries_populated event has fired, confirming all entity kinds are registered with their flags"
  }

  ensures {
    flag_consistency_checked "Every KindRegistryEntry's testable and supportsVerify flags checked for consistency"
    advisory_diagnostics_emitted "W017 or I006 diagnostics emitted for inconsistent flag combinations"
  }

  contract """
    This behavior checks boolean flag consistency generically across all
    extension-declared entity kinds. The validator MUST detect inconsistencies
    between an extension manifest's testable and supportsVerify flags
    for each entity kind.

    An entity kind marked testable=true MUST have supportsVerify=true.
    If not, the validator MUST produce a W017 warning — testability
    requires a mechanism for declaring test intent.

    An entity kind with supportsVerify=true but testable=false MUST produce
    an I006 info diagnostic (verify statements accepted but entity does not
    count toward coverage).

    These checks compare boolean flags from the same KindRegistry entry —
    the core does not interpret what "testable" means semantically, it only
    checks that the flags are not contradictory. This is a post-registration
    manifest lint pass, not a domain-semantic check. It runs after
    register_entity_kinds_from_manifest completes (during the
    registries_populated → validation_complete window). The W017 and I006
    diagnostics are advisory — they do not block compilation.
  """

  verify unit "testable kind without supportsVerify produces W017"
  verify unit "testable kind with supportsVerify=true passes"
  verify unit "kind with supportsVerify but not testable produces I006"
  verify unit "consistent testable and supportsVerify flags produce no diagnostic"
  verify contract "requires/ensures consistency for extension testability validation"

}
