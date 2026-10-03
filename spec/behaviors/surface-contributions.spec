// Surface contribution behaviors — CLI commands, MCP tools, MCP resources
//
// 9 behaviors for Phase 1 of the surface contribution model (RES-24).
// Extensions declare surface contributions in their manifest's `surfaces`
// field. Core discovers, validates, and dispatches to Wasm exports using
// the cmd__{id} and mcp__{name} naming conventions.

use "events/surface-contributions"
use "events/wasm-extensions"
use "invariants/mcp"
use "invariants/surface"
use "invariants/wasm"
use "ports/inbound"
use "ports/outbound"
use "types/errors"
use "types/mcp"
use "types/surface"
use "types/wasm"
use "types/zero-entity-core"

// ── Registration & Validation ───────────────────────────────

behavior register_surface_contributions "Register Surface Contributions" {
  features   [surface_contributions]
  invariants [surface_contribution_uniqueness]
  category   command
  types      [ManifestV2, SurfaceContributions, SurfaceRegistryEntry, SurfaceType, SurfaceError]
  consumes   [manifest_loaded]
  produces   [surface_contributions_registered]
  requires {
    manifest_loaded_fired "manifest_loaded event has fired, confirming extension manifests are parsed and available"
  }
  ensures {
    all_surfaces_registered                  "All CLI commands, MCP tools, and MCP resources from manifest surfaces fields are registered in the SurfaceRegistry"
    duplicates_detected                      "Duplicate contribution names within each surface type across extensions produce E039"
    surface_contributions_registered_emitted "surface_contributions_registered event is emitted after successful registration"
  }
  contract   """
    When extension manifests are loaded, the compiler MUST parse the
    surfaces field from each manifest and register all declared CLI
    commands, MCP tools, and MCP resources in the SurfaceRegistry.
    Registration MUST detect duplicate contribution names within each
    surface type across all extensions — duplicates MUST produce E039.
    Registration happens when the project's registries are built from
    the manifests its extensions describe, before any surface export is
    dispatched: surface contributions are declarative, like entity
    kinds.
  """
  verify unit "commands parsed from manifest surfaces field"
  verify unit "MCP tools parsed from manifest surfaces field"
  verify unit "MCP resources parsed from manifest surfaces field"
  verify unit "duplicate command ID across extensions produces E039"
  verify unit "duplicate MCP tool name across extensions produces E039"
  verify unit "registration succeeds with no duplicates"
  verify contract "Register Surface Contributions: surface contribution registration holds — manifest_loaded_fired, all_surfaces_registered, duplicates_detected, surface_contributions_registered_emitted"
}

behavior validate_surface_exports "Validate Surface Exports" {
  features   [surface_contributions]
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
    all_declared_exports_verified     "Every function declared in surface contributions has a corresponding Wasm export"
    missing_exports_diagnosed         "Missing cmd__ or mcp__ exports produce E020 diagnostics"
    surface_exports_validated_emitted "surface_exports_validated event is emitted when all exports are present"
  }
  contract   """
    After loading an extension's Wasm module, the compiler MUST verify
    that the .wasm binary exports all functions declared in the extension's
    surface contributions. CLI commands MUST have cmd__{id} exports. MCP
    tools and resources MUST have mcp__{name} exports. Missing exports
    MUST produce E020 diagnostics listing the expected export name. Extra
    exports beyond declared surfaces MUST be ignored. Extensions with no
    surfaces field are trivially valid. If the extension has no Wasm binary
    but declares surfaces, W055 MUST be emitted.
  """
  verify unit "all declared cmd__ exports present passes"
  verify unit "all declared mcp__ exports present passes"
  verify unit "missing cmd__ export produces E020"
  verify unit "missing mcp__ export produces E020"
  verify unit "no Wasm binary with surface declarations produces W055"
  verify unit "extra exports beyond surfaces are ignored"
  verify contract "Validate Surface Exports: surface export validation holds — extension_loaded_fired, all_declared_exports_verified, missing_exports_diagnosed, surface_exports_validated_emitted"
}

behavior validate_mcp_tool_schemas "Validate MCP Tool Schemas" {
  features   [surface_contributions]
  invariants [surface_schema_validity]
  category   validation
  types      [McpToolContribution, SurfaceError, JsonSchema]
  consumes   [surface_contributions_registered]
  produces   [mcp_tool_schemas_validated]
  requires {
    surface_contributions_registered_fired "surface_contributions_registered event has fired, confirming all MCP tool contributions are in the SurfaceRegistry"
  }
  ensures {
    schemas_validated                  "Every MCP tool input_schema is validated as valid JSON Schema"
    invalid_schemas_diagnosed          "Invalid JSON Schemas produce E055 diagnostics"
    missing_descriptions_warned        "MCP tools without descriptions produce W056 warnings"
    mcp_tool_schemas_validated_emitted "mcp_tool_schemas_validated event is emitted after schema validation completes"
  }
  contract   """
    After surface contributions are registered, the compiler MUST validate
    the input_schema of each MCP tool contribution. The input_schema MUST
    be valid JSON Schema. Invalid schemas MUST produce E055. MCP tools
    without a description MUST produce W056 — agents need descriptions
    for tool discovery.
  """
  verify unit "valid JSON Schema passes validation"
  verify unit "invalid JSON Schema produces E055"
  verify unit "MCP tool without description produces W056"
  verify contract "Validate MCP Tool Schemas: MCP tool schema validation holds — surface_contributions_registered_fired, schemas_validated, invalid_schemas_diagnosed, missing_descriptions_warned, mcp_tool_schemas_validated_emitted"
}

behavior validate_command_arg_types "Validate Command Arg Types" {
  features   [surface_contributions]
  invariants [surface_schema_validity]
  category   validation
  types      [CommandContribution, CommandArg, CommandArgType, SurfaceError]
  consumes   [surface_contributions_registered]
  produces   [command_args_validated]
  requires {
    surface_contributions_registered_fired "surface_contributions_registered event has fired, confirming all command contributions are in the SurfaceRegistry"
  }
  ensures {
    arg_types_validated            "Every command arg has a known CommandArgType"
    unknown_types_diagnosed        "Unknown arg types produce E055 diagnostics"
    command_args_validated_emitted "command_args_validated event is emitted after arg type validation completes"
  }
  contract   """
    After surface contributions are registered, the compiler MUST validate
    the arg type declarations on each CLI command contribution. Each arg
    MUST have a known CommandArgType (string_arg, path_arg, bool_arg,
    enum_arg, integer_arg). Unknown arg types MUST produce E055. Commands
    with no args declaration MUST produce W057 as a style warning.
  """
  verify unit "known arg types pass validation"
  verify unit "unknown arg type produces E055"
  verify unit "command with no args produces W057"
  verify contract "Validate Command Arg Types: command arg type validation holds — surface_contributions_registered_fired, arg_types_validated, unknown_types_diagnosed, command_args_validated_emitted"
}

// ── Auto-Promotion ──────────────────────────────────────────

behavior auto_promote_commands_to_mcp_tools "Auto-Promote Commands to MCP Tools" {
  features   [surface_contributions]
  invariants [surface_contribution_uniqueness]
  category   command
  types      [CommandContribution, AutoPromotedMcpTool, SurfaceRegistryEntry]
  consumes   [surface_contributions_registered]
  produces   [commands_auto_promoted]
  requires {
    surface_contributions_registered_fired "surface_contributions_registered event has fired, confirming all CLI command and MCP tool contributions are registered"
  }
  ensures {
    all_commands_promoted          "Every CLI command contribution is auto-promoted to an MCP tool"
    naming_convention_enforced     "Auto-promoted tools follow the specforge.{ext_short}.{cmd_id} naming pattern"
    explicit_tool_wins             "Explicit MCP tool contributions take precedence over auto-promoted tools with I017 emitted"
    commands_auto_promoted_emitted "commands_auto_promoted event is emitted after promotion completes"
  }
  contract   """
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
  verify contract "Auto-Promote Commands to MCP Tools: command-to-MCP-tool auto-promotion holds — surface_contributions_registered_fired, all_commands_promoted, naming_convention_enforced, explicit_tool_wins, commands_auto_promoted_emitted"
}

// ── Dispatch ────────────────────────────────────────────────

behavior dispatch_surface_command "Dispatch Surface Command" {
  features   [surface_contributions]
  invariants [surface_sandbox_ceiling, wasm_sandbox_integrity, extension_isolation]
  category   command
  types      [CommandContribution, CommandInput, CommandOutput, SurfaceError, WasmTrapInfo]
  ports      [WasmRuntime]
  produces   [surface_command_dispatched]
  requires {
    command_declared "The command is one an extension the project enables declares in its surfaces, read when the project's environment loaded, and the configuration does not disable"
  }
  ensures {
    args_serialized                    "Command arguments, the project root and the graph are serialized as JSON and passed to the cmd__ export"
    sandbox_restricted                 "The cmd__ export is granted no capability (no preopened directory, environment, arguments, stdin or network), whatever sandbox override its declaration asks for"
    traps_caught                       "Wasm traps are caught and reported as ExtensionError diagnostics"
    output_returned                    "Exit code, stdout, and stderr are returned to the caller"
    surface_command_dispatched_emitted "Over MCP, a surface_command_dispatched event records the command and its exit code once its export returns; the CLI has no event sink"
  }
  contract   """
    When a CLI command from an extension is invoked, the host MUST
    serialize the command's input as JSON (CommandInput: its args, the
    project root and the compiled graph) and call the cmd__{id} export,
    in the runtime that loaded the project's extensions to read their
    declarations: only the extensions the project enables are loaded,
    each compiled once per process (from the wasm compile cache when it
    is warm), and dispatching a command loads no module. The CLI routes
    specforge {ext_short} {command} to it, the command line built from
    the declared args; an auto-promoted MCP tool runs the same export
    with its arguments as the args, over the served graph. The export
    MUST be granted no capability: its WASI context preopens no
    directory and passes no environment, arguments, inherited stdio or
    network, so cwd is a path it is told, not one it can open, and the
    graph is all it reads. A per-command sandbox override (if declared)
    is not applied: with nothing granted there is nothing for it to
    withhold, and no override grants more (surface_sandbox_ceiling).
    Wasm traps MUST be caught and reported as ExtensionError
    diagnostics. The command's exit code, stdout, and stderr MUST be
    returned to the caller. The MCP server records each command whose
    export returned as a surface_command_dispatched event.
  """
  verify unit "the command runs in the runtime that read the project's declarations, which loaded only the extensions the project enables"
  verify unit "args serialized as JSON to cmd__ export"
  verify unit "a cmd__ export is granted no capability, whatever sandbox its declaration asks for"
  verify unit "Wasm trap caught and reported as ExtensionError"
  verify unit "exit code, stdout, stderr returned to CLI"
  verify contract "Dispatch Surface Command: surface command dispatch holds — command_declared, args_serialized, sandbox_restricted, traps_caught, output_returned, surface_command_dispatched_emitted"
}

behavior dispatch_surface_mcp_tool "Dispatch Surface MCP Tool" {
  features   [surface_contributions]
  invariants [
    surface_sandbox_ceiling,
    wasm_sandbox_integrity,
    extension_isolation,
    mcp_structured_error_responses,
  ]
  category   command
  types      [McpToolContribution, SurfaceError, WasmTrapInfo, JsonSchema]
  ports      [WasmRuntime, McpProtocol]
  consumes   [surface_exports_validated, mcp_tool_schemas_validated, commands_auto_promoted]
  produces   [surface_mcp_tool_dispatched]
  requires {
    surface_exports_validated_fired  "surface_exports_validated event has fired, confirming mcp__ exports are present in the Wasm binary"
    mcp_tool_schemas_validated_fired "mcp_tool_schemas_validated event has fired, confirming input schemas are valid JSON Schema"
    commands_auto_promoted_fired     "commands_auto_promoted event has fired, confirming auto-promoted tools are registered"
  }
  ensures {
    input_validated                     "Input is validated against the tool's declared input_schema before dispatch"
    sandbox_restricted                  "The mcp__ export is granted no capability, whatever sandbox override its declaration asks for"
    traps_as_mcp_errors                 "Wasm traps are caught and returned as structured MCP error responses"
    tool_result_returned                "Tool output is returned as a standard MCP tool result"
    surface_mcp_tool_dispatched_emitted "surface_mcp_tool_dispatched event is emitted after tool execution completes"
  }
  contract   """
    When an MCP tool contributed by an extension is invoked, the MCP
    server MUST validate the input against the tool's declared
    input_schema and call the mcp__{name} export with the validated
    input JSON, in the runtime the served project's compile loaded. The
    export MUST be granted no capability, like every surface export: a
    per-tool sandbox override (if declared) is not applied, and no
    override grants more (surface_sandbox_ceiling). Wasm traps MUST be
    caught and returned as structured MCP error responses. The tool
    output MUST be returned as a standard MCP tool result. When the tool
    declares an output_schema, an output that does not match it MUST be
    returned as a schema_mismatch MCP error naming each violation, never
    as the tool's structured result.

    BARRIER: This behavior MUST NOT execute until both
    validate_surface_exports and validate_mcp_tool_schemas have
    completed for the extension.
  """
  verify unit "input validated against declared input_schema"
  verify unit "output that does not match the declared output_schema is a schema_mismatch error"
  verify unit "the served project's runtime is the one its compile loaded and serves later calls until the project reloads"
  verify unit "input JSON passed to mcp__ export"
  verify unit "an mcp__ tool export is granted no capability, whatever sandbox its declaration asks for"
  verify unit "Wasm trap returned as structured MCP error"
  verify unit "tool output returned as MCP tool result"
  verify contract "Dispatch Surface MCP Tool: surface MCP tool dispatch holds — surface_exports_validated_fired, mcp_tool_schemas_validated_fired, commands_auto_promoted_fired, input_validated, sandbox_restricted, traps_as_mcp_errors, tool_result_returned, surface_mcp_tool_dispatched_emitted"
}

behavior dispatch_surface_mcp_resource "Dispatch Surface MCP Resource" {
  features   [surface_contributions]
  invariants [
    surface_sandbox_ceiling,
    wasm_sandbox_integrity,
    extension_isolation,
    mcp_structured_error_responses,
  ]
  category   command
  types      [McpResourceContribution, SurfaceError, WasmTrapInfo]
  ports      [WasmRuntime, McpProtocol]
  consumes   [surface_exports_validated]
  produces   [surface_mcp_resource_dispatched]
  requires {
    surface_exports_validated_fired "surface_exports_validated event has fired, confirming mcp__ exports are present in the Wasm binary"
  }
  ensures {
    uri_matched                             "Requested URI is matched against registered URI templates"
    fs_write_denied                         "MCP resources have no fs_write access (no capability at all), whatever their sandbox override asks for"
    traps_as_mcp_errors                     "Wasm traps are caught and returned as structured MCP error responses"
    content_returned                        "Resource content and mime_type are returned to the MCP client"
    surface_mcp_resource_dispatched_emitted "surface_mcp_resource_dispatched event is emitted after resource read completes"
  }
  contract   """
    When an MCP resource contributed by an extension is read, the MCP
    server MUST match the requested URI against registered URI templates
    and call the mcp__{name} export with the URI, in the runtime the
    served project's compile loaded. MCP resources MUST NOT have
    fs_write access: like every surface export, a resource's export is
    granted no capability, whatever its sandbox override asks for
    (surface_sandbox_ceiling). Wasm traps MUST be caught and returned as structured MCP
    error responses. The resource content and mime_type MUST be returned
    to the MCP client.

    BARRIER: This behavior MUST NOT execute until
    validate_surface_exports has completed for the extension.
  """
  verify unit "URI matched against registered templates"
  verify unit "URI passed to mcp__ export"
  verify unit "fs_write denied for resource contributions"
  verify unit "Wasm trap returned as structured MCP error"
  verify unit "resource content and mime_type returned to client"
  verify contract "Dispatch Surface MCP Resource: surface MCP resource dispatch holds — surface_exports_validated_fired, uri_matched, fs_write_denied, traps_as_mcp_errors, content_returned, surface_mcp_resource_dispatched_emitted"
}

// ── Configuration ───────────────────────────────────────────

behavior toggle_surface_contributions "Toggle Surface Contributions" {
  features   [surface_contributions]
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
    disabled_excluded                    "Disabled contributions are excluded from CLI routing, MCP tool listing, and MCP resource listing"
    extension_still_loaded               "The extension remains loaded even when its contributions are disabled"
    reenable_without_restart             "Re-enabling a contribution restores it to the registry without requiring a restart"
    surface_contribution_toggled_emitted "surface_contribution_toggled event is emitted after toggle completes"
  }
  contract   """
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
  verify contract "Toggle Surface Contributions: surface contribution toggling holds — surface_contributions_registered_fired, disabled_excluded, extension_still_loaded, reenable_without_restart, surface_contribution_toggled_emitted"
}
