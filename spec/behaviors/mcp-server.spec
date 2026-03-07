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
  contract """
    The MCP server MUST compile the current project graph, load all
    extension manifests, register all tools/resources/prompts from
    installed extensions, and return McpCapabilities. The server MUST
    NOT accept tool/resource calls before initialization completes.
  """
  types      [McpCapabilities, CompilerConfig]
  ports      [CompilerApi, WasmRuntime]
  produces   [mcp_initialized, mcp_initialization_failed]
  invariants [zero_domain_knowledge_core, mcp_structured_error_responses]

  verify unit "initialization registers all tools from installed extensions"
  verify unit "initialization rejects tool calls before completion"
}

behavior mcp_shutdown "MCP Shutdown" {
  contract """
    The MCP server MUST flush all pending notifications, unsubscribe
    all active subscriptions, release Wasm engine instances, and exit
    cleanly.
  """
  types      [McpSubscription]
  ports      [CompilerApi, WasmRuntime]
  produces   [mcp_server_shutdown, mcp_subscription_removed]
  invariants [mcp_subscription_cleanup, mcp_structured_error_responses]

  verify unit "shutdown flushes pending notifications"
  verify unit "shutdown releases Wasm engine instances"
}

behavior list_mcp_resources "List MCP Resources" {
  invariants [mcp_structured_error_responses, mcp_tool_idempotency]
  ports      [McpProtocol, CompilerApi]
  contract """
    The MCP server MUST return all registered resource descriptors from
    installed extensions. The list MUST be complete and reflect the
    current set of loaded extensions.
  """
  types [McpResourceDescriptor]
  produces [mcp_tool_invoked, mcp_discovery_invoked]

  verify "returns all registered resource descriptors after extension load"
}

behavior list_mcp_tools "List MCP Tools" {
  invariants [mcp_structured_error_responses, mcp_tool_idempotency]
  ports      [McpProtocol, CompilerApi]
  contract """
    The MCP server MUST return all registered tool descriptors from
    installed extensions. The list MUST be complete and reflect the
    current set of loaded extensions.
  """
  types [McpToolDescriptor]
  produces [mcp_tool_invoked, mcp_discovery_invoked]

  verify "returns all registered tool descriptors after extension load"
}

behavior list_mcp_prompts "List MCP Prompts" {
  invariants [mcp_structured_error_responses, mcp_tool_idempotency]
  ports      [McpProtocol, CompilerApi]
  contract """
    The MCP server MUST return all registered prompt descriptors from
    installed extensions. The list MUST be complete and reflect the
    current set of loaded extensions.
  """
  types [McpPromptDescriptor]
  produces [mcp_tool_invoked, mcp_discovery_invoked]

  verify "returns all registered prompt descriptors after extension load"
}

// ---------------------------------------------------------------------------
// Section 1: Resources
// ---------------------------------------------------------------------------

behavior expose_graph_as_mcp_resource "Expose Graph as MCP Resource" {
  invariants [graph_traversal_integrity, graph_schema_completeness, diagnostic_determinism, mcp_structured_error_responses]
  types      [Graph, GraphProtocolSchema, McpResourceDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_resource_read]

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

}

behavior expose_schema_as_mcp_resource "Expose Schema as MCP Resource" {
  invariants [graph_schema_completeness, diagnostic_determinism, mcp_structured_error_responses]
  types      [GraphProtocolSchema, McpResourceDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_resource_read]

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

}

behavior expose_context_as_mcp_resource "Expose Context as MCP Resource" {
  invariants [graph_traversal_integrity, graph_schema_completeness, diagnostic_determinism, mcp_structured_error_responses]
  types      [Graph, AgentExportConfig, McpResourceDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_resource_read]

  contract """
    In MCP server mode, the system MUST register a specforge://context resource
    that returns the current graph in a token-optimized context format. The output
    MUST be equivalent to specforge export --format=context. The resource MUST
    refresh after each recompilation. Agents SHOULD prefer this resource when they
    need full project understanding within a constrained token budget.
  """

  verify unit "specforge://context resource returns token-optimized format"
  verify unit "resource refreshes after recompilation"

}

behavior expose_brief_as_mcp_resource "Expose Brief as MCP Resource" {
  invariants [graph_traversal_integrity, graph_schema_completeness, diagnostic_determinism, mcp_structured_error_responses]
  types      [Graph, AgentExportConfig, McpResourceDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_resource_read]

  contract """
    In MCP server mode, the system MUST register a specforge://brief resource
    that returns the graph in a minimal IDs-and-edges format. The output MUST
    be equivalent to specforge export --format=brief. The resource MUST refresh
    after each recompilation. This format is intended for agents that only need
    structural awareness without full entity details.
  """

  verify unit "specforge://brief resource returns minimal IDs and edges format"
  verify unit "resource refreshes after recompilation"

}

behavior expose_diagnostics_as_mcp_resource "Expose Diagnostics as MCP Resource" {
  invariants [diagnostic_determinism, mcp_structured_error_responses]
  types      [DiagnosticBag, McpResourceDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_resource_read]

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

}

behavior expose_entity_as_mcp_resource "Expose Per-Entity MCP Resource" {
  invariants [graph_traversal_integrity, diagnostic_determinism, mcp_structured_error_responses]
  types      [Graph, Node, Edge, McpResourceDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_resource_read]

  contract """
    In MCP server mode, the system MUST register a specforge://graph/{entity_id}
    resource template that returns a single entity and its immediate neighbors as
    a subgraph. The resource MUST include the target node, all directly connected
    nodes, and the edges between them. If the entity_id does not exist, the
    resource MUST return a 404 error. The resource MUST refresh after recompilation.
  """

  verify unit "specforge://graph/{entity_id} returns entity and its neighbors"
  verify unit "non-existent entity_id returns 404 error"
  verify unit "resource refreshes after recompilation"

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

}

behavior notify_diagnostics_delta_via_mcp "Notify Diagnostics Delta via MCP" {
  invariants [diagnostic_determinism, mcp_structured_error_responses, mcp_subscription_cleanup]
  types      [DiagnosticsDelta, McpSubscription]
  ports      [McpProtocol, CompilerApi]
  consumes   [validation_complete]
  produces   [mcp_delta_notified, mcp_subscription_created, mcp_subscription_removed]

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

}
