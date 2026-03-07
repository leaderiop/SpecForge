// MCP Server events — signals emitted during MCP protocol interactions

use types/graph
use types/output
use types/diagnostics
use types/mcp
use behaviors/mcp-operations
use behaviors/mcp-prompts
use behaviors/mcp-server
use behaviors/mcp-tools

// MCP protocol treats tool/prompt/resource listing as tool-like operations.
// This event tracks discovery requests separately from regular tool invocations
// to distinguish capability negotiation from actual tool usage.
event mcp_discovery_invoked "MCP Discovery Invoked" {
  trigger   [list_mcp_resources, list_mcp_tools, list_mcp_prompts]
  channel   "mcp.discovery_invoked"

  payload {
    discoveryType  string
    resultCount    integer
    timestamp      timestamp
  }

  consumers []

  verify integration "emits mcp_discovery_invoked with correct discoveryType when agent lists tools, prompts, or resources"

}

event mcp_resource_read "MCP Resource Read" {
  trigger   [expose_graph_as_mcp_resource, expose_schema_as_mcp_resource, expose_context_as_mcp_resource, expose_brief_as_mcp_resource, expose_diagnostics_as_mcp_resource, expose_entity_as_mcp_resource]
  channel   "mcp.resource_read"

  payload {
    resourceUri    string
    format         string
    timestamp      timestamp
  }

  consumers []

  verify integration "emits mcp_resource_read with correct resourceUri when agent reads any MCP resource"

}

event mcp_tool_invoked "MCP Tool Invoked" {
  trigger   [list_mcp_resources, list_mcp_tools, list_mcp_prompts, provide_mcp_query_tool, provide_mcp_validate_tool, provide_mcp_export_tool, provide_mcp_trace_tool, provide_mcp_search_tool, provide_mcp_schema_tool, provide_mcp_coverage_tool, provide_mcp_stats_tool, provide_mcp_inspect_tool, provide_mcp_find_definition_tool, provide_mcp_find_references_tool, provide_mcp_outline_tool, provide_mcp_suggest_fixes_tool, provide_mcp_format_tool, provide_mcp_rename_tool, provide_mcp_init_tool, provide_mcp_add_extension_tool, provide_mcp_remove_extension_tool, provide_mcp_migrate_tool, provide_mcp_extensions_tool, provide_mcp_providers_tool, provide_mcp_doctor_tool, provide_mcp_collect_tool, provide_mcp_render_tool]
  channel   "mcp.tool_invoked"

  payload {
    toolName       string
    category       McpToolCategory
    entityId       string    @optional
    params         string    @optional
    timestamp      timestamp
  }

  consumers []

  verify integration "emits mcp_tool_invoked with correct toolName, category, and parameters for any tool call"

}

event mcp_delta_notified "MCP Delta Notified" {
  trigger   [notify_graph_delta_via_mcp, notify_diagnostics_delta_via_mcp]
  channel   "mcp.delta_notified"

  payload {
    notificationType   string
    subscriberCount    integer
    addedNodes         integer   @optional
    removedNodes       integer   @optional
    modifiedNodes      integer   @optional
    addedDiagnostics   integer   @optional
    removedDiagnostics integer   @optional
    timestamp          timestamp
  }

  consumers []

  verify integration "emits mcp_delta_notified with correct notification type and delta summary"

}

event mcp_prompt_invoked "MCP Prompt Invoked" {
  trigger   [provide_mcp_implement_prompt, provide_mcp_review_prompt, provide_mcp_trace_prompt, provide_mcp_explore_prompt]
  channel   "mcp.prompt_invoked"

  payload {
    promptName     string
    entityId       string    @optional
    kind           string    @optional
    timestamp      timestamp
  }

  consumers []

  verify integration "emits mcp_prompt_invoked with correct promptName and arguments"

}

event mcp_initialized "MCP Initialized" {
  trigger   [mcp_initialize]
  channel   "mcp.initialized"

  payload {
    tools_registered       integer
    resources_registered   integer
    prompts_registered     integer
    extensions_loaded      integer
  }

  consumers []

  verify integration "mcp initialization emits event with tool counts"

}

event mcp_mutation_completed "MCP Mutation Completed" {
  trigger   [provide_mcp_format_tool, provide_mcp_rename_tool, provide_mcp_init_tool, provide_mcp_add_extension_tool, provide_mcp_remove_extension_tool, provide_mcp_migrate_tool]
  channel   "mcp.mutation_completed"

  payload {
    toolName               string
    files_changed          integer
    entities_affected      integer
    success                boolean
    timestamp              timestamp
  }

  consumers [notify_delta_subscribers]

  verify integration "emits mcp_mutation_completed with structured outcome after each mutation tool"

}

// ── MCP Subscription Lifecycle Events ────────────────────────

event mcp_subscription_created "MCP Subscription Created" {
  trigger   [notify_graph_delta_via_mcp, notify_diagnostics_delta_via_mcp]
  channel   "mcp.subscription_created"

  payload {
    subscriptionType   string
    clientId           string
    timestamp          timestamp
  }

  consumers []

  verify integration "emits mcp_subscription_created when a client subscribes to delta notifications"

}

event mcp_subscription_removed "MCP Subscription Removed" {
  trigger   [notify_graph_delta_via_mcp, notify_diagnostics_delta_via_mcp, mcp_shutdown]
  channel   "mcp.subscription_removed"

  payload {
    subscriptionType   string
    clientId           string
    timestamp          timestamp
  }

  consumers []

  verify integration "emits mcp_subscription_removed when a client unsubscribes or server shuts down"

}

event mcp_initialization_failed "MCP Initialization Failed" {
  trigger   mcp_initialize
  channel   "mcp.initialization_failed"

  payload {
    error_kind         string
    message            string
    timestamp          timestamp
  }

  consumers []

  verify integration "emits mcp_initialization_failed when MCP server fails to initialize"

}

event mcp_server_shutdown "MCP Server Shutdown" {
  trigger   [mcp_shutdown]
  channel   "mcp.shutdown"

  payload {
    pending_notifications_flushed  integer
    subscriptions_released         integer
    wasm_engines_released          integer
    timestamp                      timestamp
  }

  consumers []

  verify integration "emits mcp_server_shutdown with correct counts when MCP server shuts down"

}

