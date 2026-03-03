// Extension features — plugins, providers, generators

use behaviors/extensions
use behaviors/wasm

feature plugin_management "Plugin Management" {
  behaviors [load_plugin_manifests, register_plugin_entity_types, remove_plugin, list_installed_plugins, custom_entity_types_via_define, load_wasm_module, initialize_wasm_package, validate_package_peer_dependencies, topological_sort_packages]

  problem """
    Not every project needs all 16 entity types. Teams need to install
    only the plugins they use, and the compiler must gracefully handle
    references to uninstalled plugins.
  """

  solution """
    Contribution-based package model: specforge add/remove installs Wasm
    plugin binaries, specforge plugins lists them. Plugins are .wasm
    modules loaded via Extism with AOT caching. Peer dependencies are
    validated at startup and plugins are initialized in topological order.
    Cross-plugin references use soft resolution (I004 if plugin missing).
    Custom entity types via define blocks for domain-specific needs.
  """
}

feature provider_based_ref_validation "Provider-Based Ref Validation" {
  behaviors [load_provider_configurations, validate_provider_refs, list_configured_providers, validate_ref_target_format, validate_provider_kinds]

  problem """
    External references (GitHub issues, Jira tickets, Figma frames)
    need validation — typos in issue numbers should be caught at
    compile time, not discovered during a review.
  """

  solution """
    Provider model: providers are Wasm modules that register ref schemes
    and kinds. The compiler delegates validation to the appropriate
    provider via the Wasm runtime. Providers use the specforge.http_get
    host function for network validation. Multiple aliased instances of
    the same provider are supported.
  """
}

feature generator_plugin_protocol "Generator Plugin Protocol" {
  behaviors [execute_generator_plugins, validate_generator_configuration, call_package_generators]

  problem """
    The built-in code generators cannot cover every language and framework.
    Community generators need a stable interface to read the spec graph
    and produce output files.
  """

  solution """
    Wasm host function protocol: generator plugins compile to .wasm
    binaries and access the graph via the specforge.query_graph host
    function. Generated files are emitted via specforge.emit_file,
    diagnostics via specforge.emit_diagnostic. Any language with a
    Wasm compilation target can implement a generator. Sandboxed
    execution with no direct filesystem or network access.
  """
}
