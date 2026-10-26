// @specforge/governance extension manifest declaration

use "extensions/governance/types"
use "types/zero-entity-core"

behavior ge_declare_manifest "Declare @specforge/governance Manifest" {
  features [ge_core_entity_kinds]
  category command
  types    [ExtensionDeclaration, EntityKindDescriptor, EdgeTypeDescriptor]
  contract """
    The @specforge/governance extension MUST declare itself with name
    "@specforge/governance". Its declaration MUST declare
    exactly 4 entity kinds (decision, constraint, failure_mode, invariant),
    its 11 edge types (DecisionProtectsInvariant, ConstraintEnforcedByBehavior,
    DecisionSupersedesDecision, ConstraintConstrainsBehavior,
    ConstraintProtectsInvariant, FailureModeTargetsInvariant,
    ConstraintGovernsFeature, DecisionAffectsFeature,
    FailureModeThreatensFeature, DecisionImposesConstraint,
    FailureModeAffectsBehavior), and W003 over invariants (ADR 0280).
  """
  requires {
    supported_protocol   "the handshake's protocol major version is the host's"
    valid_extension_name "name == '@specforge/governance'"
  }
  ensures {
    four_entity_kinds   "entityKinds.length == 4"
    eleven_edge_types   "edgeTypes.length == 11"
    all_kinds_named     "entityKinds contains decision, constraint, failure_mode, invariant"
    declared_categories "the declaration declares entities, edges and validation_rules"
    optional_peer_dep   "peer_dependencies contains @specforge/software ^1.0 (optional, for ConstrainsBehavior cross-extension edge targeting behavior kind)"
    no_sandbox_policy   "sandbox_policy is null — governance declares no sandbox policy and runs under the host's ceiling"
  }
  verify unit "manifest name is @specforge/governance"
  verify unit "manifest declares exactly 4 entity kinds"
  verify unit "manifest declares exactly 11 edge types"
  verify unit "the handshake's protocol major is the host's"
  verify unit "the declaration declares entities, edges and validation_rules"
  verify unit "peer_dependencies includes optional @specforge/software"
  verify unit "sandbox_policy is null"
  verify unit "the declaration declares invariant with its guarantee and its risk levels, and W003 over it"
  verify integration "a project that loads governance and not software declares invariants, and W003 reports the one nothing references"
  verify integration "governance and formal load together without software"
  verify integration "a project that loads software, governance and formal checks its invariants as before the move"
}

invariant ge_manifest_four_entity_kinds "Four Entity Kinds" {
  guarantee """
    The @specforge/governance manifest MUST declare exactly 4 entity kinds:
    decision, constraint, failure_mode, invariant (ADR 0280).
  """
  risk      high
  verify property "manifest entityKinds array has exactly 4 entries"
}

invariant ge_manifest_four_edge_types "Four Edge Types" {
  guarantee """
    The @specforge/governance manifest MUST declare exactly 4 edge types:
    DecisionInvariant (decision->invariant), ConstrainsBehavior
    (constraint->behavior, cross-extension), ProtectsInvariant
    (constraint->invariant), FailureModeInvariant (failure_mode->invariant).
  """
  risk      medium
  verify property "manifest edgeTypes array has exactly 4 entries"
}
