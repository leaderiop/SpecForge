// Extension-specific invariants

use "behaviors/extensions"
use "behaviors/init"
use "behaviors/wasm-authoring"
use "behaviors/wasm-lifecycle"
use "behaviors/output-schema"
invariant offline_first_extension_resolution "Offline-First Extension Resolution" {
  guarantee """
    Extension resolution MUST work fully offline when cached manifests
    and .wasm binaries are available locally. The compiler MUST never
    require network access to load already-installed extensions. Network
    registries are opt-in; local path and git sources MUST always work
    without connectivity. When a registry is unreachable, the system
    MUST fall back to the cached manifest in specforge.lock and the
    locally stored .wasm binary.
  """
  enforced_by [load_extension_manifests, configure_registries, resolve_registry_source, verify_registry_integrity, install_wasm_extension, authenticate_registry_request, search_registry, discover_extensions]
  risk high

  verify property "installed extensions load without network access"
  verify property "unreachable registry falls back to cached manifest"
}

invariant registry_api_openness "Registry API Openness" {
  guarantee """
    The extension registry API schema MUST be published as an open
    specification so that third-party registries can implement it.
    SpecForge MUST NOT be the only possible registry host.
  """
  enforced_by [configure_registries, publish_schema_specification, publish_wasm_extension]
  risk high

  verify property "registry API schema is published as open specification"
  verify unit "third-party registry implementing the API schema is accepted"

}

invariant authentication_never_gates_core_use "Authentication Never Gates Core Use" {
  guarantee """
    No SpecForge command MUST require registry authentication to function
    with the default public registry, local paths, or git sources.
    Authentication is exclusively for private and enterprise registries.
    The first-use experience (init, check, export) MUST complete without
    credentials. This is a P8 (seconds to value) structural guarantee.
  """
  enforced_by [
    authenticate_registry_request, retry_registry_request,
    validate_registry_credentials, support_private_registries,
    logout_registry, configure_registries, scaffold_new_project,
    non_interactive_init, graceful_zero_extension_init,
  ]
  risk high
  verify unit "specforge init succeeds without any registry authentication"
  verify unit "specforge check succeeds without credentials"
  verify unit "default public registry accessible without authentication"
  verify integration "full init-check-export cycle completes without credentials"
}
