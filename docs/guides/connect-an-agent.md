# Connect an agent

What you get: Claude, or any AI agent that speaks [MCP](https://modelcontextprotocol.io), reads your
project's graph, asks it questions, and changes it — and every change it makes is checked against your
extensions' rules before anything is written. The agent is the worker; SpecForge is the memory and the
referee.

This guide covers the MCP server first, because that is how agents reach SpecForge today. The LSP and the
VS Code extension, for people editing by hand, come after. AG-UI and HTTP surfaces are planned, not built.

## 1. Start the server

`specforge mcp` serves one project over stdio (JSON-RPC, one message per line):

```bash
specforge mcp /path/to/your/project
```

The argument is a path inside the project: the server serves the nearest directory at or above it that holds
`specforge.json`. If there is none yet, it serves the project once one exists there (the agent can create it
with the `specforge.init` tool). You rarely run this yourself: your MCP client starts it.

## 2. Register it with your client

**Claude Code.** Use an absolute path:

```bash
claude mcp add specforge -- specforge mcp /path/to/your/project
```

Or commit a `.mcp.json` at the project root, so everyone who opens the project gets it:

```json
{
  "mcpServers": {
    "specforge": { "command": "specforge", "args": ["mcp", "."] }
  }
}
```

**Claude Desktop, Cursor and other clients** take the same `command` and `args` in their MCP settings
(`claude_desktop_config.json` for Claude Desktop). Cursor can pass `${workspaceFolder}` as the path.

Check it worked: ask the agent to "list the SpecForge tools". Some clients show the tool names with
underscores (`specforge_validate`) instead of dots (`specforge.validate`); they are the same tools.

The server negotiates MCP protocol versions 2025-03-26, 2025-06-18 and 2025-11-25 through `initialize`, and
also serves 2026-07-28 (stateless) requests (ADR 0004, D4-c).

## 3. What the agent gets

### Tools

The server exposes 35 core tools, plus the commands of the extensions your project enables.

| Job | Tools |
|---|---|
| **Read the graph** | `specforge.query` (an entity and its neighbours, to a depth), `specforge.inspect` (one entity in full), `specforge.list` (entities by kind and field values), `specforge.search` (by id, title or text), `specforge.trace` (what an entity traces to; or check an agent's plan for gaps), `specforge.export`, `specforge.stats`, `specforge.outline` (a file's entities) |
| **Learn the vocabulary** | `specforge.schema` (every kind the loaded extensions declare, with its fields and edges), `specforge.model`, `specforge.extensions`, `specforge.outline_extensions`, `specforge.explain` (a diagnostic code) |
| **Change the graph** | `specforge.edit` (typed edits: create, set, unset, retitle, link, unlink, add_verify, remove_verify, delete — all or nothing), `specforge.rename` (an entity, across every file), `specforge.format`, `specforge.migrate`, `specforge.suggest_fixes` (the fixes the LSP offers, with their edits) |
| **Check it** | `specforge.validate` (recompile and report diagnostics), `specforge.analyze` (analysis passes, coverage among them; the SMT `--prove` pass is CLI-only), `specforge.coverage`, `specforge.doctor` |
| **Navigate to the source** | `specforge.find_definition`, `specforge.find_references`, `specforge.find_implementation`, `specforge.find_spec_for_source` |
| **Set up the project** | `specforge.init`, `specforge.add_extension`, `specforge.remove_extension`, `specforge.providers` |
| **Proof and inference** | `specforge.collect` (record what the project's tests prove; it runs a test runner only with `run: true` and your prior approval), `specforge.infer_session`, `specforge.infer_progress`, `specforge.infer_gaps` (guides for writing specs from existing code) |

**Extension commands become tools.** Each command an enabled extension declares is served as
`specforge.<extension>.<command>`. With `@specforge/product`, an agent can call
`specforge.product.milestone_completion` or `specforge.product.health` and get the same answer
`specforge product milestone-completion` prints. Your own extension's commands arrive the same way, the
moment it is installed ([Extending SpecForge](extending-specforge.md#a-command)).

### Resources and prompts

- **Resources:** `specforge://graph`, `specforge://schema`, `specforge://context`, `specforge://brief` and
  `specforge://diagnostics`, plus templates for one entity (`specforge://graph/{entity_id}`,
  `specforge://context/{entity_id}`) and one kind (`specforge://entities/{kind}`). A client that subscribes
  hears when they change.
- **Prompts:** `specforge://prompts/context`, `review`, `trace`, `explore` and `infer`: ready-made
  instructions for common jobs (load the context of an entity, review coverage gaps, explore the graph,
  infer specs from code).

## 4. The validate-every-edit loop

This is the point of connecting an agent through SpecForge rather than letting it edit files.

1. **Read first.** The agent asks `specforge.schema` which kinds exist, then `specforge.query` or
   `specforge.search` for the entities it needs. It works from facts, not from guessing what a file says.
2. **Edit through `specforge.edit`.** One call carries a list of edits. SpecForge plans them on the project
   as it is, compiles the result, and writes **only if the result adds no error**. If it would, the call is
   refused with `E080`, the new errors are listed, and nothing is written.
3. **Fix and retry.** The refusal names what is wrong (an unresolved reference, a missing required field, a
   value the field does not take). The agent fixes it — often by making both changes in the same call — and
   tries again.
4. **Report what is left.** A successful edit returns the diagnostics it added (warnings and advice it did not
   block on), so the agent can tell you what still needs attention.

A refused edit looks like this (shortened):

```json
{
  "code": "compilation_failed",
  "diagnostic": {
    "code": "E080",
    "message": "the edit would leave the project with 1 error it does not have; nothing was written\n  error[E003]: unresolved reference 'tasks_are_dated' in entity 'create_task'"
  },
  "data": { "files_written": [] }
}
```

and the same change done right is one call with two edits:

```json
{
  "edits": [
    { "op": "create", "kind": "invariant", "id": "tasks_are_dated", "title": "Tasks are dated",
      "file": "spec/tasks.spec", "fields": { "guarantee": "Every task MUST carry its creation date." } },
    { "op": "link", "entity_id": "create_task", "field": "invariants", "target": "tasks_are_dated" }
  ]
}
```

The rule holds for every author: the CLI's `specforge new`, `set`, `unset`, `link`, `unlink` and `delete` are
the same typed edits and refuse the same way. An agent that writes `.spec` files directly instead skips the
check until it calls `specforge.validate`, so ask it to use `specforge.edit`, or to validate after every
change.

A good first instruction:

> Use the SpecForge tools. Read `specforge.schema` to learn which kinds this project has. Make every change
> with `specforge.edit`. If an edit is refused, fix what it names and try again. When you're done, run
> `specforge.validate` and tell me what is still a warning.

### What the agent cannot do

- **It cannot bypass the rules.** The same validation runs whoever makes the change.
- **It cannot run your tests silently.** `specforge.collect` runs a runner only with `run: true` and an
  approval you gave for that project and command by running `specforge collect` in a terminal; otherwise
  it reads an existing report.
- **It reaches only the served project.** Paths a call names resolve inside the project, and the server
  refuses ones that escape it (ADR 0092).
- **It cannot reach outside systems through SpecForge.** Extensions run sandboxed with no file system or
  network; SpecForge does not connect to your CRM or quality system. The agent may have other tools for
  that; SpecForge is where it records and checks what it learns.

## 5. For people: the LSP and VS Code

`specforge-lsp` is a Language Server for `.spec` files: diagnostics as you type, hover, completion of kinds,
fields and ids, go to definition, find references, rename, code actions (the fixes `specforge.suggest_fixes`
returns), code lenses, document and workspace symbols, semantic highlighting and formatting. It reads the same
extensions as the CLI and reloads them when `specforge.json`, `specforge.lock` or an extension changes. Any
editor with an LSP client can use it.

The VS Code extension (`integrations/vscode/`) bundles the client: it starts `specforge-lsp`, adds commands to
initialize, check and export a project, and shows the graph, an entity explorer and stats. Set
`specforge.binDir` if the `specforge` and `specforge-lsp` binaries are not on your `PATH`.

## 6. Planned: AG-UI and HTTP

Two more surfaces are planned and **not built**:

- **AG-UI**, so an agent's work on the graph can be shown and steered in an interface built for that.
- **HTTP**, so other systems and web interfaces can read and change the graph without an MCP client.

Until they exist, people who don't use an editor or a terminal reach SpecForge through an AI agent connected
over MCP, as above.

## See also

- [Model your domain](model-your-domain.md) — give the agent kinds that fit your business.
- [Authoring `.spec` files](authoring-spec-files.md) — the language the graph is written in.
- [Troubleshooting](spec-troubleshooting.md) — what the diagnostics an agent hits mean.
