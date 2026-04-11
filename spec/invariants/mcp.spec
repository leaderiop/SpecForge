// MCP-specific invariants — guarantees for MCP server protocol interactions

use "behaviors/mcp-operations"
use "behaviors/mcp-prompts"
use "behaviors/mcp-server"
use "behaviors/mcp-tools"
use "behaviors/surface-contributions"
use "types/mcp"
invariant mcp_structured_error_responses "MCP Structured Error Responses" {
  guarantee """
    All MCP tools and resources MUST return structured error objects (not plain
    strings) with error code, message, and optional entity_id. This ensures agents
    can programmatically handle errors without parsing free-form text.
  """
  enforced_by [mcp_initialize, mcp_shutdown, list_mcp_resources, list_mcp_tools, list_mcp_prompts, expose_graph_as_mcp_resource, expose_schema_as_mcp_resource, expose_context_as_mcp_resource, expose_brief_as_mcp_resource, expose_diagnostics_as_mcp_resource, expose_entity_as_mcp_resource, notify_graph_delta_via_mcp, notify_diagnostics_delta_via_mcp, provide_mcp_query_tool, provide_mcp_validate_tool, provide_mcp_export_tool, provide_mcp_trace_tool, provide_mcp_search_tool, provide_mcp_schema_tool, provide_mcp_coverage_tool, provide_mcp_stats_tool, provide_mcp_inspect_tool, provide_mcp_find_definition_tool, provide_mcp_find_references_tool, provide_mcp_outline_tool, provide_mcp_suggest_fixes_tool, provide_mcp_format_tool, provide_mcp_rename_tool, provide_mcp_init_tool, provide_mcp_add_extension_tool, provide_mcp_remove_extension_tool, provide_mcp_migrate_tool, provide_mcp_extensions_tool, provide_mcp_providers_tool, provide_mcp_doctor_tool, provide_mcp_collect_tool, provide_mcp_render_tool, provide_mcp_context_prompt, provide_mcp_review_prompt, provide_mcp_trace_prompt, provide_mcp_explore_prompt, guard_mcp_reinitialization, handle_mcp_protocol_error, handle_mcp_request_cancellation, dispatch_surface_mcp_tool, dispatch_surface_mcp_resource]
  risk medium

  verify unit "error response includes error code and message fields"
  verify unit "error response includes entity_id when applicable"
  verify unit "no MCP endpoint returns a plain string error"

}

invariant mcp_subscription_cleanup "MCP Subscription Cleanup" {
  guarantee """
    When an MCP client disconnects, all its subscriptions MUST be removed. No
    orphan subscriptions may remain after client disconnect. This prevents
    resource leaks and ensures notification delivery targets only active clients.
  """
  enforced_by [notify_graph_delta_via_mcp, notify_diagnostics_delta_via_mcp, mcp_shutdown]
  risk high

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
  enforced_by [list_mcp_resources, list_mcp_tools, list_mcp_prompts, provide_mcp_query_tool, provide_mcp_validate_tool, provide_mcp_export_tool, provide_mcp_trace_tool, provide_mcp_search_tool, provide_mcp_schema_tool, provide_mcp_coverage_tool, provide_mcp_stats_tool, provide_mcp_inspect_tool, provide_mcp_find_definition_tool, provide_mcp_find_references_tool, provide_mcp_outline_tool, provide_mcp_suggest_fixes_tool, provide_mcp_extensions_tool, provide_mcp_providers_tool, provide_mcp_doctor_tool, provide_mcp_context_prompt, provide_mcp_review_prompt, provide_mcp_trace_prompt, provide_mcp_explore_prompt]
  risk medium

  verify property "repeated calls with same params return identical results when graph unchanged"
  verify unit "read-only tools return equivalent results for identical inputs"

}

invariant mcp_type_schema_versioning "MCP Type Schema Versioning" {
  guarantee """
    Breaking changes to types consumed by MCP tools (McpToolDescriptor,
    McpCoverageResult, McpInspectResult, McpTracePlanResult) MUST trigger
    a major version increment in the Graph Protocol schema version.
  """
  enforced_by [compute_schema_version, detect_breaking_schema_changes]
  risk high
  verify unit "adding required field to MCP type triggers major version bump"
}
