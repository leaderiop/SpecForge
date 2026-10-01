// @specforge/software validation rules — declarative validation patterns

use "extensions/software/invariants"
use "extensions/software/types"
use "types/zero-entity-core"

behavior se_validate_orphan_behaviors "W001: Orphan Behaviors" {
  features   [se_validation_suite]
  category   query
  invariants [se_edge_consistency]
  types      [ValidationRulePattern]
  contract   """
    Detect behavior entities with no incoming Implements edges.
  """
  requires {
    graph_built "entity graph is fully constructed with all edges"
  }
  ensures {
    orphan_detected   "behavior with no incoming Implements edge produces W001 warning"
    non_orphan_passes "behavior with incoming Implements edge produces no diagnostic"
    correct_severity  "W001 severity is warning"
    correct_template  "message template is: behavior '{id}' is not referenced by any feature"
  }
  verify unit "behavior with no incoming Implements edge produces W001"
  verify unit "behavior with incoming Implements edge passes"
  verify unit "W001 severity is warning"
}

behavior se_validate_orphan_types "W002: Orphan Types" {
  features [se_validation_suite]
  category query
  types    [ValidationRulePattern]
  contract """
    Detect type entities with no incoming UsesType edges.
  """
  requires {
    graph_built "entity graph is fully constructed with all edges"
  }
  ensures {
    orphan_detected  "type with no incoming UsesType edge produces W002 warning"
    correct_template "message template is: type '{id}' is not referenced by any behavior or port"
  }
  verify unit "type with no incoming UsesType edge produces W002"
  verify unit "type with incoming UsesType edge passes"
}

behavior se_validate_unused_invariants "W003: Unused Invariants" {
  features [se_validation_suite]
  category query
  types    [ValidationRulePattern]
  contract """
    Detect invariant entities that nothing references: no behavior lists
    them in `invariants`, `requires`, `ensures` or `maintains`. This is the
    only orphan-invariant finding; `analyze coverage` counts them without
    reporting them again.
  """
  requires {
    graph_built "entity graph is fully constructed with all edges"
  }
  ensures {
    unused_detected   "invariant nothing references produces W003"
    referenced_passes "invariant with incoming reference edge produces no diagnostic"
  }
  verify unit "invariant nothing references produces W003"
  verify unit "invariant with incoming reference edge passes"
}

behavior se_validate_orphan_ports "W005: Orphan Ports" {
  features [se_validation_suite]
  category query
  types    [ValidationRulePattern]
  contract """
    Detect port entities with no incoming UsesPort edges.
  """
  ensures {
    orphan_detected  "port with no incoming UsesPort edge produces W005"
    correct_template "message template is: port '{id}' is not referenced by any behavior"
  }
  verify unit "port with no incoming UsesPort edge produces W005"
  verify unit "port with incoming UsesPort edge passes"
}

// W006 is allocated to @specforge/product (Orphan Capabilities → W042)

behavior se_validate_event_triggers "E051: Invalid Event Triggers" {
  features   [se_validation_suite]
  category   query
  invariants [se_event_trigger_validity]
  types      [ValidationRulePattern, ValidationPatternKind]
  contract   """
    Detect event entities whose trigger field references a non-behavior
    entity. Check pattern: field_value_constraint.
  """
  requires {
    trigger_field_present "event entity has a trigger field value"
  }
  ensures {
    valid_trigger_passes  "event trigger referencing behavior produces no diagnostic"
    invalid_trigger_error "event trigger referencing non-behavior produces E051 error"
    correct_template      "message template is: event '{id}' trigger must reference a behavior, found {kind} '{value}'"
  }
  verify unit "event trigger referencing behavior passes"
  verify unit "event trigger referencing type produces E051"
  verify unit "event trigger referencing feature produces E051"
  verify unit "E051 severity is error"
  // Note: missing trigger field is caught by se_validate_entity_fields
  // (generic required-field check), not by this rule.
}

behavior se_validate_orphan_events "W007: Orphan Events" {
  features [se_validation_suite]
  category query
  types    [ValidationRulePattern]
  contract """
    Detect event entities with no incoming Produces edges.
  """
  ensures {
    orphan_detected  "event with no incoming Produces edge produces W007"
    correct_template "message template is: event '{id}' is not produced by any behavior"
  }
  verify unit "event with no incoming Produces edge produces W007"
  verify unit "event with incoming Produces edge passes"
}

behavior se_validate_features_with_empty_behaviors "W008: Features with Empty Behaviors" {
  features [se_validation_suite]
  category query
  types    [ValidationRulePattern]
  contract """
    Detect feature entities with an empty behaviors list.
  """
  ensures {
    empty_detected   "feature with empty behaviors list produces W008"
    non_empty_passes "feature with at least one behavior produces no diagnostic"
    correct_template "message template is: feature '{id}' has no behaviors -- specification may be incomplete"
  }
  verify unit "feature with empty behaviors list produces W008"
  verify unit "feature with at least one behavior suppresses W008"
}

behavior se_validate_port_methods "E004: Invalid Port Methods" {
  features [se_validation_suite]
  category query
  types    [ValidationRulePattern, PortOperation]
  contract """
    Detect port entities whose methods block contains operations with
    invalid type signatures.
  """
  requires {
    type_registry_available "all declared type entities are registered"
  }
  ensures {
    valid_types_pass    "port method with valid type references produces no diagnostic"
    invalid_types_error "port method with unknown type reference produces E004 error"
    correct_template    "message template is: port '{id}' method '{field}' references unknown type '{value}'"
  }
  verify unit "port method with valid type references passes"
  verify unit "port method with unknown type reference produces E004"
}

behavior se_validate_type_field_annotations "W010: Unknown Field Annotations" {
  features [se_validation_suite]
  category query
  types    [ValidationRulePattern, FieldAnnotation]
  contract """
    Detect type entities whose field definitions contain unknown
    annotations. Valid: @readonly, @unique, @optional, @literal.
  """
  ensures {
    known_passes     "field with @readonly annotation produces no diagnostic"
    unknown_warns    "field with unknown annotation produces W010 warning"
    correct_template "message template is: type '{id}' field '{field}' has unknown annotation '{value}'"
  }
  verify unit "field with @readonly annotation passes"
  verify unit "field with @unknown annotation produces W010"
}
