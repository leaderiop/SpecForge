// Extension features — CLI extension management, provider-based ref validation,
// and user-defined entity types.
//
// The extension surface is split across three feature files:
//   - features/extensions.spec  — CLI management, providers, registry (define blocks → see features/zero-entity-core.spec)
//   - features/wasm.spec        — Wasm runtime, host functions, authoring
//   - features/zero-entity-core.spec — manifest schema, registries, LSP, bootstrap

use behaviors/extensions
use invariants/extensions

// CLI surface: specforge add is in features/project-init.spec::project_initialization
// (add_extension_to_existing_project behavior). specforge remove and specforge extensions
// are managed here. See also features/wasm.spec for install/uninstall lifecycle.
feature extension_management "Extension Management" {
  behaviors [load_extension_manifests, register_extension_entity_types, remove_extension, list_installed_extensions]

  problem """
    The core compiler has zero built-in entity types. Teams need to
    install extensions that provide the entity kinds they use, and the
    compiler must gracefully handle references to uninstalled extensions.
  """

  solution """
    Contribution-based extension model: specforge remove manages
    extension removal from specforge.json (specforge add is part of
    the project initialization feature). specforge extensions lists
    installed extensions. Extension manifests are loaded and their
    entity types registered into the KindRegistry. Cross-extension
    references use soft resolution (I004 if extension missing).
    Wasm runtime loading, peer dependencies, and topological ordering
    are specified in features/wasm.spec.
  """
}

feature provider_based_ref_validation "Provider-Based Ref Validation" {
  behaviors [load_provider_configurations, register_provider_schemes, validate_provider_refs, list_configured_providers, validate_ref_target_format, validate_provider_kinds]

  problem """
    External references (issue trackers, design tools, project management
    systems) need validation — typos in reference identifiers should be
    caught at compile time, not discovered during a review.
  """

  solution """
    Provider model: providers are Wasm modules that register ref schemes
    and kinds. The compiler delegates validation to the appropriate
    provider via the Wasm runtime. Providers use the specforge.http_get
    host function for network validation. Multiple aliased instances of
    the same provider are supported. Scheme-to-provider routing is managed
    via the SchemeRegistryEntry set.
  """
}

feature extension_registry "Extension Registry" {
  // Cross-feature: registry_api_openness invariant also references
  // publish_schema_specification from features/output.spec — the open registry
  // API schema is part of the Graph Protocol publishing surface.
  behaviors [resolve_registry_source, search_registry, publish_to_registry, verify_registry_integrity, configure_registries, generate_keyword_extension_index]

  problem """
    Extensions are currently resolved from local paths or direct URLs.
    There is no registry API for discovering, searching, or publishing
    extensions. Without a registry, the H2 ecosystem cannot grow —
    extension authors have no standard way to share their work and
    users have no way to discover available extensions.
  """

  solution """
    Registry API with configurable endpoints and an open, published API
    specification (per P6: "the standard is the moat"). The registry API
    schema MUST be published as an open specification so that third-party
    registries can implement it — SpecForge MUST NOT be the only possible
    registry host. specforge.json declares registry configs with scope-based
    routing. specforge search queries registries with contribution type
    filtering. specforge extension publish uploads validated .wasm binaries
    with SHA256 integrity. Downloaded binaries are verified against declared
    hashes and assigned trust levels (verified, community, local, git)
    recorded in specforge.lock. First-use MUST NOT require network access —
    local path and git sources work offline; network registries are opt-in
    configuration.
  """
}

feature registry_authentication "Registry Authentication" {
  behaviors [authenticate_registry_request, retry_registry_request, validate_registry_credentials, support_private_registries, logout_registry]

  problem """
    Extension registries may require authentication for private or enterprise
    extensions. There is no mechanism to configure credentials, authenticate
    requests, or manage trust levels for private registries. Without
    authentication, organizations cannot use private extension repositories.
  """

  solution """
    Registry credential management via specforge registry login and
    specforge registry logout. Credentials are stored as environment variable
    references or token file paths — never raw tokens. The system authenticates
    before fetching from configured registries, respects scope filters, assigns
    appropriate trust levels, and retries on authentication failures. Logout
    securely removes stored credentials for a given registry. Error messages
    never leak credential details.
  """
}
