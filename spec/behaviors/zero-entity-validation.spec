// Zero-entity core — declarative validation engine and field validation

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

// -- Declarative Validation --------------------------------------------------

// No consumes — synchronous helper the registry build runs (registry_build_rules)
behavior parse_validation_rule_pattern "Parse Validation Rule Pattern" {
  features   [declarative_validation_rules]
  invariants [zero_domain_knowledge_core, declarative_validation_determinism]
  category   command
  types      [ValidationRulePattern, ValidationPatternKind, FieldConstraint]
  requires {
    manifest_rules_available "Extension manifest's validationRules array is accessible as structured data"
  }
  ensures {
    patterns_parsed     "Each validationRules entry parsed into a well-formed ValidationRulePattern"
    unrecognized_warned "Unrecognized pattern kinds produce warning diagnostics with extension name"
  }
  contract   """
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
  verify unit "parses field_value_constraint pattern from manifest"
  verify contract "Parse Validation Rule Pattern: validation rule parsing holds — manifest_rules_available, patterns_parsed, unrecognized_warned"
}

behavior execute_validation_pattern "Execute Validation Pattern" {
  features   [declarative_validation_rules]
  invariants [zero_domain_knowledge_core, declarative_validation_determinism]
  category   command
  types      [ValidationRulePattern, ValidationPatternKind, Diagnostic]
  ports      [WasmRuntime]
  consumes   [graph_built]
  produces   [declarative_validation_executed]
  requires {
    patterns_parsed "ValidationRulePattern is parsed and well-formed"
    graph_available "Compiled graph is available for querying"
  }
  ensures {
    all_entities_matched "Pattern matched against all applicable entities in the graph"
    violations_diagnosed "Diagnostics emitted for every pattern violation"
  }
  maintains {
    deterministic_order "Patterns executed in code-sorted order producing identical diagnostics across runs"
  }
  contract   """
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
  verify unit "an edge rule counts only edges of its edge type and is dropped when no extension declares the kind at its far end"
  verify unit "missing_field_when_flag_set detects missing specified field on flagged entity"
  verify unit "field_value_constraint rejects invalid field value"
  verify unit "cycle_detection finds cycles in edge type"
  verify unit "file_exists reports missing file-reference field targets"
  verify unit "custom pattern dispatches to registered Wasm function"
  verify unit "pattern violation produces diagnostic with configured code and severity"
  verify contract "Execute Validation Pattern: declarative validation holds — all_entities_matched, violations_diagnosed, deterministic_order"
}

behavior emit_diagnostic_from_pattern "Emit Diagnostic From Pattern" {
  features   [declarative_validation_rules]
  invariants [zero_domain_knowledge_core, declarative_validation_determinism]
  category   command
  types      [ValidationRulePattern, Diagnostic]
  requires {
    violation_detected "A declarative validation pattern has detected a violation for an entity"
    pattern_configured "The pattern's messageTemplate, code, and severity fields are available"
  }
  ensures {
    diagnostic_emitted    "Diagnostic emitted with interpolated message, configured code, and correct severity"
    template_interpolated "All interpolation variables ({id}, {kind}, {field}, {value}) resolved in messageTemplate"
  }
  contract   """
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
  verify contract "Emit Diagnostic From Pattern: pattern diagnostic emission holds — violation_detected, pattern_configured, diagnostic_emitted, template_interpolated"
}

behavior register_custom_validation_patterns "Register Custom Validation Patterns" {
  features   [declarative_validation_rules]
  invariants [zero_domain_knowledge_core, declarative_validation_determinism]
  category   command
  types      [ValidationRulePattern, CustomValidationPattern, ExtensionDeclaration]
  refs       [provide_host_function_query_graph]
  ports      [WasmRuntime]
  consumes   [extension_manifests_loaded]
  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all manifests are parsed and accessible"
    wasm_runtime_available           "WasmRuntime port is available for resolving and dispatching Wasm exports"
  }
  ensures {
    custom_patterns_registered "Custom validation patterns registered alongside declarative patterns"
    wasm_functions_resolved    "wasm_function fields resolved to Wasm exports or warnings emitted for unresolvable names"
  }
  contract   """
    When an extension declares validation rules with check kind "custom",
    the compiler MUST resolve the wasm_function field to a Wasm export in
    the extension's module. The custom pattern MUST be registered alongside
    declarative patterns. During validation, custom patterns MUST be
    dispatched to the Wasm runtime via the extension's exported function.
    The Wasm function receives the protocol's ValidatorContext (the entity,
    the resolution of its references, the declared types and the host's
    primitives) and answers the protocol's ValidatorVerdict (`pass`, or
    `fail` with the offending field and value), read strictly: an answer
    that is not a verdict is a failed call (call_extension_exports), never
    a pass or a fail.
    The function body MAY call the `specforge.query_graph` host function
    (see provide_host_function_query_graph) to access the compiled graph
    for cross-entity semantic checks and custom graph traversals.
    On failure, the engine MUST emit a diagnostic using the pattern's
    configured code, severity, and message template. Unresolvable
    wasm_function names MUST produce a warning (W112) at registration
    time: when the extensions load, each custom rule's wasm_function is
    called once on an entity of the rule's target kind that declares
    nothing, and a call that does not answer with a verdict is reported.
    The rule stays registered; dispatch then skips an entity whose call
    fails without reporting it again. A custom rule that names no
    wasm_function MUST produce W112 and MUST NOT be registered.
  """
  verify unit "custom pattern registered with wasm_function reference"
  verify unit "unresolvable wasm_function produces warning"
  verify unit "custom rule without a wasm_function produces warning and is not registered"
  verify unit "custom pattern dispatched to Wasm runtime during validation"
  verify unit "custom pattern failure emits configured diagnostic"
  verify unit "a custom validator's verdict is read as the protocol's ValidatorVerdict, and a failure is reported once as W112"
  verify contract "Register Custom Validation Patterns: custom validation pattern registration holds — extension_manifests_loaded_fired, wasm_runtime_available, custom_patterns_registered, wasm_functions_resolved"
}

// -- Field Validation --------------------------------------------------------

behavior detect_unknown_entity_fields "Detect Unknown Entity Fields" {
  features   [zero_entity_validation, dynamic_entity_registration]
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   validation
  types      [FieldRegistryEntry, KindRegistryEntry, Diagnostic]
  consumes   [registries_populated]
  requires {
    registries_populated_fired "registries_populated event has fired, confirming FieldRegistry and KindRegistry are fully populated"
  }
  ensures {
    unknown_fields_diagnosed "W020 warning emitted for every unrecognized field name on registered entity kinds"
    cascading_avoided        "Field validation skipped for entities with unregistered kinds (already E024)"
  }
  contract   """
    During Phase 2 semantic validation, the compiler MUST scan all parsed
    entity blocks whose kind is registered in the KindRegistry and check
    each field name against the FieldRegistry for that kind. Every field
    name not present in the FieldRegistry for the entity's kind MUST
    produce a W020 warning diagnostic with the entity's source span, the
    unrecognized field name, and the entity kind name. `title` is
    structural and MUST NOT be checked against the FieldRegistry. Every
    other name, `expression` included, MUST be checked like any field: the
    prove pass reads only fields an extension declares a proof role for
    (ADR 0009), so an undeclared `expression` is W020. When a builtin
    extension's enhancement declares the field on that kind (the bundled
    field index, e.g. an invariant's `expression` from @specforge/formal),
    the W020 MUST suggest installing that extension, as E024 does for a
    kind.
    `verify` is reserved syntax whose meaning extensions supply
    (ADR 0002): it MUST be accepted only on kinds an extension made
    testable (supports_verify) and produce W020 elsewhere. When the entity's kind
    itself is unregistered (already reported as E024), field validation
    MUST be skipped for that entity to avoid cascading diagnostics.
  """
  verify unit "unregistered field name produces W020"
  verify unit "W020 includes field name, entity kind, and source span"
  verify unit "registered field name does not produce W020"
  verify unit "structural fields (title, verify) not checked against FieldRegistry"
  verify unit "verify on a kind no extension made testable produces W020"
  verify unit "expression is checked like any other field (W020 where undeclared)"
  verify unit "an undeclared field a builtin enhancement adds suggests its extension"
  verify unit "field validation skipped when entity kind is unregistered"
  verify contract "Detect Unknown Entity Fields: unknown field detection holds — registries_populated_fired, unknown_fields_diagnosed, cascading_avoided"
}

behavior check_field_value_types "Check Field Value Types" {
  features   [dynamic_entity_registration]
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   validation
  types      [FieldRegistryEntry, Diagnostic]
  consumes   [registries_populated]
  requires {
    registries_populated_fired "registries_populated event has fired, confirming the FieldRegistry holds each field's declared type"
  }
  ensures {
    single_values_listed "A single value on a string_list or reference_list field is stored as a one-item list"
    mismatches_diagnosed "E061 error for every value that cannot be the field's declared type"
    undeclared_untouched "Unknown fields keep their parsed value and produce no E061"
  }
  contract   """
    The grammar parses every field value generically; the value's type
    comes from the FieldRegistry. During Phase 2 semantic validation the
    compiler MUST coerce each registered field's value to its declared
    type before references resolve and before anything is exported: a
    single string or reference on a string_list or reference_list field
    MUST become a one-item list, stored exactly as the one-item list
    syntax (`field [value]`) would be. A quoted integer or boolean on an
    integer or bool field MUST become that integer or boolean, and an
    integer or boolean on a string or enum field MUST become its text.
    Coercion MUST NOT produce a diagnostic.

    A value that still cannot be the declared type MUST produce an E061
    error at the value's source span naming the field, the declared type
    and the value given: a value that is not an integer on an integer
    field, not true or false on a bool field, not one of the declared
    values on an enum field (suggesting the closest declared value), or
    a list on a field declared as a single value. Fields no extension
    registers keep their parsed value (W020 reports them). The check MUST
    run wherever the registry checks run: check, watch and the LSP.
  """
  verify unit "a single string on a string_list field becomes a one-item list"
  verify unit "a single reference on a reference_list field becomes a one-item list"
  verify unit "a value that is not the declared integer, bool or enum type is an error"
  verify unit "a list on a field declared as a single value is an error"
  verify unit "an enum value suggests the closest declared value"
  verify unit "an export with a coerced string_list validates against the published schema"
  verify contract "Check Field Value Types: declared field types hold — registries_populated_fired, single_values_listed, mismatches_diagnosed, undeclared_untouched"
}
