// @specforge/coverage extension manifest declaration

use "extensions/coverage/types"
use "types/zero-entity-core"

behavior cv_declare_manifest "Declare @specforge/coverage Manifest" {
  category command
  types    [ManifestV2, ManifestEntityKind]
  contract """
    The @specforge/coverage extension MUST declare a v2 manifest with name
    "@specforge/coverage", manifestVersion 2. The manifest MUST declare
    contributes collectors=true. The extension declares zero entity kinds
    and zero edge types — it enhances existing entities via
    entity_enhancements and consumes specforge-report.json artifacts.
    The manifest MUST declare an optional peer_dependency on
    @specforge/software ^1.0 (for behavior coverage mapping). The
    wasmPath MUST point to the compiled Wasm module. The collect command
    is language-agnostic: it consumes specforge-report.json, not
    language-specific test output directly.
  """
  requires {
    valid_manifest_version   "manifestVersion == 2"
    valid_extension_name     "name == '@specforge/coverage'"
    wasm_module_exists       "wasmPath points to a compiled Wasm binary"
  }
  ensures  {
    zero_entity_kinds        "entityKinds is empty (coverage enhances, does not declare)"
    zero_edge_types          "edgeTypes is empty"
    contributes_collectors   "contributes declares collectors=true"
    optional_peer_dep        "peer_dependencies contains @specforge/software ^1.0 (optional)"
    sandbox_restricted       "sandbox_policy declares network_access: false, file_system_access: read-only"
    host_api_declared        "host_api_version is 1.0.0"
    starter_template_set     "starter_template is templates/coverage.spec"
  }

  verify unit "manifest name is @specforge/coverage"
  verify unit "manifest declares zero entity kinds"
  verify unit "manifest declares zero edge types"
  verify unit "manifest version is 2"
  verify unit "contributes declares collectors"
  verify unit "peer_dependencies includes optional @specforge/software"
  verify unit "sandbox_policy restricts network and filesystem"
  verify unit "host_api_version is 1.0.0"
  verify unit "starter_template points to templates/coverage.spec"
}

invariant cv_language_agnostic_collection "Language-Agnostic Collection" {
  guarantee   """
    The @specforge/coverage extension MUST consume specforge-report.json
    as its primary input format. Language-specific parsing (Rust JUnit XML,
    Python pytest, Go test output) is an internal collector behavior that
    transforms language output into the standard SpecforgeReport format.
    The CLI command specforge collect MUST NOT require a language argument —
    it reads specforge-report.json by default. Language-specific parsers
    are opt-in via --parser flag or collector_contributions.
  """
  enforced_by [cv_declare_manifest]
  risk        high

  verify property "collect command defaults to specforge-report.json input"
  verify property "language-specific parsing is opt-in, not required"
}
