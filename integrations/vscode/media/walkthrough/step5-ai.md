# Connect AI Agents

SpecForge has no AI inside. Your AI agent (Claude, Copilot, Cursor, or any other) is the worker: it reads the graph, drafts and edits specs, and SpecForge checks every change it makes. Instead of re-reading prose, the agent asks for the exact slice it needs.

## Export formats

The VS Code command offers three export formats (the CLI also has `markdown`, a document a reviewer reads, and `dot`):

| Format | Token Cost | Use Case |
|--------|-----------|----------|
| **graph** | Full | Complete JSON graph with all entities, edges, and metadata |
| **context** | Medium | Every entity's fields and obligations without file and line, for an AI agent |
| **brief** | Minimal | Each entity's id, kind and title |

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

## MCP Server for your agent

SpecForge includes a built-in **Model Context Protocol (MCP)** server. Add `specforge mcp /path/to/your/project` to your agent's MCP configuration as a stdio command to give it direct access to your graph.

### How it works

1. Your agent starts `specforge mcp` for your project
2. It can query your entities, relationships, and diagnostics
3. It can edit specs, and every edit is validated: a change that breaks a rule is refused with the exact diagnostic
4. Responses include structured entity data, not just text

### Available MCP tools

Your agent can use tools like:
- `specforge.inspect` -- get full details about an entity
- `specforge.search` -- find entities by name or kind
- `specforge.list` -- list all entities with filters
- `specforge.trace` -- follow entity relationships through the graph
- `specforge.coverage` -- check verification coverage
- `specforge.query` -- run graph queries
- `specforge.validate` -- recompile and check the project
- `specforge.edit` -- make a typed edit that is validated before it lands

Extensions can add their own tools: with `@specforge/product` enabled, the agent also gets product queries such as features, journeys and milestones.

### Available MCP resources

Your agent can read resources such as the whole graph, the schema, the diagnostics, or one entity's context -- all via `specforge://` URIs.

## Plan validation

When an AI agent produces an implementation plan, validate it against your spec:

```bash
specforge trace --plan plan.json
```

This checks that the plan covers the right entities, respects dependencies, and does not miss critical behaviors.

## What is next

- Write more specs as your project grows
- Use `specforge check` in CI to validate specs on every commit
- Explore `specforge analyze coverage` to see which entities have verification
- Teach SpecForge your own domain: `specforge extension init` scaffolds an extension that declares your kinds and rules
- Run **SpecForge: Run Doctor** if anything seems off
