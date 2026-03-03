// Extension behaviors — plugins, providers, generators

use invariants/core
use types/config
use types/wasm
use types/errors
use ports/outbound

behavior load_plugin_manifests "Load Plugin Manifests" {
  types      [PackageManifest, CompilerConfig, PackageError, ProviderSettings, ConfigEntry]

  contract """
    At startup, the compiler MUST read the plugins list from specforge.json
    and locate each plugin's Wasm manifest. The manifest MUST declare the
    entity types, edge types, validation rules, wasmPath to
    the .wasm binary, and optional peer dependencies. Missing plugins or
    unloadable .wasm binaries MUST produce a diagnostic, not a crash.
  """

  verify unit "installed plugin manifest is loaded"
  verify unit "missing plugin produces diagnostic"
  verify unit "manifest declares entity types and validations"
  verify unit "manifest includes wasmPath to .wasm binary"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior register_plugin_entity_types "Register Plugin Entity Types" {
  invariants [reference_resolution_completeness]
  types      [PackageManifest]

  contract """
    After loading manifests, the compiler MUST register each plugin's
    entity types. During the initialize() Wasm export, plugins register
    types via the specforge.register_entity host function. When resolving
    references, the registry MUST be consulted to determine which module
    owns each entity type and whether soft resolution applies.
  """

  verify unit "plugin entity types are registered via host function"
  verify unit "unregistered type triggers soft resolution"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior load_provider_configurations "Load Provider Configurations" {
  types      [ProviderConfig, CompilerConfig]

  contract """
    The compiler MUST parse provider blocks from specforge.json and
    create provider instances with their configured settings. Multiple
    instances of the same provider with different aliases MUST be
    supported.
  """

  verify unit "single provider instance is created"
  verify unit "multiple aliased instances are created"
  verify unit "provider config settings are passed through"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior validate_provider_refs "Validate Provider Refs" {
  invariants [reference_resolution_completeness]
  types      [ProviderConfig]
  ports      [RefValidator]

  contract """
    When a ref entity uses a registered scheme, the compiler MUST
    delegate validation to the corresponding provider. The provider
    MUST validate the kind and identifier format. Unknown schemes
    MUST emit I005.
  """

  verify unit "known scheme delegates to provider"
  verify unit "unknown scheme emits I005"
  verify unit "provider validates identifier format"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior execute_generator_plugins "Execute Generator Plugins" {
  types      [PackageManifest, GenConfig]
  ports      [FileSystem]

  contract """
    When specforge gen invokes a generator plugin, the compiler MUST
    call the plugin's generate() Wasm export. The plugin accesses the
    graph via the specforge.query_graph host function and emits files
    via specforge.emit_file. Diagnostics are emitted via
    specforge.emit_diagnostic. Plugin Wasm traps MUST be caught and
    forwarded as PackageError diagnostics.
  """

  verify unit "graph available via query_graph host function"
  verify unit "generated files collected via emit_file host function"
  verify unit "plugin diagnostics forwarded via emit_diagnostic"
  verify unit "Wasm trap produces PackageError"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior remove_plugin "Remove Plugin" {
  types      [CompilerConfig]
  ports      [FileSystem]

  contract """
    When specforge remove @specforge/plugin is invoked, the system MUST
    remove the plugin from the plugins list in specforge.json. Existing
    .spec files using plugin entities MUST NOT be modified — they become
    soft references (I004).
  """

  verify unit "plugin is removed from plugins list"
  verify unit "existing plugin entities become soft references"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior list_installed_plugins "List Installed Plugins" {
  types      [PackageManifest]

  contract """
    When specforge plugins is invoked, the system MUST list all installed
    plugins with their name, version, entity count, and registered entity types.
  """

  verify unit "list shows all installed plugins"
  verify unit "list includes entity counts and entity types"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior custom_entity_types_via_define "Custom Entity Types via Define" {
  types      [CompilerConfig]

  contract """
    When a define block exists in specforge.json, the compiler MUST
    register the custom entity type with its id_prefix, required fields,
    optional fields, and reference targets. Custom entities MUST participate
    in reference resolution and orphan detection like built-in entities.
  """

  verify unit "custom entity type is registered"
  verify unit "custom entity participates in reference resolution"
  verify unit "custom entity has orphan detection"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior validate_generator_configuration "Validate Generator Configuration" {
  types      [GenConfig, CompilerConfig]

  contract """
    When a gen block in specforge.json references a generator, the compiler
    MUST verify the generator's .wasm binary is loadable. Missing or
    corrupt .wasm binaries MUST produce a diagnostic. Invalid gen config
    fields MUST produce a diagnostic with the field name and expected format.
  """

  verify unit "loadable .wasm binary passes validation"
  verify unit "missing .wasm binary produces diagnostic"
  verify unit "invalid gen config field produces diagnostic"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior list_configured_providers "List Configured Providers" {
  types      [ProviderConfig]

  contract """
    When specforge providers is invoked, the system MUST list all configured
    providers with their alias, package, registered schemes, and supported
    kinds. Providers with multiple instances MUST show each alias separately.
  """

  verify unit "list shows all configured providers"
  verify unit "list includes scheme and kind registrations"
  verify unit "multiple aliases shown separately"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior validate_ref_target_format "Validate Ref Target Format" {
  invariants [reference_resolution_completeness]
  types      [Diagnostic]

  contract """
    When a provider is installed, the validator MUST check that ref
    identifiers match the provider's expected pattern. Malformed
    identifiers MUST produce an E011 diagnostic.
  """

  verify unit "valid ref identifier passes"
  verify unit "malformed ref identifier produces E011"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior validate_provider_kinds "Validate Provider Kinds" {
  types      [Diagnostic]

  contract """
    When a ref uses a known scheme but an unregistered kind, the validator
    MUST produce an E012 diagnostic listing the valid kinds for that scheme.
  """

  verify unit "valid scheme and kind passes"
  verify unit "valid scheme with unknown kind produces E012"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}
