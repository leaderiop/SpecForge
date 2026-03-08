// MCP Server behaviors — Lifecycle, resources, and notifications
//
// 13 behaviors:
//   - Lifecycle (3): initialize, shutdown, list resources/tools/prompts
//   - Resources (6): graph, schema, context, brief, diagnostics, entity
//   - Notifications (2): graph delta, diagnostics delta

use invariants/core
use invariants/validation
use invariants/mcp
use invariants/zero-entity-core
use events/mcp
use events/compilation
use types/graph
use types/output
use types/diagnostics
use types/mcp
use types/core
use types/config
use ports/inbound
use ports/outbound

behavior mcp_initialize "MCP Initialize" {
  types      [McpCapabilities, CompilerConfig]
  ports      [CompilerApi, WasmRuntime]
  produces   [mcp_initialized, mcp_initialization_failed]
  invariants [zero_domain_knowledge_core, mcp_structured_error_responses, registry_population_before_validation]

  requires {
    compiler_api_available "CompilerApi port is available and project root has been located"
    wasm_runtime_available "WasmRuntime port is available for loading extension manifests"
  }

  ensures {
    capabilities_returned "McpCapabilities returned containing all registered tools, resources, and prompts"
    surface_contributions_merged "Surface-contributed tools and resources merged with core before advertisement"
    mcp_initialized_emitted "mcp_initialized event emitted on success, mcp_initialization_failed on failure"
  }

  contract """
    The MCP server MUST compile the current project graph, load all
    extension manifests, register all tools/resources/prompts from
    installed extensions (including surface-contributed MCP tools and
    resources from extension manifests), and return McpCapabilities.
    Surface-contributed tools and resources MUST be merged with core
    tools and resources before capability advertisement. The server
    MUST NOT accept tool/resource calls before initialization completes.
  """

  verify unit "initialization registers all tools from installed extensions"
  verify unit "initialization rejects tool calls before completion"
  verify unit "all core tools registered before accepting requests"
  verify unit "all core resources registered before accepting requests"
  verify contract "requires/ensures consistency for MCP initialization"
}

behavior mcp_shutdown "MCP Shutdown" {
  types      [McpSubscription]
  ports      [CompilerApi, WasmRuntime]
  produces   [mcp_server_shutdown, mcp_subscription_removed]
  invariants [mcp_subscription_cleanup, mcp_structured_error_responses]

  requires {
    server_initialized "MCP server has been initialized (mcp_initialized has fired)"
  }

  ensures {
    notifications_flushed "All pending notifications flushed before exit"
    subscriptions_removed "All active subscriptions unsubscribed and mcp_subscription_removed emitted"
    wasm_engines_released "All Wasm engine instances released"
    shutdown_emitted "mcp_server_shutdown event emitted"
  }

  contract """
    The MCP server MUST flush all pending notifications, unsubscribe
    all active subscriptions, release Wasm engine instances, and exit
    cleanly.
  """

  verify unit "shutdown flushes pending notifications"
  verify unit "shutdown releases Wasm engine instances"
  verify unit "shutdown unsubscribes all active subscriptions"
  verify unit "shutdown rejects new tool calls during teardown"
  verify integration "shutdown completes within 5 seconds"
  verify contract "requires/ensures consistency for MCP shutdown"
}

behavior list_mcp_resources "List MCP Resources" {
  invariants [mcp_structured_error_responses, mcp_tool_idempotency]
  ports      [McpProtocol, CompilerApi]
  types [McpResourceDescriptor]
  produces [mcp_discovery_invoked]

  requires {
    server_initialized "MCP server has been initialized and all extensions loaded"
  }

  ensures {
    complete_list_returned "All registered resource descriptors returned including extension-contributed"
    disabled_excluded "Disabled surface contributions excluded from the list"
    discovery_emitted "mcp_discovery_invoked event emitted"
  }

  contract """
    The MCP server MUST return all registered resource descriptors,
    including both core-provided and extension-contributed capabilities.
    Extension-contributed MCP resources (from manifest surfaces.mcp_resources)
    MUST be included alongside core resources. Disabled surface contributions
    MUST be excluded. The list MUST be complete and reflect the current set
    of loaded extensions.
  """

  verify unit "returns all registered resource descriptors after extension load"
  verify unit "returns core-provided descriptors when no extensions installed"
  verify unit "reflects resources from newly loaded extension"
  verify contract "requires/ensures consistency for listing MCP resources"
}

behavior list_mcp_tools "List MCP Tools" {
  invariants [mcp_structured_error_responses, mcp_tool_idempotency]
  ports      [McpProtocol, CompilerApi]
  types [McpToolDescriptor]
  produces [mcp_discovery_invoked]

  requires {
    server_initialized "MCP server has been initialized and all extensions loaded"
  }

  ensures {
    complete_list_returned "All registered tool descriptors returned including auto-promoted CLI commands"
    disabled_excluded "Disabled surface contributions excluded from the list"
    discovery_emitted "mcp_discovery_invoked event emitted"
  }

  contract """
    The MCP server MUST return all registered tool descriptors,
    including both core-provided and extension-contributed capabilities.
    Extension-contributed MCP tools (from manifest surfaces.mcp_tools)
    and auto-promoted CLI commands MUST be included alongside core tools.
    Disabled surface contributions MUST be excluded. The list MUST be
    complete and reflect the current set of loaded extensions.
  """

  verify unit "returns all registered tool descriptors after extension load"
  verify unit "returns core-provided descriptors when no extensions installed"
  verify unit "reflects tools from newly loaded extension"
  verify contract "requires/ensures consistency for listing MCP tools"
}

behavior list_mcp_prompts "List MCP Prompts" {
  invariants [mcp_structured_error_responses, mcp_tool_idempotency]
  ports      [McpProtocol, CompilerApi]
  types [McpPromptDescriptor]
  produces [mcp_discovery_invoked]

  requires {
    server_initialized "MCP server has been initialized and all extensions loaded"
  }

  ensures {
    complete_list_returned "All registered prompt descriptors returned including extension-contributed"
    discovery_emitted "mcp_discovery_invoked event emitted"
  }

  contract """
    The MCP server MUST return all registered prompt descriptors,
    including both core-provided and extension-contributed capabilities.
    The list MUST be complete and reflect the current set of loaded extensions.
  """

  verify unit "returns all registered prompt descriptors after extension load"
  verify unit "returns core-provided descriptors when no extensions installed"
  verify unit "reflects prompts from newly loaded extension"
  verify contract "requires/ensures consistency for listing MCP prompts"
}

// ---------------------------------------------------------------------------
// Section 1: Resources
// ---------------------------------------------------------------------------

behavior expose_graph_as_mcp_resource "Expose Graph as MCP Resource" {
  invariants [graph_traversal_integrity, graph_schema_completeness, diagnostic_determinism, mcp_structured_error_responses]
  types      [Graph, GraphProtocolSchema, McpResourceDescriptor]
  ports      [McpProtocol, CompilerApi]
  consumes   [validation_complete]
  produces   [mcp_resource_read]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming compilation is done"
  }

  ensures {
    graph_json_returned "Graph Protocol JSON returned with embedded schema and schema_version"
    resource_read_emitted "mcp_resource_read event emitted"
  }

  contract """
    In MCP server mode, the system MUST register a specforge://graph resource
    that returns the current compiled graph as Graph Protocol JSON. The resource
    MUST refresh on each recompilation. The output MUST be equivalent to
    specforge export --format=graph. The resource MUST include the embedded
    GraphProtocolSchema and schema_version field. Delegates graph serialization
    to serve_graph_resource (behaviors/output-schema.spec) for actual graph serving;
    this behavior's role is exposing it via the MCP transport protocol.
  """

  verify unit "specforge://graph resource returns full Graph Protocol JSON"
  verify unit "resource refreshes after recompilation"
  verify unit "output includes embedded schema and schema_version"
  verify contract "requires/ensures consistency for graph MCP resource"

}

behavior expose_schema_as_mcp_resource "Expose Schema as MCP Resource" {
  invariants [graph_schema_completeness, diagnostic_determinism, mcp_structured_error_responses]
  types      [GraphProtocolSchema, McpResourceDescriptor]
  ports      [McpProtocol, CompilerApi]
  consumes   [validation_complete]
  produces   [mcp_resource_read]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming compilation is done"
  }

  ensures {
    schema_json_returned "GraphProtocolSchema returned as JSON reflecting current compilation state"
    resource_read_emitted "mcp_resource_read event emitted"
  }

  contract """
    In MCP server mode, the system MUST register a specforge://schema resource
    that returns the current GraphProtocolSchema as JSON. The resource MUST
    reflect the current compilation state and update when extensions are added
    or removed. This allows agents to introspect the graph structure without
    parsing the full graph. Delegates schema serialization to
    serve_schema_resource (behaviors/output-schema.spec).
  """

  verify unit "specforge://schema resource returns GraphProtocolSchema JSON"
  verify unit "schema updates when extensions change"
  verify contract "requires/ensures consistency for schema MCP resource"

}

behavior expose_context_as_mcp_resource "Expose Context as MCP Resource" {
  invariants [graph_traversal_integrity, graph_schema_completeness, diagnostic_determinism, mcp_structured_error_responses]
  types      [Graph, AgentExportConfig, McpResourceDescriptor]
  ports      [McpProtocol, CompilerApi]
  consumes   [validation_complete]
  produces   [mcp_resource_read]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming compilation is done"
  }

  ensures {
    context_format_returned "Token-optimized context format returned equivalent to --format=context"
    resource_read_emitted "mcp_resource_read event emitted"
  }

  contract """
    In MCP server mode, the system MUST register a specforge://context resource
    that returns the current graph in a token-optimized context format. The output
    MUST be equivalent to specforge export --format=context. The resource MUST
    refresh after each recompilation. Agents SHOULD prefer this resource when they
    need full project understanding within a constrained token budget.
  """

  verify unit "specforge://context resource returns token-optimized format"
  verify unit "resource refreshes after recompilation"
  verify contract "requires/ensures consistency for context MCP resource"

}

behavior expose_brief_as_mcp_resource "Expose Brief as MCP Resource" {
  invariants [graph_traversal_integrity, graph_schema_completeness, diagnostic_determinism, mcp_structured_error_responses]
  types      [Graph, AgentExportConfig, McpResourceDescriptor]
  ports      [McpProtocol, CompilerApi]
  consumes   [validation_complete]
  produces   [mcp_resource_read]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming compilation is done"
  }

  ensures {
    brief_format_returned "Minimal IDs-and-edges format returned equivalent to --format=brief"
    resource_read_emitted "mcp_resource_read event emitted"
  }

  contract """
    In MCP server mode, the system MUST register a specforge://brief resource
    that returns the graph in a minimal IDs-and-edges format. The output MUST
    be equivalent to specforge export --format=brief. The resource MUST refresh
    after each recompilation. This format is intended for agents that only need
    structural awareness without full entity details.
  """

  verify unit "specforge://brief resource returns minimal IDs and edges format"
  verify unit "resource refreshes after recompilation"
  verify contract "requires/ensures consistency for brief MCP resource"

}

behavior expose_diagnostics_as_mcp_resource "Expose Diagnostics as MCP Resource" {
  invariants [diagnostic_determinism, mcp_structured_error_responses]
  types      [DiagnosticBag, McpResourceDescriptor]
  ports      [McpProtocol, CompilerApi]
  consumes   [validation_complete]
  produces   [mcp_resource_read]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming compilation is done"
  }

  ensures {
    diagnostics_returned "DiagnosticBag returned as JSON with severity, code, message, file, and span"
    resource_read_emitted "mcp_resource_read event emitted"
  }

  contract """
    In MCP server mode, the system MUST register a specforge://diagnostics
    resource that returns the current DiagnosticBag as JSON. The resource MUST
    update after each recompilation. The output MUST include all diagnostics
    with severity, code, message, file path, and span. Agents MAY poll this
    resource to check project health without triggering a new compilation.
  """

  verify unit "specforge://diagnostics resource returns current DiagnosticBag as JSON"
  verify unit "resource updates after recompilation"
  verify unit "each diagnostic includes severity, code, message, file, and span"
  verify contract "requires/ensures consistency for diagnostics MCP resource"

}

behavior expose_entity_as_mcp_resource "Expose Per-Entity MCP Resource" {
  invariants [graph_traversal_integrity, graph_schema_completeness, diagnostic_determinism, mcp_structured_error_responses]
  types      [Graph, Node, Edge, McpResourceDescriptor]
  ports      [McpProtocol, CompilerApi]
  consumes   [validation_complete]
  produces   [mcp_resource_read]

  requires {
    validation_complete_fired "validation_complete event has fired, confirming compilation is done"
  }

  ensures {
    subgraph_returned "Target node and all directly connected nodes and edges returned as subgraph"
    resource_read_emitted "mcp_resource_read event emitted"
  }

  contract """
    In MCP server mode, the system MUST register a specforge://graph/{entity_id}
    resource template that returns a single entity and its immediate neighbors as
    a subgraph. The resource MUST include the target node, all directly connected
    nodes, and the edges between them. If the entity_id does not exist, the
    resource MUST return a 404 error. The resource MUST refresh after recompilation.
  """

  verify unit "specforge://graph/{entity_id} returns entity and its neighbors"
  verify unit "non-existent entity_id returns 404 error"
  verify unit "malformed entity_id returns 400 error"
  verify unit "resource refreshes after recompilation"
  verify contract "requires/ensures consistency for per-entity MCP resource"

}

// ---------------------------------------------------------------------------
// Section 2: Notifications
// ---------------------------------------------------------------------------

// This is the MCP-specific implementation of notify_delta_subscribers
// (behaviors/incremental.spec). It adapts the delta notification to the
// MCP transport protocol.
behavior notify_graph_delta_via_mcp "Notify Graph Delta via MCP" {
  invariants [incremental_correctness, graph_traversal_integrity, mcp_structured_error_responses, mcp_subscription_cleanup]
  types      [GraphDelta, McpSubscription]
  ports      [McpProtocol, CompilerApi]
  consumes   [graph_delta_computed]
  produces   [mcp_delta_notified, mcp_subscription_created, mcp_subscription_removed]

  requires {
    graph_delta_computed_fired "graph_delta_computed event has fired after incremental rebuild"
  }

  ensures {
    subscribers_notified "All subscribed MCP clients receive notifications/graph_changed with GraphDelta payload"
    no_notification_when_empty "Notification suppressed when no clients are subscribed"
    delta_notified_emitted "mcp_delta_notified event emitted after notification delivery"
  }

  contract """
    When an incremental rebuild completes in MCP server mode, the system MUST
    send a notifications/graph_changed notification to all subscribed MCP
    clients. The notification payload MUST include the GraphDelta describing
    added, removed, and modified nodes and edges. Clients MUST be able to
    subscribe and unsubscribe from delta notifications. If no clients are
    subscribed, the notification MUST be suppressed.
  """

  verify unit "graph_changed notification sent after incremental rebuild"
  verify unit "notification includes GraphDelta payload"
  verify unit "unsubscribed clients do not receive notifications"
  verify unit "no notification when no clients subscribed"
  verify contract "requires/ensures consistency for graph delta MCP notification"

}

behavior notify_diagnostics_delta_via_mcp "Notify Diagnostics Delta via MCP" {
  invariants [incremental_correctness, diagnostic_determinism, mcp_structured_error_responses, mcp_subscription_cleanup]
  types      [DiagnosticsDelta, McpSubscription]
  ports      [McpProtocol, CompilerApi]
  consumes   [validation_complete]
  produces   [mcp_delta_notified, mcp_subscription_created, mcp_subscription_removed]

  requires {
    validation_complete_fired "validation_complete event has fired after compilation"
  }

  ensures {
    subscribers_notified "All subscribed MCP clients receive notifications/diagnostics_changed"
    unchanged_suppressed "Notification suppressed when diagnostics are unchanged or no clients subscribed"
    delta_notified_emitted "mcp_delta_notified event emitted after notification delivery"
  }

  contract """
    When validation completes in MCP server mode, the system MUST send a
    notifications/diagnostics_changed notification to all subscribed MCP clients.
    The notification payload MUST include added and removed diagnostics since the
    previous compilation. Clients MUST be able to subscribe and unsubscribe. If
    no clients are subscribed or the diagnostics are unchanged, the notification
    MUST be suppressed.
  """

  verify unit "diagnostics_changed notification sent after validation"
  verify unit "payload includes added and removed diagnostics"
  verify unit "unsubscribed clients do not receive notifications"
  verify unit "no notification when diagnostics are unchanged"
  verify contract "requires/ensures consistency for diagnostics delta MCP notification"

}

// ---------------------------------------------------------------------------
// Section 3: Protocol Compliance
// ---------------------------------------------------------------------------

behavior handle_mcp_protocol_error "Handle MCP Protocol Error" {
  invariants [mcp_structured_error_responses]
  types      [McpError]
  ports      [McpProtocol]
  produces   [mcp_protocol_error_handled]

  requires {
    mcp_protocol_available "McpProtocol port is available and server is accepting requests"
  }

  ensures {
    standard_error_returned "JSON-RPC 2.0 standard error code returned with human-readable message"
    no_state_leaked "Error response does not leak internal state (stack traces, file paths, memory addresses)"
    server_operational "Server remains operational after protocol error"
    error_handled_emitted "mcp_protocol_error_handled event emitted"
  }

  contract """
    When the MCP server receives a malformed JSON-RPC request (parse error,
    invalid method, missing required params), it MUST respond with the
    standard JSON-RPC 2.0 error codes: -32700 (Parse error), -32600
    (Invalid Request), -32601 (Method not found), -32602 (Invalid params),
    -32603 (Internal error). The error response MUST NOT crash the server
    or leak internal state (stack traces, file paths, memory addresses).
    The error response MUST include a human-readable message field.
  """

  verify unit "malformed JSON produces -32700 Parse error"
  verify unit "invalid method produces -32601 Method not found"
  verify unit "missing required params produces -32602 Invalid params"
  verify unit "error response does not leak internal state"
  verify unit "server remains operational after protocol error"
  verify unit "returns -32600 for invalid request"
  verify unit "returns -32603 for internal error"
  verify contract "requires/ensures consistency for MCP protocol error handling"

}

behavior handle_mcp_request_cancellation "Handle MCP Request Cancellation" {
  invariants [mcp_structured_error_responses]
  types      [McpError]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_request_cancelled]

  requires {
    mcp_protocol_available "McpProtocol port is available and server is accepting requests"
  }

  ensures {
    cancellation_safe "Server state remains consistent after cancellation — no crash or corruption"
    request_cancelled_emitted "mcp_request_cancelled event emitted when operation is cancelled"
  }

  contract """
    When a notifications/cancelled message arrives referencing an in-progress
    request ID, the server MUST cancel the operation if possible and respond
    with appropriate status. If the operation has already completed, the
    cancellation MUST be a no-op. If the operation is cancellable (e.g., a
    long-running export or validation), the server SHOULD attempt to stop it
    and return a partial result or cancellation acknowledgment. The server
    MUST NOT crash or enter an inconsistent state due to cancellation.
  """

  verify unit "cancellation of in-progress request stops operation"
  verify unit "cancellation of completed request is a no-op"
  verify unit "server state remains consistent after cancellation"
  verify integration "cancelled long-running export returns partial result or acknowledgment"
  verify contract "requires/ensures consistency for MCP request cancellation"

}

behavior guard_mcp_reinitialization "Guard MCP Reinitialization" {
  invariants [mcp_structured_error_responses]
  types      [McpCapabilities]
  ports      [McpProtocol]
  produces   [mcp_protocol_error_handled]

  requires {
    server_initialized "MCP server has already been initialized (first initialize completed)"
  }

  ensures {
    reinit_rejected "JSON-RPC error -32600 returned for duplicate initialize request"
    session_unaffected "Existing session continues unaffected — no state reset or resource leak"
    error_handled_emitted "mcp_protocol_error_handled event emitted"
  }

  contract """
    When an already-initialized MCP server receives another initialize
    request, it MUST respond with a JSON-RPC error (-32600 Invalid Request)
    per MCP protocol specification. The server MUST NOT re-initialize,
    reset state, or leak resources. The existing session MUST continue
    unaffected.
  """

  verify unit "second initialize request returns -32600 error"
  verify unit "existing session continues after rejected reinitialization"
  verify unit "no resources leaked on rejected reinitialization"
  verify contract "requires/ensures consistency for MCP reinitialization guard"

}
