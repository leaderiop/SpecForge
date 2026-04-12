// MCP Server events — signals emitted during MCP protocol interactions

use "types/graph"
use "types/output"
use "types/diagnostics"
use "types/mcp"
// MCP protocol treats tool/prompt/resource listing as tool-like operations.
// This event tracks discovery requests separately from regular tool invocations
// to distinguish capability negotiation from actual tool usage.
event mcp_discovery_invoked "MCP Discovery Invoked" {
  channel   "mcp.discovery_invoked"

  payload {
    discoveryType  string
    resultCount    integer
    timestamp      timestamp
  }


  verify integration "emits mcp_discovery_invoked with correct discoveryType when agent lists tools, prompts, or resources"

}

event mcp_resource_read "MCP Resource Read" {
  channel   "mcp.resource_read"

  payload {
    resourceUri    string
    format         string
    timestamp      timestamp
  }


  verify integration "emits mcp_resource_read with correct resourceUri when agent reads any MCP resource"

}

event mcp_tool_invoked "MCP Tool Invoked" {
  channel   "mcp.tool_invoked"

  payload {
    toolName       string
    category       McpToolCategory
    entityId       string    @optional
    params         string    @optional
    timestamp      timestamp
  }


  verify integration "emits mcp_tool_invoked with correct toolName, category, and parameters for any tool call"

}

event mcp_delta_notified "MCP Delta Notified" {
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


  verify integration "emits mcp_delta_notified with correct notification type and delta summary"

}

event mcp_prompt_invoked "MCP Prompt Invoked" {
  channel   "mcp.prompt_invoked"

  payload {
    promptName     string
    entityId       string    @optional
    kind           string    @optional
    timestamp      timestamp
  }


  verify integration "emits mcp_prompt_invoked with correct promptName and arguments"

}

event mcp_initialized "MCP Initialized" {
  channel   "mcp.initialized"

  payload {
    tools_registered       integer
    resources_registered   integer
    prompts_registered     integer
    extensions_loaded      integer
    surface_tools_registered    integer  @optional
    surface_resources_registered integer @optional
    auto_promoted_tools         integer  @optional
  }


  verify integration "mcp initialization emits event with tool counts"

}

event mcp_mutation_completed "MCP Mutation Completed" {
  channel   "mcp.mutation_completed"

  payload {
    toolName               string
    files_changed          integer
    entities_affected      integer
    success                boolean
    timestamp              timestamp
  }


  verify integration "emits mcp_mutation_completed with structured outcome after each mutation tool"

}

// ── MCP Subscription Lifecycle Events ────────────────────────

event mcp_subscription_created "MCP Subscription Created" {
  channel   "mcp.subscription_created"

  payload {
    subscriptionType   string
    clientId           string
    timestamp          timestamp
  }


  verify integration "emits mcp_subscription_created when a client subscribes to delta notifications"

}

event mcp_subscription_removed "MCP Subscription Removed" {
  channel   "mcp.subscription_removed"

  payload {
    subscriptionType   string
    clientId           string
    timestamp          timestamp
  }


  verify integration "emits mcp_subscription_removed when a client unsubscribes or server shuts down"

}

event mcp_initialization_failed "MCP Initialization Failed" {
  channel   "mcp.initialization_failed"

  payload {
    error_kind         string
    message            string
    timestamp          timestamp
  }


  verify integration "emits mcp_initialization_failed when MCP server fails to initialize"

}

event mcp_protocol_error_handled "MCP Protocol Error Handled" {
  channel   "mcp.protocol_error_handled"

  payload {
    errorCode      integer
    errorMessage   string
    method         string    @optional
    timestamp      timestamp
  }


  verify integration "emits mcp_protocol_error_handled with correct errorCode for each error type"

}

event mcp_request_cancelled "MCP Request Cancelled" {
  channel   "mcp.request_cancelled"

  payload {
    requestId      string
    wasInProgress  boolean
    timestamp      timestamp
  }


  verify integration "emits mcp_request_cancelled with correct requestId and wasInProgress flag"

}

event mcp_server_shutdown "MCP Server Shutdown" {
  channel   "mcp.shutdown"

  payload {
    pending_notifications_flushed  integer
    subscriptions_released         integer
    wasm_engines_released          integer
    timestamp                      timestamp
  }


  verify integration "emits mcp_server_shutdown with correct counts when MCP server shuts down"

}

