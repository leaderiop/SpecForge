// @specforge/compliance extension manifest declaration

use types/zero-entity-core

behavior ce_declare_manifest "Declare @specforge/compliance Manifest" {
  category command
  types [ManifestV2, ManifestEntityKind, ManifestEdgeType]

  contract """
    The @specforge/compliance extension MUST declare a v2 manifest with name
    "@specforge/compliance", manifestVersion 2. The manifest MUST declare
    exactly 4 entity kinds (regulation, control, evidence, audit), 4 edge
    types (Governs, ImplementedBy, ProvidedBy, Audits), and all associated
    validation rules. The wasmPath MUST point to the compiled Wasm module.
  """

  requires {
    valid_manifest_version   "manifestVersion == 2"
    valid_extension_name     "name == '@specforge/compliance'"
    wasm_module_exists       "wasmPath points to a compiled Wasm binary"
  }

  ensures {
    four_entity_kinds        "entityKinds.length == 4"
    four_edge_types          "edgeTypes.length == 4"
    all_kinds_named          "entityKinds contains regulation, control, evidence, audit"
    all_edges_named          "edgeTypes contains Governs, ImplementedBy, ProvidedBy, Audits"
    contributes_declared     "contributes declares entities=true, validators=true, renderers=true"
  }

  verify unit "manifest name is @specforge/compliance"
  verify unit "manifest declares exactly 4 entity kinds"
  verify unit "manifest declares exactly 4 edge types"
  verify unit "manifest version is 2"
  verify unit "contributes declares entities, validators, and renderers"

}

invariant ce_manifest_four_entity_kinds "Four Entity Kinds" {
  guarantee """
    The @specforge/compliance manifest MUST declare exactly 4 entity kinds:
    regulation, control, evidence, audit. These represent the minimal
    complete set for regulatory compliance specification.
  """
  enforced_by [ce_declare_manifest]
  risk high

  verify property "manifest entityKinds array has exactly 4 entries"

}

invariant ce_manifest_four_edge_types "Four Edge Types" {
  guarantee """
    The @specforge/compliance manifest MUST declare exactly 4 edge types:
    Governs (regulation -> control), ImplementedBy (control -> evidence),
    ProvidedBy (evidence -> audit), Audits (audit -> regulation). These
    edges model the compliance traceability chain.
  """
  enforced_by [ce_declare_manifest]
  risk medium

  verify property "manifest edgeTypes array has exactly 4 entries"

}
