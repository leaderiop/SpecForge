// Surface contribution events — CLI command, MCP tool, and MCP resource lifecycle

use "types/surface"
use "types/mcp"
event surface_contributions_registered "Surface Contributions Registered" {
  channel   "surface.contributions_registered"

  payload {
    extensionName       string
    commandCount        integer
    mcpToolCount        integer
    mcpResourceCount    integer
  }


  verify integration "emits surface_contributions_registered with correct counts per extension"

}

event surface_exports_validated "Surface Exports Validated" {
  channel   "surface.exports_validated"

  payload {
    extensionName       string
    validatedExports    integer
  }


  verify integration "emits surface_exports_validated after all cmd__ and mcp__ exports verified"

}

event surface_export_validation_failed "Surface Export Validation Failed" {
  channel   "surface.export_validation_failed"

  payload {
    extensionName       string
    missingExports      string[]
    declaredSurfaces    string[]
  }


  verify integration "emits surface_export_validation_failed with missing export names"

}

event mcp_tool_schemas_validated "MCP Tool Schemas Validated" {
  channel   "surface.mcp_tool_schemas_validated"

  payload {
    extensionName       string
    validCount          integer
    invalidCount        integer
  }


  verify integration "emits mcp_tool_schemas_validated with valid and invalid counts"

}

event command_args_validated "Command Args Validated" {
  channel   "surface.command_args_validated"

  payload {
    extensionName       string
    commandCount        integer
    warningCount        integer
  }


  verify integration "emits command_args_validated with correct command and warning counts"

}

event commands_auto_promoted "Commands Auto-Promoted" {
  channel   "surface.commands_auto_promoted"

  payload {
    promotedCount       integer
    conflictCount       integer
  }


  verify integration "emits commands_auto_promoted with correct promoted and conflict counts"

}

event surface_command_dispatched "Surface Command Dispatched" {
  channel   "surface.command_dispatched"

  payload {
    extensionName       string
    commandId           string
    exitCode            integer
    durationMs          integer
  }


  verify integration "emits surface_command_dispatched with correct commandId and exitCode"

}

event surface_mcp_tool_dispatched "Surface MCP Tool Dispatched" {
  channel   "surface.mcp_tool_dispatched"

  payload {
    extensionName       string
    toolName            string
    durationMs          integer
    success             boolean
  }


  verify integration "emits surface_mcp_tool_dispatched with correct toolName and success"

}

event surface_mcp_resource_dispatched "Surface MCP Resource Dispatched" {
  channel   "surface.mcp_resource_dispatched"

  payload {
    extensionName       string
    uriTemplate         string
    mimeType            string    @optional
    durationMs          integer
  }


  verify integration "emits surface_mcp_resource_dispatched with correct uriTemplate"

}

event surface_permission_denied "Surface Permission Denied" {
  channel   "surface.permission_denied"

  payload {
    extensionName       string
    surfaceType         string
    contributionId      string
    deniedCapability    string
    reason              string
  }


  verify integration "emits surface_permission_denied with correct denied capability and reason"

}

event surface_contribution_toggled "Surface Contribution Toggled" {
  channel   "surface.contribution_toggled"

  payload {
    extensionName       string
    surfaceType         string
    contributionName    string
    enabled             boolean
  }


  verify integration "emits surface_contribution_toggled with correct enabled state"

}
