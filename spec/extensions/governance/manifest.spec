// @specforge/governance extension manifest declaration

use "extensions/governance/types"
use "types/zero-entity-core"

behavior ge_declare_manifest "Declare @specforge/governance Manifest" {
  category command
  types    [ManifestV2, ManifestEntityKind, ManifestEdgeType]
  contract """
    The @specforge/governance extension MUST declare a v2 manifest with name
    "@specforge/governance", manifestVersion 2. The manifest MUST declare
    exactly 3 entity kinds (decision, constraint, failure_mode), 4 edge types
    (DecisionInvariant, ConstrainsBehavior, ProtectsInvariant,
    FailureModeInvariant), and all associated validation rules.
  """
  requires {
    valid_manifest_version   "manifestVersion == 2"
    valid_extension_name     "name == '@specforge/governance'"
  }
  ensures  {
    three_entity_kinds       "entityKinds.length == 3"
    four_edge_types          "edgeTypes.length == 4"
    all_kinds_named          "entityKinds contains decision, constraint, failure_mode"
    all_edges_named          "edgeTypes contains DecisionInvariant, ConstrainsBehavior, ProtectsInvariant, FailureModeInvariant"
    contributes_declared     "contributes declares entities=true and validators=true"
    optional_peer_dep        "peer_dependencies contains @specforge/software ^1.0 (optional, for ConstrainsBehavior cross-extension edge targeting behavior kind)"
    sandbox_restricted       "sandbox_policy declares network_access=false, file_system_access=read-only, max_memory_mb=256, max_execution_ms=5000"
    host_api_declared        "host_api_version is 1.0.0"
    starter_tmpl_declared    "starter_template is templates/decision.spec"
  }

  verify unit "manifest name is @specforge/governance"
  verify unit "manifest declares exactly 3 entity kinds"
  verify unit "manifest declares exactly 4 edge types"
  verify unit "manifest version is 2"
  verify unit "contributes declares entities and validators"
  verify unit "peer_dependencies includes optional @specforge/software"
  verify unit "sandbox_policy declares no network access and read-only filesystem"
  verify unit "host_api_version is 1.0.0"
  verify unit "starter_template is templates/decision.spec"
}

invariant ge_manifest_three_entity_kinds "Three Entity Kinds" {
  guarantee   """
    The @specforge/governance manifest MUST declare exactly 3 entity kinds:
    decision, constraint, failure_mode. All three are declarative records
    with testable=false and supportsVerify=false.
  """
  enforced_by [ge_declare_manifest]
  risk        high

  verify property "manifest entityKinds array has exactly 3 entries"
}

invariant ge_manifest_four_edge_types "Four Edge Types" {
  guarantee   """
    The @specforge/governance manifest MUST declare exactly 4 edge types:
    DecisionInvariant (decision->invariant), ConstrainsBehavior
    (constraint->behavior, cross-extension), ProtectsInvariant
    (constraint->invariant), FailureModeInvariant (failure_mode->invariant).
  """
  enforced_by [ge_declare_manifest]
  risk        medium

  verify property "manifest edgeTypes array has exactly 4 entries"
}
