// Host function ABI contracts for Wasm extensions
//
// Design note: Host functions are synchronous RPC-style ABI calls within a
// single compilation step, not asynchronous pipeline stages. They do NOT
// produce events themselves — traceability comes from the calling behavior's
// events (e.g., extension_validated, contribution_exports_dispatched), not
// from the host function invocation. This is an intentional architectural
// decision: host functions are leaf operations in the call chain, not
// pipeline stages that trigger downstream consumers.

use invariants/wasm
use invariants/zero-entity-core
use types/wasm
use types/zero-entity-core
use ports/outbound

behavior compute_extension_query_scope "Compute Extension Query Scope" {
  invariants [wasm_sandbox_integrity, host_function_type_safety]
  types      [HostFunctionBinding, ManifestV2, SandboxPolicy]
  ports      [WasmRuntime]

  contract """
    Before serving a query_graph call, the runtime MUST compute the
    calling extension's query scope. The scope is derived from the
    extension's manifest: it includes all entity kinds declared by
    the extension itself, all entity kinds declared by its peer
    dependencies, and any entity kinds explicitly listed in the
    manifest's query_scope field. If query_scope is omitted, the
    default is "all" — the full graph is visible. If query_scope
    is set to "own", only entities of kinds declared by the extension
    and its peers are included. The computed scope MUST be cached
    per extension for the duration of the compilation.
  """

  verify unit "default query_scope exposes full graph"
  verify unit "query_scope 'own' limits to extension and peer kinds"
  verify unit "explicit query_scope list limits to listed kinds"
  verify unit "computed scope cached per extension per compilation"

}

behavior provide_host_function_query_graph "Provide Host Function: query_graph" {
  invariants [wasm_sandbox_integrity, host_function_type_safety]
  types      [HostFunctionBinding]
  ports      [WasmRuntime]

  contract """
    The specforge.query_graph host function MUST expose the compiled
    graph as a JSON string to the calling extension. The graph MUST include
    all entities, edges, and metadata accessible to the extension based
    on its declared scope as computed by compute_extension_query_scope.
    Extensions with query_scope "all" (default) receive the full graph.
    Extensions with restricted scope receive a filtered subgraph.
  """

  verify unit "query_graph returns valid JSON graph"
  verify unit "graph includes entities and edges"
  verify unit "restricted scope returns filtered subgraph"

}

behavior provide_host_function_emit_diagnostic "Provide Host Function: emit_diagnostic" {
  invariants [host_function_type_safety]
  types      [HostFunctionBinding]
  ports      [WasmRuntime]

  contract """
    The specforge.emit_diagnostic host function MUST accept a diagnostic
    object with severity, code, message, and optional source span. The
    diagnostic MUST be added to the compiler's diagnostic collection and
    rendered like core diagnostics.
  """

  verify unit "emit_diagnostic adds to compiler diagnostic collection"
  verify unit "extension diagnostics rendered like core diagnostics"

}

// NOTE: Entity kinds and edge types are registered DECLARATIVELY from
// extension manifests (see register_entity_kinds_from_manifest and
// register_edge_types_from_manifest in behaviors/zero-entity-core.spec).
// The specforge.add_graph_node and specforge.add_graph_edge host functions
// allow extensions to add graph node and edge INSTANCES at runtime during
// contribution exports. These do NOT register new entity kinds or edge types —
// they add instances of already-declared kinds to the graph.

behavior provide_host_function_add_graph_node "Provide Host Function: add_graph_node" {
  invariants [host_function_type_safety, zero_domain_knowledge_core]
  types      [HostFunctionBinding, ManifestV2]
  ports      [WasmRuntime]

  contract """
    The specforge.add_graph_node host function MUST accept a graph node
    instance with an entity kind, ID, and field values. The entity kind
    MUST already be declared in the extension's manifest — this function
    adds node instances to the graph, NOT new entity kinds. The runtime
    MUST reject nodes whose entity kind is not declared by any loaded
    extension manifest. Field values MUST be validated against the kind's
    declared schema. Added nodes MUST participate in resolution and
    validation like parser-produced nodes.
  """

  verify unit "adds graph node instance for declared entity kind"
  verify unit "rejects node for undeclared entity kind"
  verify unit "validates field values against kind schema"

}

behavior provide_host_function_add_graph_edge "Provide Host Function: add_graph_edge" {
  invariants [host_function_type_safety, zero_domain_knowledge_core]
  types      [HostFunctionBinding]
  ports      [WasmRuntime]

  contract """
    The specforge.add_graph_edge host function MUST accept a graph edge
    instance with a label, source node ID, and target node ID. The edge
    label MUST correspond to an edge type already declared in a loaded
    extension manifest — this function adds edge instances to the graph,
    NOT new edge types. The runtime MUST reject edges whose label is not
    declared by any loaded extension manifest. Both source and target
    nodes MUST exist in the graph. Added edges MUST participate in
    graph queries and validation like parser-produced edges.
  """

  verify unit "adds graph edge instance for declared edge type"
  verify unit "rejects edge for undeclared edge label"
  verify unit "rejects edge when source or target node missing"

}

behavior provide_host_function_emit_file "Provide Host Function: emit_file" {
  invariants [wasm_sandbox_integrity, host_function_type_safety, extension_isolation]
  types      [HostFunctionBinding, SandboxPolicy]
  ports      [WasmRuntime, FileSystem]

  contract """
    The specforge.emit_file host function MUST accept a file path and
    content from the extension. Valid files MUST be written to disk
    relative to the project output directory. The host MUST reject
    paths outside the output directory. Extensions MUST only use emit_file
    for non-code outputs (reports, traceability matrices, dashboards,
    graph visualizations).

    WHITELIST MODEL: The runtime MUST enforce a strict allowlist for
    output file extensions. The default allowed_output_extensions is:
    [.json, .html, .csv, .svg, .dot, .xml, .txt, .pdf]. All other
    extensions MUST be rejected unless explicitly added to
    SandboxPolicy.allowed_output_extensions in specforge.json or the
    extension manifest. This is a whitelist — unlisted extensions are
    denied by default. Note: .yaml/.yml are NOT in the defaults —
    extensions that need YAML output MUST explicitly opt in via their
    manifest or project config.

    Extensions MUST NOT use emit_file to generate source code,
    configuration files, or executable artifacts — SpecForge provides
    context, agents produce output.
  """

  verify unit "valid file path within output directory is written"
  verify unit "file path outside output directory is rejected"
  verify unit "file with non-allowed extension is rejected"
  verify unit "file with .json extension is accepted"
  verify unit "file with .html extension is accepted"
  verify unit "emit_file rejects .rs extension"
  verify unit "emit_file rejects .py extension"
  verify unit "emit_file rejects .go extension"
  verify unit "emit_file rejects .js extension"
  verify unit "emit_file rejects .ts extension"
  verify unit "emit_file rejects .sh extension"

}

behavior provide_host_function_http_get "Provide Host Function: http_get" {
  invariants [wasm_sandbox_integrity, host_function_type_safety, extension_isolation]
  types      [HostFunctionBinding, SandboxPolicy]
  ports      [WasmRuntime]

  contract """
    The specforge.http_get host function MUST fetch a URL and return
    the response body to the calling extension. The URL MUST be validated
    against the sandbox policy's allowed domains. Requests to disallowed
    domains MUST be rejected. Timeouts MUST be enforced. The default
    per-request timeout MUST be 5 seconds. The total time budget for all
    http_get calls within a single compilation MUST NOT exceed 15 seconds.
    Timeout failures MUST produce a W-level warning diagnostic, not a hard
    error, to preserve the seconds-to-value guarantee (P8). The timeout is
    configurable via SandboxPolicy. The specforge.http_get host function
    MUST only be callable from providers contribution exports. Calls from
    validators, renderers, or entities contributions MUST return a
    permission error.
  """

  verify unit "allowed domain returns response body"
  verify unit "disallowed domain is rejected"
  verify unit "timeout is enforced"
  verify unit "timeout defaults to 5 seconds when not configured"
  verify unit "timeout failure produces warning, not error"
  verify unit "http_get from validator contribution returns permission error"

}
