// @specforge/software extension behaviors — entity kind and edge registration

use "types/zero-entity-core"
use "extensions/software/types"
use "extensions/software/invariants"
behavior se_register_entity_kinds "Register Software Entity Kinds" {
  category command
  invariants [se_behavior_testability, se_invariant_testability, se_event_testability, se_feature_non_testable, se_manifest_six_entity_kinds]
  types [ManifestEntityKind, SoftwareBehavior, SoftwareInvariant, SoftwareFeature, SoftwareEvent, SoftwareTypeDef, SoftwarePort]

  contract """
    The @specforge/software extension MUST register 6 entity kinds with
    full metadata in the KindRegistry.
  """

  requires {
    manifest_loaded        "ManifestV2 is parsed and schema-validated"
    no_duplicate_kinds     "KindRegistry has no entries with names matching this extension's kinds"
  }

  ensures {
    behavior_registered    "KindRegistry contains behavior: testable=true, supportsVerify=true, semanticToken=function, lspIcon=Method, dotShape=box"
    invariant_registered   "KindRegistry contains invariant: testable=true, supportsVerify=true, semanticToken=property, lspIcon=Property, dotShape=diamond"
    feature_registered     "KindRegistry contains feature: testable=false, supportsVerify=false, semanticToken=class, lspIcon=Package, dotShape=hexagon"
    event_registered       "KindRegistry contains event: testable=true, supportsVerify=true, semanticToken=event, lspIcon=Event, dotShape=ellipse"
    type_registered        "KindRegistry contains type: testable=false, supportsVerify=false, semanticToken=type, lspIcon=Struct, dotShape=rectangle"
    port_registered        "KindRegistry contains port: testable=false, supportsVerify=false, semanticToken=interface, lspIcon=Interface, dotShape=trapezium"
    six_kinds_total        "KindRegistry has exactly 6 domain entries after registration"
  }

  verify unit "behavior registered with testable=true and supportsVerify=true"
  verify unit "invariant registered with testable=true and supportsVerify=true"
  verify unit "feature registered with testable=false"
  verify unit "event registered with semanticToken=event"
  verify unit "type registered with dotShape=rectangle"
  verify unit "port registered with lspIcon=Interface"

}

behavior se_register_edge_types "Register Software Edge Types" {
  category command
  invariants [se_manifest_nine_edge_types]
  types [ManifestEdgeType]

  contract """
    The @specforge/software extension MUST register 9 edge types that
    model all relationships between the 6 entity kinds.
  """

  requires {
    kinds_registered       "all 6 entity kinds are in KindRegistry"
  }

  ensures {
    references_edge        "EdgeTypeSet contains References (general cross-reference)"
    implements_edge        "EdgeTypeSet contains Implements (feature->behavior, sourceKind=feature, targetKind=behavior)"
    produces_edge          "EdgeTypeSet contains Produces (behavior->event, sourceKind=behavior, targetKind=event)"
    consumes_edge          "EdgeTypeSet contains Consumes (behavior->event, sourceKind=behavior, targetKind=event)"
    uses_type_edge         "EdgeTypeSet contains UsesType (behavior->type, port->type)"
    uses_port_edge         "EdgeTypeSet contains UsesPort (behavior->port)"
    enforces_edge          "EdgeTypeSet contains Enforces (invariant->behavior, sourceKind=invariant, targetKind=behavior)"
    imports_edge           "EdgeTypeSet contains Imports (spec file use statements)"
    links_to_edge          "EdgeTypeSet contains LinksTo (generic linkage: tests field, external refs)"
    nine_edges_total       "EdgeTypeSet has exactly 9 entries"
  }

  verify unit "all 9 edge types registered in edge set"
  verify unit "Implements edge has sourceKind=feature and targetKind=behavior"
  verify unit "Produces edge has sourceKind=behavior and targetKind=event"
  verify unit "Enforces edge has sourceKind=invariant and targetKind=behavior"

}

behavior se_register_field_definitions "Register Software Field Definitions" {
  category command
  types [ManifestField, ManifestEntityKind, BehaviorCategory, PortDirection, TypeDefKind, TypeFieldDef, RiskLevel, ProofObligationKind]

  contract """
    The @specforge/software extension MUST register field definitions for
    each entity kind with name, type, edge mapping, and target kind.
  """

  requires {
    kinds_and_edges_registered "all 6 kinds and 9 edge types are registered"
  }

  ensures {
    behavior_fields        "behavior has: contract(string), invariants(reference[]->invariant, Enforces), types(reference[]->type, UsesType), ports(reference[]->port, UsesPort), produces(reference[]->event, Produces), consumers(reference[]->event, Consumes), category(string), abstract(string), refines(reference->behavior, References), requires(block), ensures(block), maintains(block), tests(string[]), gherkin(string[], file_reference=true)"
    invariant_fields       "invariant has: guarantee(string), enforced_by(reference[]->behavior, Enforces), risk(string)"
    feature_fields         "feature has: behaviors(reference[]->behavior, Implements), problem(string), solution(string)"
    event_fields           "event has: trigger(reference->behavior, Produces), channel(string), payload(reference->type, UsesType), consumers(reference[]->behavior, Consumes), sync(block)"
    type_fields            "type has: kind(string), fields(block)"
    port_fields            "port has: direction(string), category(string), methods(block)"
  }

  verify unit "behavior contract field registered as string type"
  verify unit "behavior invariants field registered with Enforces edge"
  verify unit "feature behaviors field registered with Implements edge"
  verify unit "event trigger field registered with Produces edge"

}

behavior se_register_validation_rules "Register Software Validation Rules" {
  category command
  types [ValidationRulePattern, ValidationPatternKind]

  contract """
    The @specforge/software extension MUST register declarative validation
    rules in its manifest.
  """

  requires {
    field_definitions_registered "all field definitions are in FieldRegistry"
  }

  ensures {
    rules_registered       "all W001-W005, W007-W010, E006, E004 rules are registered"
    rules_sorted           "rules are sorted by diagnostic code for deterministic execution"
  }

  verify unit "validation rules registered from manifest"
  verify unit "rules include W001-W005, W007-W010, E006, and E004"
  verify unit "rules sorted by diagnostic code"

}

behavior se_register_verify_kinds "Register Software Verify Kinds" {
  category command
  types [ManifestEntityKind, SoftwareVerifyKind]

  contract """
    The @specforge/software extension MUST register 11 verify kinds across
    its entity kinds.
  """

  requires {
    kinds_registered       "all 6 entity kinds are in KindRegistry"
  }

  ensures {
    behavior_verify_kinds  "behavior allows: unit, integration, property, load, e2e, contract, refinement, trace, mutation"
    invariant_verify_kinds "invariant allows: property, unit, mutation"
    event_verify_kinds     "event allows: integration, deadlock_free, liveness"
    eleven_kinds_total     "11 unique verify kinds registered across all entity kinds"
  }

  verify unit "behavior allows unit, integration, property, load, e2e, contract, refinement, trace, mutation"
  verify unit "invariant allows property, unit, mutation"
  verify unit "event allows integration, deadlock_free, liveness"
  verify unit "unknown verify kind on behavior produces warning"

}

behavior se_register_lsp_metadata "Register Software LSP Metadata" {
  category command
  types [ManifestEntityKind, KindRegistryEntry]

  contract """
    The @specforge/software extension MUST register LSP metadata for each
    entity kind: semanticToken for highlighting, lspIcon for outline.
  """

  ensures {
    semantic_tokens_set    "all 6 entity kinds have a semanticToken value"
    lsp_icons_set          "all 6 entity kinds have an lspIcon value"
  }

  verify unit "semantic tokens registered for all 6 entity kinds"
  verify unit "LSP icons registered for all 6 entity kinds"

}

behavior se_validate_entity_fields "Validate Software Entity Fields" {
  category query
  invariants [se_port_direction_constraint]
  types [ManifestField, ManifestEntityKind]

  contract """
    During semantic validation, field definitions MUST be used to
    validate field values on parsed entities.
  """

  requires {
    registries_populated   "KindRegistry, FieldRegistry, EdgeTypeSet are fully populated"
    phase_two_active       "compilation is in Phase 2 (semantic validation)"
  }

  ensures {
    reference_kinds_checked "reference fields resolve to entities of the correct target kind"
    required_fields_checked "missing required fields produce diagnostics"
    block_structure_checked "block fields have valid internal structure"
  }

  verify unit "reference field resolving to correct kind passes"
  verify unit "reference field resolving to wrong kind produces error"
  verify unit "missing required field produces diagnostic"

}

behavior se_parse_gherkin_statements "Register Gherkin Field" {
  category command
  invariants [se_behavior_testability]

  contract """
    The @specforge/software extension MUST declare a gherkin field with
    type string_list and file_reference=true on the behavior entity kind
    via the FieldRegistry. The field is parsed as a standard StringList
    value — no dedicated grammar rule or AST type is needed. File
    existence validation is handled by the generic
    validate_file_reference_paths behavior (E016). The gherkin field
    is NOT a core grammar construct — it is a regular extension-declared
    field like any other.
  """

  verify unit "gherkin field registered with type string_list"
  verify unit "gherkin field has file_reference=true"
  verify unit "gherkin values parsed as standard StringList"

}

behavior se_validate_entity_references "Validate Software Entity References" {
  category query
  types [ManifestField, ManifestEdgeType]

  contract """
    For each reference field, the compiler MUST verify that the
    referenced entity exists and is of the expected target kind.
  """

  requires {
    graph_constructed      "entity graph is fully built with all nodes"
  }

  ensures {
    resolved_refs_valid    "references to existing entities of correct kind pass"
    unresolved_refs_error  "references to non-existent entities produce error"
    cross_extension_soft     "references to entities from uninstalled extensions produce I004 info"
  }

  verify unit "reference to existing entity of correct kind passes"
  verify unit "reference to non-existent entity produces error"
  verify unit "reference to entity from uninstalled extension produces I004"

}
