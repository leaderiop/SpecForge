// Extension behaviors — plugins, providers, generators

use invariants/core
use types/config
use types/errors
use ports/outbound

behavior load_plugin_manifests "Load Plugin Manifests" {
  types      [PluginManifest, CompilerConfig, PluginError, ProviderSettings, ConfigEntry]

  contract """
    At startup, the compiler MUST read the plugins list from specforge.spec
    and load each plugin's manifest. The manifest MUST declare the entity
    types, ID prefixes, edge types, and validation rules the plugin adds.
    Missing plugins MUST produce a diagnostic, not a crash.
  """

  verify unit "installed plugin manifest is loaded"
  verify unit "missing plugin produces diagnostic"
  verify unit "manifest declares prefixes and validations"
}

behavior register_plugin_prefixes "Register Plugin Prefixes" {
  invariants [reference_resolution_completeness]
  types      [PluginManifest]

  contract """
    After loading manifests, the compiler MUST register each plugin's
    ID prefixes in the prefix registry. When resolving references, the
    registry MUST be consulted to determine which module owns each prefix
    and whether soft resolution applies.
  """

  verify unit "plugin prefixes are registered"
  verify unit "unregistered prefix triggers soft resolution"
}

behavior load_provider_configurations "Load Provider Configurations" {
  types      [ProviderConfig, CompilerConfig]

  contract """
    The compiler MUST parse provider blocks from specforge.spec and
    create provider instances with their configured settings. Multiple
    instances of the same provider with different aliases MUST be
    supported.
  """

  verify unit "single provider instance is created"
  verify unit "multiple aliased instances are created"
  verify unit "provider config settings are passed through"
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
}

behavior execute_generator_plugins "Execute Generator Plugins" {
  types      [PluginManifest, GenConfig]
  ports      [FileSystem]

  contract """
    When specforge gen invokes a generator plugin, the compiler MUST
    serialize the graph to JSON, pipe it to the plugin's stdin, and
    collect generated files from stdout. Plugin stderr MUST be captured
    and forwarded as diagnostics.
  """

  verify unit "graph JSON is piped to plugin stdin"
  verify unit "generated files are collected from stdout"
  verify unit "plugin stderr becomes diagnostics"
}

behavior remove_plugin "Remove Plugin" {
  types      [CompilerConfig]
  ports      [FileSystem]

  contract """
    When specforge remove @specforge/plugin is invoked, the system MUST
    remove the plugin from the plugins list in specforge.spec. Existing
    .spec files using plugin entities MUST NOT be modified — they become
    soft references (I004).
  """

  verify unit "plugin is removed from plugins list"
  verify unit "existing plugin entities become soft references"
}

behavior list_installed_plugins "List Installed Plugins" {
  types      [PluginManifest]

  contract """
    When specforge plugins is invoked, the system MUST list all installed
    plugins with their name, version, entity count, and registered prefixes.
  """

  verify unit "list shows all installed plugins"
  verify unit "list includes entity counts and prefixes"
}

behavior custom_entity_types_via_define "Custom Entity Types via Define" {
  types      [CompilerConfig]

  contract """
    When a define block exists in specforge.spec, the compiler MUST
    register the custom entity type with its id_prefix, required fields,
    optional fields, and reference targets. Custom entities MUST participate
    in reference resolution and orphan detection like built-in entities.
  """

  verify unit "custom entity type is registered"
  verify unit "custom entity participates in reference resolution"
  verify unit "custom entity has orphan detection"
}

behavior validate_generator_configuration "Validate Generator Configuration" {
  types      [GenConfig, CompilerConfig]

  contract """
    When a gen block in specforge.spec references a generator, the compiler
    MUST verify the generator is installed and executable. Missing generators
    MUST produce a diagnostic. Invalid gen config fields MUST produce a
    diagnostic with the field name and expected format.
  """

  verify unit "installed generator passes validation"
  verify unit "missing generator produces diagnostic"
  verify unit "invalid gen config field produces diagnostic"
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
}

behavior validate_provider_kinds "Validate Provider Kinds" {
  types      [Diagnostic]

  contract """
    When a ref uses a known scheme but an unregistered kind, the validator
    MUST produce an E012 diagnostic listing the valid kinds for that scheme.
  """

  verify unit "valid scheme and kind passes"
  verify unit "valid scheme with unknown kind produces E012"
}
