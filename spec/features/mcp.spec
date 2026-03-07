// MCP Server features

use behaviors/mcp-operations
use behaviors/mcp-prompts
use behaviors/mcp-server
use behaviors/mcp-tools

feature mcp_resource_exposure "MCP Resource Exposure" {
  behaviors [expose_graph_as_mcp_resource, expose_schema_as_mcp_resource, expose_context_as_mcp_resource, expose_brief_as_mcp_resource, expose_diagnostics_as_mcp_resource, expose_entity_as_mcp_resource]

  problem """
    AI agents using MCP need resource-based access to the spec graph, schema,
    and diagnostics without invoking CLI commands. Without MCP resources, agents
    must shell out to specforge export, adding latency and complexity.
  """

  solution """
    Six MCP resources — specforge://graph, specforge://schema, specforge://context,
    specforge://brief, specforge://diagnostics, and specforge://graph/{entity_id} —
    expose the compiled graph in multiple resolutions. Agents read these directly
    through MCP, receiving Graph Protocol JSON that refreshes on each recompilation.
  """
}

feature mcp_core_tools "MCP Core Tools" {
  behaviors [provide_mcp_query_tool, provide_mcp_validate_tool, provide_mcp_export_tool, provide_mcp_trace_tool, provide_mcp_search_tool, provide_mcp_schema_tool, provide_mcp_coverage_tool, provide_mcp_stats_tool]

  problem """
    Agents need to invoke core spec operations (query subgraphs, trigger validation,
    export formats, search entities, inspect schema, check coverage) through MCP.
    Without structured tools, agents must parse CLI stdout, losing type safety and
    structured error handling.
  """

  solution """
    Eight core MCP tools — specforge.query, specforge.validate, specforge.export,
    specforge.trace, specforge.search, specforge.schema, specforge.coverage, and
    specforge.stats — provide structured, typed access to all read-only compiler
    operations through the MCP protocol. Note: specforge.coverage operates on
    generic testable flags from the KindRegistry (populated by extension manifests),
    not domain-specific coverage concepts.
  """
}

feature mcp_navigation_tools "MCP Navigation Tools" {
  behaviors [provide_mcp_inspect_tool, provide_mcp_find_definition_tool, provide_mcp_find_references_tool, provide_mcp_outline_tool, provide_mcp_suggest_fixes_tool]

  problem """
    Agents need LSP-equivalent navigation capabilities — inspecting entities, finding
    definitions and references, browsing file outlines, and getting fix suggestions —
    without running an LSP client. These operations are essential for agents that
    modify spec files.
  """

  solution """
    Five navigation MCP tools — specforge.inspect, specforge.find_definition,
    specforge.find_references, specforge.outline, and specforge.suggest_fixes —
    mirror LSP navigation capabilities through the MCP protocol, giving agents
    the same code intelligence available to IDE users.
  """
}

feature mcp_mutation_tools "MCP Mutation Tools" {
  behaviors [provide_mcp_format_tool, provide_mcp_rename_tool, provide_mcp_init_tool, provide_mcp_add_extension_tool, provide_mcp_remove_extension_tool, provide_mcp_migrate_tool]

  problem """
    Agents that author or maintain spec projects need to format files, rename
    entities, initialize projects, manage extensions, and run migrations — all
    through MCP. Agents connected to an existing MCP server can initialize new
    projects at different paths. Without mutation tools, agents must shell out
    to CLI commands, losing structured responses and atomicity.
  """

  solution """
    Six mutation MCP tools — specforge.format, specforge.rename, specforge.init,
    specforge.add_extension, specforge.remove_extension, and specforge.migrate —
    expose all write operations through MCP with structured input/output, enabling
    agents to fully manage spec projects programmatically.
  """
}

feature mcp_project_management_tools "MCP Project Management Tools" {
  behaviors [provide_mcp_extensions_tool, provide_mcp_providers_tool, provide_mcp_doctor_tool, provide_mcp_collect_tool, provide_mcp_render_tool]

  problem """
    Agents managing spec projects need to list extensions, check providers,
    diagnose issues, collect test results, and render outputs — operations
    that provide project health, status, and output generation through MCP.
    Three tools are read-only queries (extensions, providers, doctor); two
    tools perform write operations (collect writes specforge-report.json,
    render writes output files to disk).
  """

  solution """
    Five project management MCP tools: specforge.extensions (read-only,
    lists installed extensions with entity counts), specforge.providers
    (read-only, lists configured providers with registered schemes),
    specforge.doctor (read-only, runs health checks and reports conflicts),
    specforge.collect (write, invokes a collector contribution to parse
    test results and produce specforge-report.json), and specforge.render
    (write, invokes renderer contributions to produce output files such as
    JSON, DOT, or extension-defined formats). Together they give agents full
    visibility into project configuration and health, plus test collection
    and rendering capabilities.
  """
}

feature mcp_delta_notifications "MCP Delta Notifications" {
  behaviors [notify_graph_delta_via_mcp, notify_diagnostics_delta_via_mcp]

  problem """
    Agents consuming the spec graph via MCP have no way to know when the graph
    or diagnostics change. Without change notifications, agents must poll for
    updates, wasting resources and introducing latency.
  """

  solution """
    Two MCP subscriptions — notifications/graph_changed and
    notifications/diagnostics_changed — deliver delta payloads to subscribed
    clients after each incremental rebuild. Agents receive only the diff,
    enabling efficient incremental context updates without re-reading the full
    graph or re-validating.
  """
}

feature mcp_prompts "MCP Prompts" {
  behaviors [provide_mcp_implement_prompt, provide_mcp_review_prompt, provide_mcp_trace_prompt, provide_mcp_explore_prompt]

  problem """
    Agents need guided workflows for common spec tasks — implementing an entity,
    reviewing test coverage, tracing a plan, or exploring the graph. Raw tool
    access requires agents to compose multi-step workflows themselves, increasing
    token usage and error rates.
  """

  solution """
    Four MCP prompts assemble structured context from multiple graph queries
    into single responses optimized for agent consumption. Each prompt returns
    graph-derived data (entities, relationships, coverage gaps, traceability
    chains) without directives or recommendations — agents interpret the data
    and decide on actions.
  """
}

feature mcp_lifecycle "MCP Lifecycle" {
  behaviors [mcp_initialize, mcp_shutdown]

  problem """
    The MCP server must manage its full lifecycle — initialization of the
    compiler, extension loading, tool/resource/prompt registration, and clean
    shutdown with resource cleanup. Without explicit lifecycle management,
    servers may leak Wasm engine instances, leave subscriptions dangling, or
    accept tool calls before the graph is ready.
  """

  solution """
    Two lifecycle behaviors — mcp_initialize and mcp_shutdown — bracket the
    MCP server session. Initialization compiles the project graph, loads
    extensions, and registers all MCP capabilities before accepting requests.
    Shutdown flushes pending notifications, releases all subscriptions and
    Wasm engine instances, and exits cleanly.
  """
}

feature mcp_discovery "MCP Discovery" {
  behaviors [list_mcp_resources, list_mcp_tools, list_mcp_prompts]

  problem """
    Agents connecting to the MCP server have no way to discover which tools,
    resources, and prompts are available. Because extensions dynamically
    register capabilities, the set of available operations varies per project.
    Without a discovery mechanism, agents must hardcode tool names or guess
    at available operations, leading to errors and wasted tokens.
  """

  solution """
    Three listing operations — list_mcp_resources, list_mcp_tools, and
    list_mcp_prompts — return structured descriptors for all registered MCP
    capabilities. Each descriptor includes the operation name, description,
    and schema, enabling agents to dynamically adapt to the project's
    installed extensions without prior knowledge of the available operations.
  """
}
