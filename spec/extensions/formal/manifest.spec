// @specforge/formal extension manifest declaration
//
// @specforge/formal contributes 6 entity kinds (condition, property, axiom,
// protocol, refinement, process) and 11 edge types. It enhances @specforge/software
// entity kinds via entity_enhancements and contributes 4 compiler passes,
// 3 feature flags, and 4 verify kinds. Formal analysis warnings require
// warning_level=strict.

use "extensions/formal/features"
use "extensions/formal/types"
use "types/zero-entity-core"

behavior fa_declare_manifest "Declare @specforge/formal Manifest" {
  category command
  types    [ManifestV2, CompilerPassDeclaration, FeatureFlagDeclaration, FormalCondition, FormalProperty, FormalAxiom, FormalProtocol, FormalRefinement, FormalProcess]
  contract """
    The @specforge/formal extension MUST declare a v2 manifest with name
    "@specforge/formal", manifestVersion 2. The manifest MUST declare
    6 entity kinds and 11 edge types.

    Entity kind declarations (all testable=false, supports_verify=false):
    - condition: named, reusable precondition/postcondition/frame invariant
      Shape: { description string, references EntityId[] @optional }
    - property: temporal/behavioral assertion (safety/liveness/fairness)
      Shape: { description string, kind PropertyKind, references EntityId[] @optional }
    - axiom: assumed-true foundation (no proof required, no coverage tracking item)
      Shape: { description string, justification string @optional, references EntityId[] @optional }
    - protocol: shared synchronization contract across events
      Shape: { description string, ordering string[] @optional, timeout string @optional, delivery DeliverySemantics @optional, references EntityId[] @optional }
    - refinement: abstract->concrete behavior mapping as first-class graph node
      Shape: { description string, abstract_id EntityId, concrete_id EntityId, conditions ConditionDelta @optional, status RefinementStatus @optional, references EntityId[] @optional }
    - process: CSP-style communicating process with alphabet and composition
      Shape: { description string, alphabet EntityId[] @optional, states ProcessState[] @optional, composition CompositionOperator @optional, references EntityId[] @optional }

    Conditions support dual-mode usage:
    - Inline:    requires { name "description" }
    - Reference: requires [condition_id]
    Both modes can be combined on a single entity.

    Edge types (11 total):
    - RequiresCondition:    behavior -> condition (precondition link)
    - EnsuresCondition:     behavior -> condition (postcondition link)
    - MaintainsCondition:   behavior|invariant -> condition (frame invariant link)
    - AssumedBy:            condition -> axiom (axiom dependency — "this condition rests on this axiom")
    - Satisfies:            behavior -> property (temporal property satisfaction)
    - FollowsProtocol:      event -> protocol (sync contract reference)
    - PropertyDependsOn:    property -> condition (property-condition dependency)
    - RefinesTo:            refinement -> behavior (abstract-to-concrete mapping)
    - RefinementChainLink:  refinement -> refinement (multi-level refinement)
    - ParticipatesIn:       event -> process (event membership in process alphabet)
    - ProcessComposition:   process -> process (parallel/sequential/choice composition)

    Entity enhancements add formal fields to @specforge/software entities:
    - behavior: requires, ensures, maintains, abstract, refines, assumes, satisfies, refinement
    - invariant: maintains
    - event: sync, follows_protocol, process
    - port.methods: requires, ensures

    The requires/ensures/maintains fields accept BOTH inline blocks
    AND condition entity reference lists (dual-mode). Inline blocks
    produce ConditionEntry nodes; reference lists produce edge links
    to condition entities.

    Compiler passes: condition_check, layering_verify, event_graph_analyze,
    coverage_tracking (with proper dependency ordering).

    Verify kinds contributed: contract, refinement, deadlock_free, liveness.

    Feature flags: conditions (default true, no deps), layering (default
    true, requires conditions), concurrency (default true, no deps).

    Warning level requirement: all formal warnings (W028-W040, W058-W074)
    require warning_level=strict. This prevents overwhelming new users.

    Safety-critical scope: @specforge/formal is intended for projects
    that benefit from structural analysis — safety-critical systems,
    distributed architectures, and formally-inclined teams. It is NOT
    required for basic SpecForge usage.
  """
  requires {
    valid_manifest_version   "manifestVersion == 2"
    valid_extension_name     "name == '@specforge/formal'"
    wasm_module_exists       "wasmPath points to a compiled Wasm binary"
  }
  ensures  {
    six_entity_kinds         "entityKinds contains condition, property, axiom, protocol, refinement, process (all testable=false, supports_verify=false)"
    eleven_edge_types        "edgeTypes contains RequiresCondition, EnsuresCondition, MaintainsCondition, AssumedBy, Satisfies, FollowsProtocol, PropertyDependsOn, RefinesTo, RefinementChainLink, ParticipatesIn, ProcessComposition"
    requires_condition_edge  "RequiresCondition: source=behavior, target=condition"
    ensures_condition_edge   "EnsuresCondition: source=behavior, target=condition"
    maintains_condition_edge "MaintainsCondition: source=behavior|invariant, target=condition"
    assumed_by_edge          "AssumedBy: source=condition, target=axiom"
    satisfies_edge           "Satisfies: source=behavior, target=property"
    follows_protocol_edge    "FollowsProtocol: source=event, target=protocol"
    property_depends_on_edge "PropertyDependsOn: source=property, target=condition"
    refines_to_edge          "RefinesTo: source=refinement, target=behavior"
    refinement_chain_link_edge "RefinementChainLink: source=refinement, target=refinement"
    participates_in_edge     "ParticipatesIn: source=event, target=process"
    process_composition_edge "ProcessComposition: source=process, target=process"
    four_passes              "passes contains condition_check, layering_verify, event_graph_analyze, coverage_tracking"
    pass_ordering            "layering_verify depends_on condition_check; event_graph_analyze depends_on layering_verify; coverage_tracking depends_on event_graph_analyze"
    three_feature_flags      "feature_flags contains conditions, layering, concurrency"
    flag_dependencies        "layering requires conditions"
    dual_mode_fields         "requires/ensures/maintains fields accept both inline blocks AND condition entity reference lists"
    enhancements_declared    "entity_enhancements add requires/ensures/maintains/abstract/refines/assumes/satisfies/refinement to behavior, maintains to invariant, sync/follows_protocol/process to event, requires/ensures to port.methods"
    verify_kinds_declared    "verify_kinds contains contract, refinement, deadlock_free, liveness"
    peer_dep_software        "peer_dependencies contains @specforge/software ^1.0 (required)"
    warning_level_strict     "all formal warnings require warning_level=strict"
    sandbox_restricted       "sandbox_policy declares network_access=false, file_system_access=read-only"
    host_api_declared        "host_api_version is 1.0.0"
  }

  features [fa_progressive_warnings]

  verify unit "manifest name is @specforge/formal"
  verify unit "manifest declares 6 entity kinds (condition, property, axiom, protocol, refinement, process)"
  verify unit "manifest declares 11 edge types"
  verify unit "all entity kinds have testable=false"
  verify unit "RequiresCondition edge: behavior -> condition"
  verify unit "EnsuresCondition edge: behavior -> condition"
  verify unit "MaintainsCondition edge: behavior|invariant -> condition"
  verify unit "AssumedBy edge: condition -> axiom"
  verify unit "Satisfies edge: behavior -> property"
  verify unit "FollowsProtocol edge: event -> protocol"
  verify unit "PropertyDependsOn edge: property -> condition"
  verify unit "RefinesTo edge: refinement -> behavior"
  verify unit "RefinementChainLink edge: refinement -> refinement"
  verify unit "ParticipatesIn edge: event -> process"
  verify unit "ProcessComposition edge: process -> process"
  verify unit "manifest declares 4 passes in dependency order"
  verify unit "manifest declares 3 feature flags"
  verify unit "layering flag requires conditions flag"
  verify unit "dual-mode requires/ensures/maintains fields"
  verify unit "entity_enhancements add formal fields to software entities"
  verify unit "entity_enhancements add refinement field to behavior"
  verify unit "entity_enhancements add process field to event"
  verify unit "verify_kinds contains contract, refinement, deadlock_free, liveness"
  verify unit "peer_dependencies requires @specforge/software"
  verify unit "formal warnings require warning_level=strict"
}
