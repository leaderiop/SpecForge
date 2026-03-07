// MCP operation behaviors — mutations and project management queries
//
// 11 behaviors:
//   - Mutation Tools (6): format, rename, init, add_extension, remove_extension, migrate
//   - Project Management Tools (5): extensions, providers, doctor, collect, render

use invariants/core
use invariants/validation
use invariants/formatting
use invariants/mcp
use events/mcp
use types/graph
use types/output
use types/diagnostics
use types/mcp
use types/core
use types/config
use types/formatting
use ports/inbound
use ports/outbound

// ---------------------------------------------------------------------------
// Mutation Tools
// ---------------------------------------------------------------------------

behavior provide_mcp_format_tool "Provide MCP Format Tool" {
  invariants [diagnostic_determinism, formatting_idempotency, mcp_structured_error_responses]
  types      [McpFormatResult, FormatDiff, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi, FileSystem]
  produces   [mcp_tool_invoked, mcp_mutation_completed]

  contract """
    In MCP server mode, the system MUST register a specforge.format tool that
    accepts paths?[] (optional file paths, defaults to all), check? (optional
    boolean, report only without modifying), and diff? (optional boolean, return
    diffs). The tool MUST format spec files according to the canonical style.
    In check mode, the tool MUST NOT modify files. In diff mode, the tool MUST
    return FormatDiff entries for each changed file.
  """

  verify unit "specforge.format formats spec files"
  verify unit "check mode reports without modifying files"
  verify unit "diff mode returns FormatDiff entries"
  verify unit "paths filter restricts to specified files"

}

behavior provide_mcp_rename_tool "Provide MCP Rename Tool" {
  invariants [entity_id_uniqueness, graph_traversal_integrity, diagnostic_determinism, mcp_structured_error_responses]
  types      [McpRenameResult, TextEdit, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi, FileSystem]
  produces   [mcp_tool_invoked, mcp_mutation_completed]

  contract """
    In MCP server mode, the system MUST register a specforge.rename tool that
    accepts entity_id (required), new_name (required), and dry_run? (optional
    boolean, default false). The tool MUST rename the entity and update all
    references across all spec files. The response MUST include the list of
    TextEdit operations applied. When dry_run is true, the tool MUST return
    the rename plan (affected files and TextEdit operations) without applying
    any changes. If the entity does not exist, the tool MUST return an error.
    If new_name is invalid (e.g., not a legal entity ID), the tool MUST return
    a validation error.
  """

  verify unit "specforge.rename renames entity and all references"
  verify unit "non-existent entity returns error response"
  verify unit "invalid new_name returns validation error"
  verify unit "dry_run returns rename plan without applying changes"

}

// MCP init creates a project at a specified path, not the current project.
// The agent is connected to an existing project's MCP server and uses it
// to scaffold a new project elsewhere. For bootstrapping the very first
// project, use the CLI: specforge init.
behavior provide_mcp_init_tool "Provide MCP Init Tool" {
  invariants [diagnostic_determinism, init_config_validity, mcp_structured_error_responses]
  types      [McpInitResult, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi, FileSystem]
  produces   [mcp_tool_invoked, mcp_mutation_completed]

  contract """
    In MCP server mode, the system MUST register a specforge.init tool that
    accepts path (required, target directory for the new project), name
    (required), extensions?[] (optional list of extension names to install),
    and version? (optional, defaults to 0.1.0). The tool MUST accept a path
    parameter specifying the target directory for the new project. The path
    MUST be outside the current project's spec_root. If the path is inside
    the current project, the tool MUST return an error. The tool MUST create
    a new specforge.json project configuration file and scaffold the spec
    directory at the specified path. If extensions are specified, they MUST
    be added to the config and their manifests validated. MCP init is always
    non-interactive — extension selection is provided via the extensions
    parameter. Interactive extension selection (TTY prompting) is only
    available via the CLI init command.
  """

  verify unit "specforge.init creates specforge.json project"
  verify unit "extensions installed when specified"
  verify unit "default version is 0.1.0"
  verify unit "path inside current project returns error"

}

behavior provide_mcp_add_extension_tool "Provide MCP Add Extension Tool" {
  invariants [diagnostic_determinism, mcp_structured_error_responses]
  types      [McpExtensionInfo, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi, FileSystem]
  produces   [mcp_tool_invoked, mcp_mutation_completed]

  contract """
    In MCP server mode, the system MUST register a specforge.add_extension
    tool that accepts extension (required, name or path). The tool MUST add
    the extension to specforge.json, download the Wasm module if remote, and
    validate the extension manifest. If the manifest is invalid or the
    extension conflicts with an existing one, the tool MUST return an error.
  """

  verify unit "specforge.add_extension adds extension to config"
  verify unit "wasm module downloaded for remote extensions"
  verify unit "invalid manifest returns error"

}

behavior provide_mcp_remove_extension_tool "Provide MCP Remove Extension Tool" {
  invariants [diagnostic_determinism, mcp_structured_error_responses]
  types      [McpRemoveExtensionResult, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi, FileSystem]
  produces   [mcp_tool_invoked, mcp_mutation_completed]

  contract """
    In MCP server mode, the system MUST register a specforge.remove_extension
    tool that accepts extension (required, name). The tool MUST remove the
    extension from specforge.json. If removing the extension would leave
    orphan entities (entities of kinds only defined by that extension), the
    tool MUST include a warning in the response but still proceed.
  """

  verify unit "specforge.remove_extension removes extension from config"
  verify unit "orphan entities produce a warning"

}

behavior provide_mcp_migrate_tool "Provide MCP Migrate Tool" {
  invariants [diagnostic_determinism, mcp_structured_error_responses]
  types      [MigrationResult, MigrationSummary, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi, FileSystem]
  produces   [mcp_tool_invoked, mcp_mutation_completed]

  contract """
    In MCP server mode, the system MUST register a specforge.migrate tool
    that accepts dry_run? (optional boolean, default false). The tool MUST
    detect and apply pending migrations to spec files. In dry_run mode, the
    tool MUST return the diff without modifying any files. After migration,
    the tool MUST validate the result and report any post-migration errors.
  """

  verify unit "specforge.migrate applies pending migrations"
  verify unit "dry_run returns diff without modifying files"
  verify unit "post-migration validation reports errors"

}

// ---------------------------------------------------------------------------
// Project Management Tools
// ---------------------------------------------------------------------------

behavior provide_mcp_extensions_tool "Provide MCP Extensions Tool" {
  invariants [diagnostic_determinism, mcp_structured_error_responses, mcp_tool_idempotency]
  types      [McpExtensionInfo, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]

  contract """
    In MCP server mode, the system MUST register a specforge.extensions tool
    with no required parameters. The tool MUST return a list of all installed
    extensions including name, version, entity kinds contributed, and status.
    The response MUST reflect the current specforge.json configuration.
  """

  verify unit "specforge.extensions lists all installed extensions"
  verify unit "each entry includes name, version, entity kinds, and status"

}

behavior provide_mcp_providers_tool "Provide MCP Providers Tool" {
  invariants [diagnostic_determinism, mcp_structured_error_responses, mcp_tool_idempotency]
  types      [McpProviderInfo, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]

  contract """
    In MCP server mode, the system MUST register a specforge.providers tool
    with no required parameters. The tool MUST return a list of all configured
    providers including scheme, alias, extension, and status. Providers supply
    external reference validation via registered schemes.
  """

  verify unit "specforge.providers lists all configured providers"
  verify unit "each entry includes scheme, alias, extension, and status"

}

behavior provide_mcp_doctor_tool "Provide MCP Doctor Tool" {
  invariants [diagnostic_determinism, mcp_structured_error_responses, mcp_tool_idempotency]
  types      [McpDoctorReport, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]

  contract """
    In MCP server mode, the system MUST register a specforge.doctor tool with
    no required parameters. The tool MUST check project health: extension
    conflicts, stale Wasm cache entries, missing specforge.json fields, version
    mismatches, and orphan entities. The response MUST include detected issues
    and deterministic resolution steps.
  """

  verify unit "specforge.doctor detects extension conflicts"
  verify unit "response checks wasm cache integrity"
  verify unit "response provides deterministic resolution steps"

}

behavior provide_mcp_collect_tool "Provide MCP Collect Tool" {
  invariants [diagnostic_determinism, mcp_structured_error_responses]
  types      [McpCollectResult, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi, FileSystem]
  produces   [mcp_tool_invoked]

  contract """
    In MCP server mode, the system MUST register a specforge.collect tool that
    accepts extension (required), format? (optional), and path (required, path
    to test results). The extension parameter selects which installed extension
    provides the collector contribution (e.g., "@specforge/rust"). The format
    parameter selects the specific collector within that extension when it
    registers multiple collectors (e.g., format="junit" vs format="cargo-json");
    when omitted, the extension's default collector MUST be used. The tool MUST
    delegate to the extension's registered collector contribution to parse the
    test results file and map tests to spec entities using the extension's
    entity-to-test mapping, and emit a specforge-report.json.

    Example: specforge.collect(extension="@specforge/rust", format="junit",
    path="target/test-results.xml") invokes the junit collector from the
    @specforge/rust extension.

    If the extension is not installed or does not provide a collector, the tool
    MUST return an error. If the format is unrecognized by the extension, the
    tool MUST return an error listing available formats. If the path is invalid,
    the tool MUST return an error.
  """

  verify unit "specforge.collect parses test results and maps to entities"
  verify unit "emits specforge-report.json"
  verify unit "invalid path returns error"
  verify unit "unrecognized format returns error listing available formats"
  verify unit "unknown extension returns error"

}

behavior provide_mcp_render_tool "Provide MCP Render Tool" {
  invariants [graph_traversal_integrity, diagnostic_determinism, mcp_structured_error_responses]
  types      [McpRenderResult, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi, FileSystem]
  produces   [mcp_tool_invoked]

  contract """
    In MCP server mode, the system MUST register a specforge.render tool that
    accepts format (required, a format string matching a registered renderer)
    and out_dir (required, output directory path). The tool MUST invoke the
    matching registered renderer and write output files to out_dir.
    The json and dot renderers are provided by the core graph engine (see P7
    justification in features/output.spec); additional renderers come from extensions.
    Output files are graph serialization formats (JSON, DOT) and extension-contributed
    non-code reports (traceability matrices, dashboards, coverage summaries) — never
    source code, configuration files, or documentation artifacts.
    Unrecognized format strings MUST return an error listing available renderers.
    The response MUST list all files written.
  """

  verify unit "specforge.render writes output files to out_dir"
  verify unit "registered renderer invoked for matching format"
  verify unit "unrecognized format returns error listing available renderers"

}
