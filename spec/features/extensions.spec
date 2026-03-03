// Extension features — plugins, providers, generators

use behaviors/extensions

feature plugin_management "Plugin Management" {
  behaviors [load_plugin_manifests, register_plugin_prefixes, remove_plugin, list_installed_plugins, custom_entity_types_via_define]

  problem """
    Not every project needs all 16 entity types. Teams need to install
    only the plugins they use, and the compiler must gracefully handle
    references to uninstalled plugins.
  """

  solution """
    Terraform-style plugin model: specforge add/remove installs plugins,
    specforge plugins lists them. Cross-plugin references use soft
    resolution (I004 if plugin missing). Custom entity types via
    define blocks for domain-specific needs.
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
    Provider model: providers register ref schemes and kinds. The
    compiler delegates validation to the appropriate provider.
    Multiple aliased instances of the same provider are supported.
  """
}

feature generator_plugin_protocol "Generator Plugin Protocol" {
  behaviors [execute_generator_plugins, validate_generator_configuration]

  problem """
    The built-in code generators cannot cover every language and framework.
    Community generators need a stable interface to read the spec graph
    and produce output files.
  """

  solution """
    Subprocess I/O protocol: compiler pipes JSON graph to stdin,
    plugin writes files to stdout, diagnostics to stderr. Any language
    can implement a generator. No FFI or shared memory required.
  """
}
