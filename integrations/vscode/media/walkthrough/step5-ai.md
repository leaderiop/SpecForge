# Connect AI Agents

SpecForge exists to give AI agents **structured context** instead of prose. The entity graph dramatically improves first-attempt accuracy from ~30% (prose) to 70-85% (structured graph).

## Export formats

Three export formats serve different use cases:

| Format | Token Cost | Use Case |
|--------|-----------|----------|
| **graph** | Full | Complete JSON graph with all entities, edges, and metadata |
| **context** | Medium | Optimized context window for Claude, Copilot, and other LLMs |
| **brief** | Minimal | Summary with entity counts, key relationships, and coverage stats |

### Exporting from VS Code

1. Run **SpecForge: Export Graph** from the Command Palette
2. Select a format (graph, context, or brief)
3. The output appears in the terminal

### Exporting from the CLI

```bash
# Full graph for programmatic consumption
specforge export --format=graph

# Token-optimized context for AI agents
specforge export --format=context

# Quick summary
specforge export --format=brief
```

## MCP Server for Claude

SpecForge includes a built-in **Model Context Protocol (MCP)** server that auto-starts when you open a SpecForge project. This gives Claude direct access to your entity graph.

### How it works

When `specforge.mcp.autoStart` is enabled (the default):

1. The MCP server starts alongside the language server
2. Claude can query your entities, relationships, and diagnostics
3. Responses include structured entity data, not just text

### Available MCP tools

Claude can use tools like:
- `specforge_inspect` -- get full details about an entity
- `specforge_search` -- find entities by name or kind
- `specforge_list` -- list all entities with filters
- `specforge_trace` -- follow entity relationships through the graph
- `specforge_coverage` -- check verification coverage
- `specforge_query` -- run graph queries

### Available MCP resources

Claude can read resources like entity definitions, extension schemas, and project stats -- all via structured URIs.

## Plan validation

When an AI agent produces an implementation plan, validate it against your spec:

```bash
specforge trace --plan plan.json
```

This checks that the plan covers the right entities, respects dependencies, and does not miss critical behaviors.

## What is next

- Write more specs as your project grows
- Use `specforge check` in CI to validate specs on every commit
- Explore `specforge coverage` to see which entities have verification
- Run **SpecForge: Run Doctor** if anything seems off
