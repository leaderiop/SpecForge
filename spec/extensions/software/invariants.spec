// @specforge/software extension invariants — guarantees on entity behavior

use extensions/software/types
use extensions/software/behaviors
use extensions/software/formal-contracts
use extensions/software/formal-refinement
use extensions/software/validation-rules

invariant se_behavior_testability "Behavior Testability" {
  guarantee """
    Behavior entities MUST have testable=true. They MUST accept verify
    kinds: unit, integration, property, load, e2e, contract, refinement,
    trace, mutation. Behavior entities MUST also support gherkin blocks
    via supportsGherkin=true. This is the primary testable entity kind.
  """
  enforced_by [se_register_entity_kinds]
  risk high

  verify property "behavior kind has testable=true in manifest"
  verify unit "behavior accepts all 9 standard verify kinds"

}

invariant se_invariant_testability "Invariant Testability" {
  guarantee """
    Invariant entities MUST have testable=true. They MUST accept verify
    kinds: property, unit, mutation. Invariants represent guarantees
    that can be verified via property-based testing, direct unit tests,
    or mutation testing of contract-derived mutants (RES-25).
  """
  enforced_by [se_register_entity_kinds]
  risk medium

  verify property "invariant kind has testable=true in manifest"
  verify unit "invariant accepts property, unit, and mutation verify kinds"

}

invariant se_event_testability "Event Testability" {
  guarantee """
    Event entities MUST have testable=true. They MUST accept verify
    kinds: integration, deadlock_free, liveness. Events represent
    asynchronous communication that requires integration-level testing
    and formal concurrency verification.
  """
  enforced_by [se_register_entity_kinds]
  risk medium

  verify property "event kind has testable=true in manifest"
  verify unit "event accepts integration, deadlock_free, liveness verify kinds"

}

invariant se_feature_non_testable "Feature, Type, Port Non-Testable" {
  guarantee """
    Feature, type, and port entities MUST have testable=false. Features
    are grouping constructs (tested through their behaviors). Types are
    data shape declarations. Ports are interface contracts. None of
    these directly produce test obligations.
  """
  enforced_by [se_register_entity_kinds]
  risk medium

  verify property "feature kind has testable=false"
  verify property "type kind has testable=false"
  verify property "port kind has testable=false"

}

invariant se_edge_consistency "Edge-Field Mapping Consistency" {
  guarantee """
    Every field definition with an edge mapping MUST have a corresponding
    edgeType declaration in the manifest. The field's targetKind MUST
    reference an entity kind declared in this manifest or a peer
    dependency. No orphan edge mappings MUST exist.
  """
  enforced_by [se_register_edge_types, se_register_field_definitions]
  risk medium

  verify property "every field edge mapping has a corresponding edgeType"
  verify unit "orphan edge mapping detected and reported"

}

invariant se_refinement_dag "Refinement DAG" {
  guarantee """
    The refines edges between behaviors MUST form a directed acyclic
    graph (DAG). Cycles in refinement chains MUST produce E032 error
    diagnostics. This ensures well-founded refinement from abstract
    specifications to concrete implementations.
  """
  enforced_by [se_build_refinement_chain]
  risk high

  verify property "refines edges form a DAG with no cycles"
  verify unit "cycle in refinement chain produces E032"

}

invariant se_formal_contract_consistency "Formal Contract Consistency" {
  guarantee """
    In an ensures block, condition descriptions MUST NOT reference
    identifiers that are absent from the corresponding requires block
    or the entity's own scope (types, ports, invariants fields). This
    prevents postconditions from depending on undefined state.
  """
  enforced_by [se_validate_contract_consistency]
  risk medium

  verify unit "ensures referencing unknown identifier detected"
  verify unit "ensures referencing requires identifier passes"

}

invariant se_event_trigger_validity "Event Trigger Validity" {
  guarantee """
    An event's trigger field MUST reference a behavior entity. Events
    are caused by behaviors — they cannot trigger themselves or reference
    non-behavior entity kinds. Invalid trigger references MUST produce
    E006 error diagnostics.
  """
  enforced_by [se_validate_event_triggers]
  risk high

  verify unit "event trigger referencing behavior passes"
  verify unit "event trigger referencing non-behavior produces E006"

}

invariant se_port_direction_constraint "Port Direction Constraint" {
  guarantee """
    A port's direction field MUST be one of: inbound, outbound. No
    other values are permitted. Missing direction MUST produce an
    error diagnostic. This enforces the hexagonal architecture
    boundary model.
  """
  enforced_by [se_validate_entity_fields]
  risk low

  verify unit "port with direction inbound passes"
  verify unit "port with direction outbound passes"
  verify unit "port with invalid direction produces error"

}
