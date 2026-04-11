// @specforge/formal validation rules — formal-specific declarative validation patterns
//
// W058 (downgraded from E033): behavior not satisfying feature requirements
// is a structural coverage check, not semantic verification.
// W059-W060: condition entity validation
// W061-W063: property entity validation
// W064-W065: axiom entity validation
// W066-W068: protocol entity validation
// W069-W071: refinement entity validation
// W072-W074: process entity validation

use "extensions/formal/types"
use "extensions/formal/invariants"
use "types/zero-entity-core"

// ── Condition Validation (W058-W060) ─────────────────────────

behavior fa_validate_w058_feature_coverage_mismatch "W058: Feature Coverage Mismatch" {
  category query
  types    [RefinementChain]
  contract """
    Detect behaviors that claim to implement a feature but whose
    conditions do not structurally cover the feature's requirements.
    Downgraded from E033 (error) to W058 (warning) per expert review:
    this is a structural coverage check, not semantic verification.
    The check has no implementable algorithm for semantic requirement
    satisfaction — it can only check structural condition name overlap.
    Requires warning_level=strict to fire.
  """
  requires {
    layering_chains_built  "layering chains are fully constructed"
    features_resolved      "feature references in behaviors are resolved"
    strict_warning_level   "warning_level is set to strict"
  }
  ensures  {
    covering_passes        "behavior whose conditions structurally cover feature requirements produces no diagnostic"
    uncovering_warned      "behavior not structurally covering feature requirements produces W058 warning"
    correct_template       "message template is: behavior '{id}' may not satisfy requirements of feature '{feature_id}': {reason} (structural check only)"
  }

  features [fa_specification_layering]

  verify unit "behavior covering feature requirements passes"
  verify unit "behavior not covering feature requirements produces W058"
  verify unit "W058 severity is warning (not error)"
  verify unit "W058 only fires at warning_level=strict"
}

behavior fa_validate_orphan_conditions "W059: Orphan Conditions" {
  category   query
  invariants [fa_condition_entity_reachability]
  types      [FormalCondition]
  contract   """
    Detect condition entities with no incoming RequiresCondition,
    EnsuresCondition, or MaintainsCondition edges. An orphan condition
    is a graph node that exists but is never referenced by any behavior
    or invariant — it should either be referenced or removed.
    Requires warning_level=strict to fire.
  """
  requires   {
    graph_built            "entity graph is fully constructed with all edges"
    strict_warning_level   "warning_level is set to strict"
  }
  ensures    {
    orphan_detected        "condition with no incoming edges produces W059 warning"
    referenced_passes      "condition with at least one incoming edge produces no diagnostic"
    correct_template       "message template is: condition '{id}' is not referenced by any behavior or invariant"
  }

  features [fa_structured_conditions]

  verify unit "condition with no incoming edges produces W059"
  verify unit "condition with RequiresCondition edge passes"
  verify unit "condition with EnsuresCondition edge passes"
  verify unit "condition with MaintainsCondition edge passes"
  verify unit "W059 only fires at warning_level=strict"
}

behavior fa_validate_condition_description "W060: Empty Condition Description" {
  category query
  types    [FormalCondition]
  contract """
    Detect condition entities with an empty or whitespace-only description.
    Conditions are machine-readable contracts — a blank description defeats
    the purpose and makes the condition opaque to agents.
  """
  ensures  {
    empty_warned           "condition with empty description produces W060 warning"
    non_empty_passes       "condition with non-empty description passes"
    correct_template       "message template is: condition '{id}' has empty description"
  }

  features [fa_structured_conditions]

  verify unit "condition with empty description produces W060"
  verify unit "condition with non-empty description passes"
}

// ── Property Validation (W061-W063) ──────────────────────────

behavior fa_validate_orphan_property "W061: Orphan Property" {
  category   query
  invariants [fa_property_entity_reachability]
  types      [FormalProperty]
  contract   """
    Detect property entities with no incoming Satisfies edges. An orphan
    property is a temporal assertion that no behavior claims to satisfy —
    it should either be referenced via the satisfies field or removed.
    Requires warning_level=strict to fire.
  """
  requires   {
    graph_built            "entity graph is fully constructed with all edges"
    strict_warning_level   "warning_level is set to strict"
  }
  ensures    {
    orphan_detected        "property with no incoming Satisfies edges produces W061 warning"
    referenced_passes      "property with at least one incoming Satisfies edge produces no diagnostic"
    correct_template       "message template is: property '{id}' is not satisfied by any behavior"
  }

  features [fa_temporal_properties]

  verify unit "property with no incoming Satisfies edges produces W061"
  verify unit "property with Satisfies edge passes"
  verify unit "W061 only fires at warning_level=strict"
}

behavior fa_validate_empty_property_description "W062: Empty Property Description" {
  category query
  types    [FormalProperty]
  contract """
    Detect property entities with an empty or whitespace-only description.
    Properties are temporal assertions — a blank description makes the
    property opaque to agents and reviewers.
  """
  ensures  {
    empty_warned           "property with empty description produces W062 warning"
    non_empty_passes       "property with non-empty description passes"
    correct_template       "message template is: property '{id}' has empty description"
  }

  features [fa_temporal_properties]

  verify unit "property with empty description produces W062"
  verify unit "property with non-empty description passes"
}

behavior fa_validate_property_without_kind "W063: Property Without Kind" {
  category query
  types    [FormalProperty, PropertyKind]
  contract """
    Detect property entities with no kind field. The kind field
    (safety/liveness/fairness) classifies the temporal assertion and
    is required for meaningful graph queries and agent consumption.
  """
  ensures  {
    missing_warned         "property with no kind field produces W063 warning"
    present_passes         "property with kind field produces no diagnostic"
    correct_template       "message template is: property '{id}' has no kind (safety/liveness/fairness)"
  }

  features [fa_temporal_properties]

  verify unit "property with no kind produces W063"
  verify unit "property with kind=safety passes"
  verify unit "property with kind=liveness passes"
  verify unit "property with kind=fairness passes"
}

// ── Axiom Validation (W064-W065) ─────────────────────────────

behavior fa_validate_orphan_axiom "W064: Orphan Axiom" {
  category   query
  invariants [fa_axiom_entity_reachability]
  types      [FormalAxiom]
  contract   """
    Detect axiom entities with no incoming AssumedBy edges. An orphan
    axiom is an assumption that no condition depends on — it should
    either be referenced via the assumes field or removed.
    Requires warning_level=strict to fire.
  """
  requires   {
    graph_built            "entity graph is fully constructed with all edges"
    strict_warning_level   "warning_level is set to strict"
  }
  ensures    {
    orphan_detected        "axiom with no incoming AssumedBy edges produces W064 warning"
    referenced_passes      "axiom with at least one incoming AssumedBy edge produces no diagnostic"
    correct_template       "message template is: axiom '{id}' is not assumed by any condition"
  }

  features [fa_axiom_foundations]

  verify unit "axiom with no incoming AssumedBy edges produces W064"
  verify unit "axiom with AssumedBy edge passes"
  verify unit "W064 only fires at warning_level=strict"
}

behavior fa_validate_empty_axiom_description "W065: Empty Axiom Description" {
  category query
  types    [FormalAxiom]
  contract """
    Detect axiom entities with an empty or whitespace-only description.
    Axioms are assumed-true foundations — a blank description makes
    the assumption invisible and unjustifiable.
  """
  ensures  {
    empty_warned           "axiom with empty description produces W065 warning"
    non_empty_passes       "axiom with non-empty description passes"
    correct_template       "message template is: axiom '{id}' has empty description"
  }

  features [fa_axiom_foundations]

  verify unit "axiom with empty description produces W065"
  verify unit "axiom with non-empty description passes"
}

// ── Protocol Validation (W066-W068) ──────────────────────────

behavior fa_validate_orphan_protocol "W066: Orphan Protocol" {
  category   query
  invariants [fa_protocol_entity_reachability]
  types      [FormalProtocol]
  contract   """
    Detect protocol entities with no incoming FollowsProtocol edges.
    An orphan protocol is a sync contract that no event follows — it
    should either be referenced via the follows_protocol field or removed.
    Requires warning_level=strict to fire.
  """
  requires   {
    graph_built            "entity graph is fully constructed with all edges"
    strict_warning_level   "warning_level is set to strict"
  }
  ensures    {
    orphan_detected        "protocol with no incoming FollowsProtocol edges produces W066 warning"
    referenced_passes      "protocol with at least one incoming FollowsProtocol edge produces no diagnostic"
    correct_template       "message template is: protocol '{id}' is not followed by any event"
  }

  features [fa_protocol_contracts]

  verify unit "protocol with no incoming FollowsProtocol edges produces W066"
  verify unit "protocol with FollowsProtocol edge passes"
  verify unit "W066 only fires at warning_level=strict"
}

behavior fa_validate_empty_protocol_description "W067: Empty Protocol Description" {
  category query
  types    [FormalProtocol]
  contract """
    Detect protocol entities with an empty or whitespace-only description.
    Protocols are synchronization contracts — a blank description makes
    the contract opaque to agents and event graph analysis.
  """
  ensures  {
    empty_warned           "protocol with empty description produces W067 warning"
    non_empty_passes       "protocol with non-empty description passes"
    correct_template       "message template is: protocol '{id}' has empty description"
  }

  features [fa_protocol_contracts]

  verify unit "protocol with empty description produces W067"
  verify unit "protocol with non-empty description passes"
}

behavior fa_validate_protocol_ordering_conflict "W068: Protocol Ordering Conflict" {
  category query
  types    [FormalProtocol]
  contract """
    Detect protocol entities whose ordering field references event IDs
    that do not exist in the graph. The ordering field declares the
    expected sequence of events — if an event in the ordering is not
    found, the protocol is misconfigured.
  """
  requires {
    graph_built            "entity graph is fully constructed with all edges"
  }
  ensures  {
    missing_event_warned   "protocol ordering referencing non-existent event produces W068 warning"
    valid_ordering_passes  "protocol ordering with all valid event references produces no diagnostic"
    correct_template       "message template is: protocol '{id}' ordering references unknown event '{event_id}'"
  }

  features [fa_protocol_contracts]

  verify unit "protocol ordering referencing non-existent event produces W068"
  verify unit "protocol ordering with all valid events passes"
  verify unit "protocol with empty ordering passes (no ordering to validate)"
}

// ── Refinement Validation (W069-W071) ───────────────────────

behavior fa_validate_orphan_refinement "W069: Orphan Refinement" {
  category   query
  invariants [fa_refinement_entity_reachability]
  types      [FormalRefinement]
  contract   """
    Detect refinement entities with no incoming RefinesTo or
    RefinementChainLink edges. An orphan refinement is a graph node
    that captures an abstract-to-concrete mapping but is disconnected
    from all behaviors and other refinements — it should either be
    connected or removed. Requires warning_level=strict to fire.
  """
  requires   {
    graph_built            "entity graph is fully constructed with all edges"
    strict_warning_level   "warning_level is set to strict"
  }
  ensures    {
    orphan_detected        "refinement with no RefinesTo or RefinementChainLink edges produces W069 warning"
    referenced_passes      "refinement with at least one RefinesTo or RefinementChainLink edge produces no diagnostic"
    correct_template       "message template is: refinement '{id}' is not connected to any behavior or refinement chain"
  }

  features [fa_refinement_layering]

  verify unit "refinement with no edges produces W069"
  verify unit "refinement with RefinesTo edge passes"
  verify unit "refinement with RefinementChainLink edge passes"
  verify unit "W069 only fires at warning_level=strict"
}

behavior fa_validate_empty_refinement_description "W070: Empty Refinement Description" {
  category query
  types    [FormalRefinement]
  contract """
    Detect refinement entities with an empty or whitespace-only description.
    Refinements capture abstract-to-concrete mappings — a blank description
    makes the mapping opaque to agents and reviewers.
  """
  ensures  {
    empty_warned           "refinement with empty description produces W070 warning"
    non_empty_passes       "refinement with non-empty description passes"
    correct_template       "message template is: refinement '{id}' has empty description"
  }

  features [fa_refinement_layering]

  verify unit "refinement with empty description produces W070"
  verify unit "refinement with non-empty description passes"
}

behavior fa_validate_refinement_without_delta "W071: Refinement Without Condition Delta" {
  category query
  types    [FormalRefinement, ConditionDelta]
  contract """
    Detect refinement entities with no conditions field. The conditions
    field captures what changes between abstract and concrete — without
    it, the refinement is purely structural with no formal content.
    Requires warning_level=strict to fire.
  """
  requires {
    strict_warning_level   "warning_level is set to strict"
  }
  ensures  {
    missing_warned         "refinement with no conditions field produces W071 warning"
    present_passes         "refinement with conditions field produces no diagnostic"
    correct_template       "message template is: refinement '{id}' has no condition delta"
  }

  features [fa_refinement_layering]

  verify unit "refinement with no conditions produces W071"
  verify unit "refinement with conditions passes"
  verify unit "W071 only fires at warning_level=strict"
}

// ── Refinement Structural Validation (E041) ─────────────────

behavior fa_validate_refinement_self_reference "E041b: Refinement Self-Reference" {
  category query
  types    [FormalRefinement]
  contract """
    Detect refinement entities where abstract_id equals concrete_id.
    A refinement that maps a behavior to itself is structurally invalid —
    it creates a trivial cycle. Produces E041 error.
  """
  requires {
    graph_built            "entity graph is fully constructed with all edges"
  }
  ensures  {
    self_ref_detected      "refinement with abstract_id == concrete_id produces E041 error"
    distinct_passes        "refinement with distinct abstract_id and concrete_id produces no diagnostic"
    correct_template       "message template is: refinement '{id}' maps behavior '{behavior_id}' to itself"
  }

  features [fa_refinement_layering]

  verify unit "refinement with abstract_id == concrete_id produces E041"
  verify unit "refinement with distinct IDs passes"
}

// ── Process Validation (W072-W074) ──────────────────────────

behavior fa_validate_orphan_process "W072: Orphan Process" {
  category   query
  invariants [fa_process_entity_reachability]
  types      [FormalProcess]
  contract   """
    Detect process entities with no incoming ParticipatesIn edges. An
    orphan process is a communicating process that no event participates
    in — it should either have events assigned to its alphabet or be
    removed. Requires warning_level=strict to fire.
  """
  requires   {
    graph_built            "entity graph is fully constructed with all edges"
    strict_warning_level   "warning_level is set to strict"
  }
  ensures    {
    orphan_detected        "process with no incoming ParticipatesIn edges produces W072 warning"
    referenced_passes      "process with at least one incoming ParticipatesIn edge produces no diagnostic"
    correct_template       "message template is: process '{id}' has no events participating in it"
  }

  features [fa_process_modeling]

  verify unit "process with no incoming ParticipatesIn edges produces W072"
  verify unit "process with ParticipatesIn edge passes"
  verify unit "W072 only fires at warning_level=strict"
}

behavior fa_validate_empty_process_description "W073: Empty Process Description" {
  category query
  types    [FormalProcess]
  contract """
    Detect process entities with an empty or whitespace-only description.
    Processes model communicating sequential processes — a blank description
    makes the process opaque to agents and event graph analysis.
  """
  ensures  {
    empty_warned           "process with empty description produces W073 warning"
    non_empty_passes       "process with non-empty description passes"
    correct_template       "message template is: process '{id}' has empty description"
  }

  features [fa_process_modeling]

  verify unit "process with empty description produces W073"
  verify unit "process with non-empty description passes"
}

behavior fa_validate_process_without_alphabet "W074: Process Without Alphabet" {
  category query
  types    [FormalProcess]
  contract """
    Detect process entities with an empty or absent alphabet field. The
    alphabet defines which events the process can engage in — without it,
    the process is disconnected from the event graph.
    Requires warning_level=strict to fire.
  """
  requires {
    strict_warning_level   "warning_level is set to strict"
  }
  ensures  {
    missing_warned         "process with empty or absent alphabet produces W074 warning"
    present_passes         "process with non-empty alphabet produces no diagnostic"
    correct_template       "message template is: process '{id}' has no alphabet (no events declared)"
  }

  features [fa_process_modeling]

  verify unit "process with empty alphabet produces W074"
  verify unit "process with non-empty alphabet passes"
  verify unit "W074 only fires at warning_level=strict"
}
