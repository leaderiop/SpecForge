# Engine tools are one catalogue below the surfaces, named by `specforge:` classes

**Status:** accepted (2026-11-12, R3 of the Beyond Parity build plan). **Extends**
[ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md) (a contribution's `tools` may name
SpecForge's own tools; install resolves them and writes a generated section) and
[ADR 0049](0049-a-core-tool-is-one-module.md) (a core tool is asserted to be its engine-tool row). Takes extension
protocol 2.1.0.

## Context

- The engine's tools had no single list. The MCP server holds its 39 core tools (`CORE_TOOLS`), the CLI its command
  tree, and the two were twins only by convention: `anchors` is `specforge.find_implementation`, `query --inputs` is
  `specforge.verdict_inputs`, `new`/`set`/`link`/… are `specforge.edit`.
- The host adapters knew one thing about SpecForge: Claude Code added `mcp__specforge__*` to every agent, skill and
  slash command. A read-only checker was granted all 80 tools of the server, 34 of them writing tools, and paid for
  about 209 KB of tool schemas on every spawn. Codex and Cursor agents were never told which SpecForge tools exist.
- An extension could not say which of SpecForge's tools an agent needs: the tool classes of ADR 0292 name the host's
  own tools (`read`, `shell`, …) and other MCP servers (`mcp:<server>/<glob>`), not the engine's.
- Nothing could hold a prompt to "name the engine's graph tools", because nothing said what those tools are.

## Decisions

- **D1. One catalogue.** `specforge_protocol_types::engine_tools::ENGINE_TOOLS` holds one row per engine tool, in
  `CORE_TOOLS` order: its name, its effect (reads, writes output, mutates), its group, the CLI lines that do the same
  thing (its CLI twins, each the words after `specforge`; empty for a tool only MCP serves) and one "use when"
  sentence. The SDK re-exports it. Every core tool is asserted at compile time to be its row (`Agrees::CHECKED`: the
  row exists, the effect agrees, the group agrees with the tool's MCP group), a test checks the names and their order,
  and a CLI test checks that every twin a row names is a declared command with its flags. `CoreTool` keeps its module
  (ADR 0049); the use-when line serves the generated section and does not replace the MCP description.
- **D2. Groups.** Every row is in one of five groups: `reads`, `navigation`, `edits`, `output` and `setup`. The graph
  tools are every group but `setup` (project creation, extensions, settings, migration, doctor, prompts, inference
  sessions). Groups are explicit membership, not MCP's listing groups, whose `management` mixes reads and writes.
- **D3. Classes.** An agent, skill or slash command's `tools` may name `specforge:<tool>`, `specforge:<group>` or
  `specforge:<short>.<command>` (a command a loaded extension declares). The registry checks each one, and each of
  these is W021 and drops the item: a class naming no tool, group or declared command; a human-attested or refused
  command; `mcp:specforge/<glob>` (a second spelling of the whole server); and any `specforge:` class on a hook, which
  matches the host's own tool calls.
- **D4. Resolution.** Install resolves each contribution to concrete engine tools. Named tools and each named group's
  rows come in catalogue order, then named commands by MCP name. A contribution that names no `specforge:` class gets
  the default: the `reads` and `navigation` tools, the `edits` and `output` tools when its host classes include
  `write` or `edit`, and its own extension's commands that are neither human-attested nor refused. A slash command
  also gets the tool its trampoline calls. An agent or skill that declares no tools keeps the host's whole set (a list
  would strip its host tools). A non-empty declaration never resolves to an empty list: one that resolves to no engine
  tool keeps its host classes plus `specforge:query`, since a host that writes no list grants everything.
- **D5. Per host.** Claude Code names each resolved tool: `mcp__specforge__` and the tool's MCP name with every
  character outside `[A-Za-z0-9_-]` written `_` (`specforge.trace` is `mcp__specforge__specforge_trace`); the server
  name is the one install writes into `.mcp.json`. No installed file grants `mcp__specforge__*`. Codex and Cursor carry
  no per-agent MCP list, so the generated section is what tells their agents.
- **D6. The section.** On every host, each installed agent and skill body is followed by a `## SpecForge tools`
  section: one line per resolved tool, naming its MCP name, its CLI line (or "MCP only") and when to use it. Sections
  come in one order: tools, then the context an agent reads, then its judged rules, so every later section names
  tools the first lists. A slash command has no section: its body is the trampoline.
- **D7. Protocol 2.1.0.** The classes are a wire addition a 2.0 host would refuse with W021 and drop the agent; at
  2.1.0 a guest naming them is refused whole by an older host (E088), which says which side is behind.

## Rejected

- **Deriving `CoreTool` from the catalogue.** It would split a core tool across two crates; the compile-time check
  already makes drift impossible.
- **A role table in the kernel.** Roles are an extension's: it names the classes each of its agents needs. The kernel
  owns the catalogue, the classes and the default.
- **Filtering `tools/list` per role on the server.** The host decides what a subagent sees; the server cannot tell
  one subagent's call from another's.
- **Keeping `mcp__specforge__*` for a contribution that names no class.** That is the blanket grant this record
  removes; the default names the read tools one by one.

## Consequences

- A read-only Claude Code subagent loads its own few SpecForge schemas (about 28–43 KB for GSD's roles) instead of
  the whole server's.
- A setup tool (init, add, remove, migrate, settings set, …) is granted only to a contribution that names it.
- Every installed agent and skill body gains the section, so an install made before it is reported stale.
- A core tool added without its catalogue row does not build; a row whose CLI twin is not a declared command fails a
  test.

## What would reopen it

A host that carries a per-agent MCP tool list other than Claude Code's (then its adapter names the tools too), or a
host that names MCP tools by another rule than D5's.
