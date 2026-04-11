// Surface contribution behaviors — CLI commands, MCP tools, MCP resources
//
// 10 behaviors for Phase 1 of the surface contribution model (RES-24).
// Extensions declare surface contributions in their manifest's `surfaces`
// field. Core discovers, validates, and dispatches to Wasm exports using
// the cmd__{id} and mcp__{name} naming conventions.

use "invariants/surface"
use "invariants/wasm"
use "invariants/mcp"
use "types/surface"
use "types/wasm"
use "types/zero-entity-core"
use "types/mcp"
use "types/errors"
use "ports/inbound"
use "ports/outbound"
use "events/surface-contributions"
use "events/wasm-extensions"
// ── Registration & Validation ───────────────────────────────

behavior register_surface_contributions "Register Surface Contributions" {
  invariants [surface_contribution_uniqueness]
  category   command
  types      [ManifestV2, SurfaceContributions, SurfaceRegistryEntry, SurfaceType, SurfaceError]
  consumes   [manifest_loaded]
  produces   [surface_contributions_registered]

  requires {
    manifest_loaded_fired "manifest_loaded event has fired, confirming extension manifests are parsed and available"
  }

  ensures {
    all_surfaces_registered "All CLI commands, MCP tools, and MCP resources from manifest surfaces fields are registered in the SurfaceRegistry"
    duplicates_detected "Duplicate contribution names within each surface type across extensions produce E039"
    surface_contributions_registered_emitted "surface_contributions_registered event is emitted after successful registration"
  }

  contract """
    When extension manifests are loaded, the compiler MUST parse the
    surfaces field from each manifest and register all declared CLI
    commands, MCP tools, and MCP resources in the SurfaceRegistry.
    Registration MUST detect duplicate contribution names within each
    surface type across all extensions — duplicates MUST produce E039.
    Registration MUST happen after manifest loading but before Wasm
    module loading, because surface contributions are manifest-driven
    (declarative) like entity kinds.
  """

  verify unit "commands parsed from manifest surfaces field"
  verify unit "MCP tools parsed from manifest surfaces field"
  verify unit "MCP resources parsed from manifest surfaces field"
  verify unit "duplicate command ID across extensions produces E039"
  verify unit "duplicate MCP tool name across extensions produces E039"
  verify unit "registration succeeds with no duplicates"
  verify contract "requires/ensures consistency for surface contribution registration"

}

behavior validate_surface_exports "Validate Surface Exports" {
  invariants [surface_sandbox_ceiling, host_function_type_safety]
  category   validation
  types      [ManifestV2, SurfaceContributions, SurfaceError]
  ports      [WasmRuntime]
  consumes   [extension_loaded]
  produces   [surface_exports_validated, surface_export_validation_failed]

  requires {
    extension_loaded_fired "extension_loaded event has fired, confirming the Wasm module is loaded and its exports are inspectable"
  }

  ensures {
    all_declared_exports_verified "Every function declared in surface contributions has a corresponding Wasm export"
    missing_exports_diagnosed "Missing cmd__ or mcp__ exports produce E036 diagnostics"
    surface_exports_validated_emitted "surface_exports_validated event is emitted when all exports are present"
  }

  contract """
    After loading an extension's Wasm module, the compiler MUST verify
    that the .wasm binary exports all functions declared in the extension's
    surface contributions. CLI commands MUST have cmd__{id} exports. MCP
    tools and resources MUST have mcp__{name} exports. Missing exports
    MUST produce E036 diagnostics listing the expected export name. Extra
    exports beyond declared surfaces MUST be ignored. Extensions with no
    surfaces field are trivially valid. If the extension has no Wasm binary
    but declares surfaces, W055 MUST be emitted.
  """

  verify unit "all declared cmd__ exports present passes"
  verify unit "all declared mcp__ exports present passes"
  verify unit "missing cmd__ export produces E036"
  verify unit "missing mcp__ export produces E036"
  verify unit "no Wasm binary with surface declarations produces W055"
  verify unit "extra exports beyond surfaces are ignored"
  verify contract "requires/ensures consistency for surface export validation"

}

behavior validate_mcp_tool_schemas "Validate MCP Tool Schemas" {
  invariants [surface_schema_validity]
  category   validation
  types      [McpToolContribution, SurfaceError, JsonSchema]
  consumes   [surface_contributions_registered]
  produces   [mcp_tool_schemas_validated]

  requires {
    surface_contributions_registered_fired "surface_contributions_registered event has fired, confirming all MCP tool contributions are in the SurfaceRegistry"
  }

  ensures {
    schemas_validated "Every MCP tool input_schema is validated as valid JSON Schema"
    invalid_schemas_diagnosed "Invalid JSON Schemas produce E037 diagnostics"
    missing_descriptions_warned "MCP tools without descriptions produce W056 warnings"
    mcp_tool_schemas_validated_emitted "mcp_tool_schemas_validated event is emitted after schema validation completes"
  }

  contract """
    After surface contributions are registered, the compiler MUST validate
    the input_schema of each MCP tool contribution. The input_schema MUST
    be valid JSON Schema. Invalid schemas MUST produce E037. MCP tools
    without a description MUST produce W056 — agents need descriptions
    for tool discovery.
  """

  verify unit "valid JSON Schema passes validation"
  verify unit "invalid JSON Schema produces E037"
  verify unit "MCP tool without description produces W056"
  verify contract "requires/ensures consistency for MCP tool schema validation"

}

behavior validate_command_arg_types "Validate Command Arg Types" {
  invariants [surface_schema_validity]
  category   validation
  types      [CommandContribution, CommandArg, CommandArgType, SurfaceError]
  consumes   [surface_contributions_registered]
  produces   [command_args_validated]

  requires {
    surface_contributions_registered_fired "surface_contributions_registered event has fired, confirming all command contributions are in the SurfaceRegistry"
  }

  ensures {
    arg_types_validated "Every command arg has a known CommandArgType"
    unknown_types_diagnosed "Unknown arg types produce E038 diagnostics"
    command_args_validated_emitted "command_args_validated event is emitted after arg type validation completes"
  }

  contract """
    After surface contributions are registered, the compiler MUST validate
    the arg type declarations on each CLI command contribution. Each arg
    MUST have a known CommandArgType (string_arg, path_arg, bool_arg,
    enum_arg, integer_arg). Unknown arg types MUST produce E038. Commands
    with no args declaration MUST produce W057 as a style warning.
  """

  verify unit "known arg types pass validation"
  verify unit "unknown arg type produces E038"
  verify unit "command with no args produces W057"
  verify contract "requires/ensures consistency for command arg type validation"

}

// ── Auto-Promotion ──────────────────────────────────────────

behavior auto_promote_commands_to_mcp_tools "Auto-Promote Commands to MCP Tools" {
  invariants [surface_contribution_uniqueness]
  category   command
  types      [CommandContribution, AutoPromotedMcpTool, SurfaceRegistryEntry]
  consumes   [surface_contributions_registered]
  produces   [commands_auto_promoted]

  requires {
    surface_contributions_registered_fired "surface_contributions_registered event has fired, confirming all CLI command and MCP tool contributions are registered"
  }

  ensures {
    all_commands_promoted "Every CLI command contribution is auto-promoted to an MCP tool"
    naming_convention_enforced "Auto-promoted tools follow the specforge.{ext_short}.{cmd_id} naming pattern"
    explicit_tool_wins "Explicit MCP tool contributions take precedence over auto-promoted tools with I017 emitted"
    commands_auto_promoted_emitted "commands_auto_promoted event is emitted after promotion completes"
  }

  contract """
    After surface contributions are registered, the compiler MUST
    auto-promote every CLI command contribution to an MCP tool with
    the naming convention specforge.{ext_short}.{cmd_id}. The derived
    input_schema MUST be computed from the command's args declaration.
    If an explicit MCP tool contribution already exists with the same
    name, the explicit tool MUST win and I017 MUST be emitted. Auto-
    promoted tools appear in list_mcp_tools alongside explicit tools.
  """

  verify unit "CLI command auto-promoted to MCP tool"
  verify unit "auto-promoted tool name follows specforge.{ext}.{cmd} pattern"
  verify unit "derived input_schema computed from command args"
  verify unit "explicit MCP tool wins over auto-promoted tool with I017"
  verify contract "requires/ensures consistency for command-to-MCP-tool auto-promotion"

}

// ── Dispatch ────────────────────────────────────────────────

behavior dispatch_surface_command "Dispatch Surface Command" {
  invariants [surface_sandbox_ceiling, wasm_sandbox_integrity, extension_isolation]
  category   command
  types      [CommandContribution, CommandInput, CommandOutput, SurfaceError, WasmTrapInfo]
  ports      [WasmRuntime]
  consumes   [surface_exports_validated, command_args_validated]
  produces   [surface_command_dispatched]

  requires {
    surface_exports_validated_fired "surface_exports_validated event has fired, confirming cmd__ exports are present in the Wasm binary"
    command_args_validated_fired "command_args_validated event has fired, confirming arg types are valid for this extension"
  }

  ensures {
    args_serialized "Command arguments are serialized as JSON and passed to the cmd__ export"
    sandbox_restricted "Per-command sandbox override is intersected with extension policy (can only restrict, never expand)"
    traps_caught "Wasm traps are caught and reported as ExtensionError diagnostics"
    output_returned "Exit code, stdout, and stderr are returned to the CLI caller"
    surface_command_dispatched_emitted "surface_command_dispatched event is emitted after command execution completes"
  }

  contract """
    When a CLI command from an extension is invoked, the compiler MUST
    lazily load the extension's Wasm module (if not already loaded),
    serialize the command arguments as JSON, and call the cmd__{id}
    export. The per-command sandbox override (if declared) MUST be
    intersected with the extension's SandboxPolicy — the override can
    only restrict, never expand. Wasm traps MUST be caught and reported
    as ExtensionError diagnostics. The command's exit code, stdout, and
    stderr MUST be returned to the CLI caller.

    BARRIER: This behavior MUST NOT execute until both
    validate_surface_exports and validate_command_arg_types have
    completed for the extension.
  """

  verify unit "lazy Wasm load on first command invocation"
  verify unit "args serialized as JSON to cmd__ export"
  verify unit "sandbox override intersected with extension policy"
  verify unit "Wasm trap caught and reported as ExtensionError"
  verify unit "exit code, stdout, stderr returned to CLI"
  verify contract "requires/ensures consistency for surface command dispatch"

}

behavior dispatch_surface_mcp_tool "Dispatch Surface MCP Tool" {
  invariants [surface_sandbox_ceiling, wasm_sandbox_integrity, extension_isolation, mcp_structured_error_responses]
  category   command
  types      [McpToolContribution, SurfaceError, WasmTrapInfo, JsonSchema]
  ports      [WasmRuntime, McpProtocol]
  consumes   [surface_exports_validated, mcp_tool_schemas_validated, commands_auto_promoted]
  produces   [surface_mcp_tool_dispatched]

  requires {
    surface_exports_validated_fired "surface_exports_validated event has fired, confirming mcp__ exports are present in the Wasm binary"
    mcp_tool_schemas_validated_fired "mcp_tool_schemas_validated event has fired, confirming input schemas are valid JSON Schema"
    commands_auto_promoted_fired "commands_auto_promoted event has fired, confirming auto-promoted tools are registered"
  }

  ensures {
    input_validated "Input is validated against the tool's declared input_schema before dispatch"
    sandbox_restricted "Per-tool sandbox override is intersected with extension policy (can only restrict, never expand)"
    traps_as_mcp_errors "Wasm traps are caught and returned as structured MCP error responses"
    tool_result_returned "Tool output is returned as a standard MCP tool result"
    surface_mcp_tool_dispatched_emitted "surface_mcp_tool_dispatched event is emitted after tool execution completes"
  }

  contract """
    When an MCP tool contributed by an extension is invoked, the MCP
    server MUST validate the input against the tool's declared
    input_schema, lazily load the extension's Wasm module (if not
    already loaded), and call the mcp__{name} export with the validated
    input JSON. The per-tool sandbox override (if declared) MUST be
    intersected with the extension's SandboxPolicy. Wasm traps MUST be
    caught and returned as structured MCP error responses. The tool
    output MUST be returned as a standard MCP tool result.

    BARRIER: This behavior MUST NOT execute until both
    validate_surface_exports and validate_mcp_tool_schemas have
    completed for the extension.
  """

  verify unit "input validated against declared input_schema"
  verify unit "lazy Wasm load on first tool invocation"
  verify unit "input JSON passed to mcp__ export"
  verify unit "sandbox override intersected with extension policy"
  verify unit "Wasm trap returned as structured MCP error"
  verify unit "tool output returned as MCP tool result"
  verify contract "requires/ensures consistency for surface MCP tool dispatch"

}

behavior dispatch_surface_mcp_resource "Dispatch Surface MCP Resource" {
  invariants [surface_sandbox_ceiling, wasm_sandbox_integrity, extension_isolation, mcp_structured_error_responses]
  category   command
  types      [McpResourceContribution, SurfaceError, WasmTrapInfo]
  ports      [WasmRuntime, McpProtocol]
  consumes   [surface_exports_validated]
  produces   [surface_mcp_resource_dispatched]

  requires {
    surface_exports_validated_fired "surface_exports_validated event has fired, confirming mcp__ exports are present in the Wasm binary"
  }

  ensures {
    uri_matched "Requested URI is matched against registered URI templates"
    fs_write_denied "MCP resources have no fs_write access regardless of extension policy"
    traps_as_mcp_errors "Wasm traps are caught and returned as structured MCP error responses"
    content_returned "Resource content and mime_type are returned to the MCP client"
    surface_mcp_resource_dispatched_emitted "surface_mcp_resource_dispatched event is emitted after resource read completes"
  }

  contract """
    When an MCP resource contributed by an extension is read, the MCP
    server MUST match the requested URI against registered URI templates,
    lazily load the extension's Wasm module (if not already loaded), and
    call the mcp__{name} export with the URI. MCP resources MUST NOT
    have fs_write access — the sandbox ceiling for resources denies
    writes. Wasm traps MUST be caught and returned as structured MCP
    error responses. The resource content and mime_type MUST be returned
    to the MCP client.

    BARRIER: This behavior MUST NOT execute until
    validate_surface_exports has completed for the extension.
  """

  verify unit "URI matched against registered templates"
  verify unit "lazy Wasm load on first resource read"
  verify unit "URI passed to mcp__ export"
  verify unit "fs_write denied for resource contributions"
  verify unit "Wasm trap returned as structured MCP error"
  verify unit "resource content and mime_type returned to client"
  verify contract "requires/ensures consistency for surface MCP resource dispatch"

}

// ── Sandbox Enforcement ─────────────────────────────────────

behavior enforce_surface_sandbox "Enforce Surface Sandbox" {
  invariants [surface_sandbox_ceiling, wasm_sandbox_integrity]
  category   command
  types      [SurfaceSandboxOverride, SandboxPolicy, SurfaceType, SurfaceError]
  produces   [surface_permission_denied]

  requires {
    sandbox_policy_available "Extension's SandboxPolicy is loaded and accessible for intersection"
    surface_type_known "The surface type (CLI command, MCP tool, or MCP resource) is determined for ceiling lookup"
  }

  ensures {
    effective_sandbox_computed "Effective sandbox is the intersection of per-contribution override, extension policy, and surface-type ceiling"
    ceiling_enforced "Per-contribution overrides cannot expand beyond the surface-type ceiling"
    denial_event_produced "Attempts to exceed the ceiling produce a surface_permission_denied event and deny the call"
  }

  contract """
    Before dispatching any surface contribution Wasm export, the compiler
    MUST compute the effective sandbox by intersecting the per-contribution
    override with the extension's SandboxPolicy and the surface-type
    ceiling. Surface-type ceilings: MCP resources cannot fs_write. CLI
    commands have no additional ceiling beyond the extension policy. MCP
    tools have no additional ceiling beyond the extension policy.
    Per-contribution overrides can only restrict below the ceiling, never
    expand. Attempts to exceed the ceiling MUST produce a
    surface_permission_denied event and deny the call.
  """

  verify unit "effective sandbox is intersection of override, policy, and ceiling"
  verify unit "MCP resource fs_write attempt denied"
  verify unit "per-contribution override cannot expand beyond extension policy"
  verify unit "permission denial produces surface_permission_denied event"
  verify contract "requires/ensures consistency for surface sandbox enforcement"

}

// ── Configuration ───────────────────────────────────────────

behavior toggle_surface_contributions "Toggle Surface Contributions" {
  invariants [surface_contribution_uniqueness]
  category   command
  types      [SurfaceRegistryEntry, SurfaceType]
  ports      [CompilerApi]
  consumes   [surface_contributions_registered]
  produces   [surface_contribution_toggled]

  requires {
    surface_contributions_registered_fired "surface_contributions_registered event has fired, confirming surfaces are available in the registry for toggling"
  }

  ensures {
    disabled_excluded "Disabled contributions are excluded from CLI routing, MCP tool listing, and MCP resource listing"
    extension_still_loaded "The extension remains loaded even when its contributions are disabled"
    reenable_without_restart "Re-enabling a contribution restores it to the registry without requiring a restart"
    surface_contribution_toggled_emitted "surface_contribution_toggled event is emitted after toggle completes"
  }

  contract """
    The specforge.json configuration MUST support enabling or disabling
    individual surface contributions. Disabled contributions MUST be
    excluded from CLI command routing, MCP tool listing, and MCP resource
    listing. The extension MUST still be loaded — only the disabled
    surface contributions are hidden. Re-enabling a contribution MUST
    restore it to the registry without requiring a restart.
  """

  verify unit "disabled command excluded from CLI routing"
  verify unit "disabled MCP tool excluded from tool listing"
  verify unit "disabled MCP resource excluded from resource listing"
  verify unit "re-enabled contribution restored without restart"
  verify contract "requires/ensures consistency for surface contribution toggling"

}
