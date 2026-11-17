requirement FEATURE-073 "MCP Resource Exposure" {
  category "Feature"
  tier     v1
  text     """
    AI agents using MCP need resource-based access to the spec graph, schema,
    and diagnostics without invoking CLI commands. Without MCP resources, agents
    must shell out to specforge export, adding latency and complexity.

    Six MCP resources — specforge://graph, specforge://schema, specforge://context,
    specforge://brief, specforge://diagnostics, and specforge://graph/{+entity_id} —
    expose the compiled graph in multiple resolutions. Agents read these directly
    through MCP, receiving Graph Protocol JSON that refreshes on each recompilation.
  """
}

requirement FEATURE-064 "MCP Core Tools" {
  category "Feature"
  tier     v1
  text     """
    Agents need to invoke core spec operations (query subgraphs, trigger validation,
    export formats, search entities, inspect schema, check coverage) through MCP.
    Without structured tools, agents must parse CLI stdout, losing type safety and
    structured error handling.

    Eight core MCP tools — specforge.query, specforge.validate, specforge.export,
    specforge.trace, specforge.search, specforge.schema, specforge.coverage, and
    specforge.stats — provide structured, typed access to all read-only compiler
    operations through the MCP protocol. Note: specforge.coverage operates on
    generic testable flags from the KindRegistry (populated by extension manifests),
    not domain-specific coverage concepts.
  """
}

requirement FEATURE-069 "MCP Navigation Tools" {
  category "Feature"
  tier     v1
  text     """
    Agents need LSP-equivalent navigation capabilities — inspecting entities, finding
    definitions and references, browsing file outlines, and getting fix suggestions —
    without running an LSP client. These operations are essential for agents that
    modify spec files.

    Five navigation MCP tools — specforge.inspect, specforge.find_definition,
    specforge.find_references, specforge.outline, and specforge.suggest_fixes —
    mirror LSP navigation capabilities through the MCP protocol, giving agents
    the same code intelligence available to IDE users.
  """
}

requirement FEATURE-068 "MCP Mutation Tools" {
  category "Feature"
  tier     v1
  text     """
    Agents that author or maintain spec projects need to format files, rename
    entities, initialize projects, manage extensions, and run migrations — all
    through MCP. Agents connected to an existing MCP server can initialize new
    projects at different paths. Without mutation tools, agents must shell out
    to CLI commands, losing structured responses and atomicity.

    Nine mutation MCP tools — specforge.format, specforge.rename,
    specforge.edit, specforge.init, specforge.add_extension,
    specforge.remove_extension, specforge.settings_set, specforge.migrate, and
    specforge.infer_session (which records inference progress in
    specforge-infer.json, project state no source regenerates) — expose all
    write operations through MCP with structured input/output, enabling
    agents to fully manage spec projects programmatically.
  """
}

requirement FEATURE-070 "MCP Project Management Tools" {
  category "Feature"
  tier     v1
  text     """
    Agents managing spec projects need to list extensions, check providers,
    diagnose issues, collect test results, and render outputs — operations
    that provide project health, status, and output generation through MCP.
    Three tools are read-only queries (extensions, providers, doctor); two
    tools write output artifacts (collect writes specforge-report.json,
    render writes output files to disk). These are output-artifact writes,
    not spec-source mutations — they produce mcp_tool_invoked events, not
    mcp_mutation_completed, because they do not modify .spec source files.

    Five project management MCP tools: specforge.extensions (read-only,
    lists installed extensions with entity counts), specforge.providers
    (read-only, lists configured providers with registered schemes),
    specforge.doctor (read-only, runs health checks and reports conflicts),
    specforge.collect (writes output artifact specforge-report.json by
    invoking a collector contribution to parse test results), and
    specforge.render (writes the graph as JSON, DOT, context or brief
    output, the formats specforge export writes, to files or inline).
    Together they give agents full visibility into project configuration
    and health, plus test collection and rendering capabilities.
  """
}

requirement FEATURE-065 "MCP Delta Notifications" {
  category "Feature"
  tier     v1
  text     """
    Agents consuming the spec graph via MCP have no way to know when the graph
    or diagnostics change. Without change notifications, agents must poll for
    updates, wasting resources and introducing latency.

    A client that subscribes to a resource hears
    notifications/resources/updated when a rebuild changes it, in both MCP
    eras (resources/subscribe, subscriptions/listen). A handshake subscriber
    of a graph view or of the diagnostics also receives the delta itself,
    specforge/graphChanged or specforge/diagnosticsChanged, so agents
    receive only the diff, enabling efficient incremental context updates
    without re-reading the full graph or re-validating.
  """
}

requirement FEATURE-071 "MCP Prompts" {
  category "Feature"
  tier     v1
  text     """
    Agents need guided workflows for common spec tasks — implementing an entity,
    reviewing test coverage, tracing a plan, or exploring the graph. Raw tool
    access requires agents to compose multi-step workflows themselves, increasing
    token usage and error rates.

    Four MCP prompts assemble structured context from multiple graph queries
    into single responses optimized for agent consumption. Each prompt returns
    graph-derived data (entities, relationships, coverage gaps, traceability
    chains) without directives or recommendations — agents interpret the data
    and decide on actions.
  """
}

requirement FEATURE-072 "MCP Protocol Compliance" {
  category "Feature"
  tier     v1
  text     """
    The MCP server implements SpecForge-specific tools and resources but does not
    model standard JSON-RPC 2.0 / MCP protocol edge cases: malformed requests,
    request cancellation, and duplicate initialization. Without explicit handling,
    these scenarios may crash the server or produce non-standard error responses.

    Four protocol compliance behaviors — handle_mcp_protocol_error for standard
    JSON-RPC error codes, handle_mcp_request_cancellation for notifications/cancelled,
    guard_mcp_reinitialization for rejecting duplicate initialize requests, and
    follow_negotiated_mcp_revision for what the negotiated protocol revision
    changes on the wire (batches, structuredContent) — ensure the server adheres
    to the MCP protocol specification and remains robust under adversarial or
    misbehaving clients.
  """
}

requirement FEATURE-067 "MCP Lifecycle" {
  category "Feature"
  tier     v1
  text     """
    The MCP server must manage its full lifecycle — initialization of the
    compiler, extension loading, tool/resource/prompt registration, and clean
    shutdown with resource cleanup. Without explicit lifecycle management,
    servers may leak Wasm engine instances, leave subscriptions dangling, or
    accept tool calls before the graph is ready.

    Two lifecycle behaviors — mcp_initialize and mcp_shutdown — bracket the
    MCP server session. Initialization compiles the project graph, loads
    extensions, registers all MCP capabilities (including surface-contributed
    tools and resources from extension manifests), and merges them with core
    capabilities before accepting requests. Shutdown flushes pending
    notifications, releases all subscriptions and Wasm engine instances,
    and exits cleanly.
  """
}

requirement FEATURE-066 "MCP Discovery" {
  category "Feature"
  tier     v1
  text     """
    Agents connecting to the MCP server have no way to discover which tools,
    resources, and prompts are available. Because extensions dynamically
    register capabilities, the set of available operations varies per project.
    Without a discovery mechanism, agents must hardcode tool names or guess
    at available operations, leading to errors and wasted tokens.

    Three listing operations — list_mcp_resources, list_mcp_tools, and
    list_mcp_prompts — return structured descriptors for all registered MCP
    capabilities, including extension-contributed tools and resources from
    surface contributions and auto-promoted CLI commands. Each descriptor
    includes the operation name, description, and schema, enabling agents
    to dynamically adapt to the project's installed extensions without prior
    knowledge of the available operations.
  """
}
