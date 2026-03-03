// Wasm/Extism plugin runtime features

use behaviors/wasm

feature wasm_plugin_runtime "Wasm Plugin Runtime" {
  behaviors [
    load_wasm_module, initialize_wasm_plugin, call_wasm_validate, call_wasm_generate,
    provide_host_function_query_graph, provide_host_function_emit_diagnostic,
    provide_host_function_register_entity, provide_host_function_register_edge,
    provide_host_function_emit_file, provide_host_function_http_get,
    enforce_wasm_sandbox, aot_compile_wasm_module, cache_aot_artifacts, warm_wasm_engine_instance,
    validate_wasm_peer_dependencies, topological_sort_plugins,
  ]

  problem """
    Plugins, providers, and generators need a unified, sandboxed runtime
    that works across all platforms without requiring specific language
    runtimes (Node.js, Python, JVM) on the host machine. The runtime
    must support AOT compilation for fast cold starts and warm instances
    for interactive use in LSP/MCP contexts.
  """

  solution """
    Wasm/Extism as the sole plugin runtime. Plugins compile to .wasm
    binaries and communicate with the compiler via host functions
    (specforge.query_graph, specforge.emit_diagnostic, specforge.register_entity,
    specforge.register_edge, specforge.emit_file, specforge.http_get).
    Sandbox enforcement via linear memory limits, fuel metering, and
    domain allowlists. AOT compilation cached in .specforge/cache/
    for CLI cold start; warm engine instances for LSP/MCP.
  """
}

feature wasm_plugin_authoring "Wasm Plugin Authoring" {
  behaviors [scaffold_wasm_plugin_project, build_wasm_plugin, test_wasm_plugin_locally, publish_wasm_plugin]

  problem """
    Plugin authors need a streamlined workflow to create, test, and
    publish Wasm plugins. Without tooling, authors must manually
    configure build targets, sandbox policies, and registry publishing.
  """

  solution """
    specforge plugin CLI subcommands: init scaffolds a project with
    PDK skeleton, build compiles to .wasm targeting wasm32-wasi,
    test loads the binary in a production sandbox against fixtures,
    publish packages and uploads to npm/OCI/GitHub Releases.
  """
}
