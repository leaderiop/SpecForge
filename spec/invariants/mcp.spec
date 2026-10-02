// MCP-specific invariants — guarantees for MCP server protocol interactions

use "types/mcp"

invariant mcp_structured_error_responses "MCP Structured Error Responses" {
  guarantee """
    All MCP tools and resources MUST return structured error objects (not plain
    strings) with error code, message, and optional entity_id. This ensures agents
    can programmatically handle errors without parsing free-form text. A failed
    tool call is an isError result whose content is an McpError; a diagnostic
    code behind the failure is in its diagnostic, not only in the message.
  """
  risk      medium
  verify unit "error response includes error code and message fields"
  verify unit "error response includes entity_id when applicable"
  verify unit "no MCP endpoint returns a plain string error"
  verify unit "success responses never have error field"
  verify unit "a failed tool call is an isError result whose content is an McpError with a code"
  verify unit "a diagnostic code behind a failed tool call is in its McpError diagnostic"
}

invariant mcp_subscription_cleanup "MCP Subscription Cleanup" {
  guarantee """
    When an MCP client disconnects, all its subscriptions MUST be removed. No
    orphan subscriptions may remain after client disconnect. This prevents
    resource leaks and ensures notification delivery targets only active clients.
  """
  risk      high
  verify unit "client disconnect removes all subscriptions for that client"
  verify unit "no orphan subscriptions remain after disconnect"
  verify integration "rapid connect/disconnect cycles leave zero subscriptions"
}

invariant mcp_tool_idempotency "MCP Tool Idempotency" {
  guarantee """
    Read-only MCP tools (core, navigation, and read-only project management:
    extensions, providers, doctor) and all MCP
    prompts MUST be idempotent: repeated calls with the same parameters MUST
    return the same result if the graph has not changed between calls. This
    guarantees agents can safely retry read operations and prompt invocations.
  """
  risk      medium
  verify property "repeated calls with same params return identical results when graph unchanged"
  verify unit "read-only tools return equivalent results for identical inputs"
}

invariant mcp_served_project_consistency "MCP Served Project Consistency" {
  guarantee """
    The project an MCP server serves is replaced whole or not at all.
    Every compile that replaces it (initialize, a refresh after watch
    writes a newer snapshot, validate, analyze, doctor, collect, a
    mutation tool that wrote files) installs the graph, diagnostics,
    registries and extension tools and resources of that one compile together, and
    tells subscribed clients what changed. A tool call whose path names
    another project compiles that project for the call only: the server
    keeps serving its own.
  """
  risk      high
  verify unit "a refresh after a newer watch snapshot updates the extension tools listed"
  verify unit "analyze notifies subscribers when the diagnostics it compiled changed"
  verify unit "validate with a path to another project leaves the served project in place"
  verify unit "a mutation tool that wrote files leaves the server serving what is on disk"
}

invariant mcp_type_schema_versioning "MCP Type Schema Versioning" {
  guarantee """
    Breaking changes to types consumed by MCP tools (McpToolDescriptor,
    McpCoverageResult, McpInspectResult, McpTracePlanResult) MUST trigger
    a major version increment in the Graph Protocol schema version.
  """
  risk      high
  verify unit "adding required field to MCP type triggers major version bump"
}
