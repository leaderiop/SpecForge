// @specforge/software extension manifest declaration

use types/zero-entity-core
use extensions/software/types

behavior se_declare_manifest "Declare @specforge/software Manifest" {
  category command
  types [ManifestV2, ManifestEntityKind, ManifestEdgeType]

  contract """
    The @specforge/software extension MUST declare a v2 manifest with name
    "@specforge/software", manifestVersion 2. The manifest MUST declare
    exactly 6 entity kinds (behavior, invariant, feature, event, type,
    port), 9 edge types (References, Implements, Produces, Consumes,
    UsesType, UsesPort, Enforces, Imports, LinksTo), and all associated
    validation rules. The wasmPath MUST point to the compiled Wasm module.
  """

  requires {
    valid_manifest_version   "manifestVersion == 2"
    valid_extension_name     "name == '@specforge/software'"
    wasm_module_exists       "wasmPath points to a compiled Wasm binary"
  }

  ensures {
    six_entity_kinds         "entityKinds.length == 6"
    nine_edge_types          "edgeTypes.length == 9"
    all_kinds_named          "entityKinds contains behavior, invariant, feature, event, type, port"
    all_edges_named          "edgeTypes contains References, Implements, Produces, Consumes, UsesType, UsesPort, Enforces, Imports, LinksTo"
    contributes_declared     "contributes declares entities=true and validators=true"
    no_peer_deps             "peer_dependencies is empty (software has no peer deps)"
  }

  verify unit "manifest name is @specforge/software"
  verify unit "manifest declares exactly 6 entity kinds"
  verify unit "manifest declares exactly 9 edge types"
  verify unit "manifest version is 2"
  verify unit "contributes declares entities and validators"
  verify unit "peer_dependencies is empty"

}

invariant se_manifest_six_entity_kinds "Six Entity Kinds" {
  guarantee """
    The @specforge/software manifest MUST declare exactly 6 entity kinds:
    behavior, invariant, feature, event, type, port. No more, no fewer.
    This count was validated by a 10-expert panel (RES-27) and represents
    the minimal complete set for software engineering specification.
  """
  enforced_by [se_declare_manifest]
  risk high

  verify property "manifest entityKinds array has exactly 6 entries"

}

invariant se_manifest_nine_edge_types "Nine Edge Types" {
  guarantee """
    The @specforge/software manifest MUST declare exactly 9 edge types:
    References, Implements, Produces, Consumes, UsesType, UsesPort,
    Enforces, Imports, LinksTo. These edges model all relationships
    between the 6 entity kinds in the software engineering domain.
  """
  enforced_by [se_declare_manifest]
  risk medium

  verify property "manifest edgeTypes array has exactly 9 entries"

}
