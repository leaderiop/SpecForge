# SpecForge for VS Code

SpecForge turns what you know (the things you work with, how they connect, and the rules they must obey) into one **checked graph** that you and your AI agents read and change. A change that breaks a rule is refused, whoever made it.

SpecForge itself knows no domain. **Extensions** teach it one: software specs, a product plan, a marketing calendar, a regulated procedure, or your own. This extension is the editor for those `.spec` files.

## What you get

- **Live checking.** Every save is compiled; broken references, missing fields and rule violations show up as squiggles with the diagnostic code. **SpecForge: Explain Error** tells you what a code means and how to fix it.
- **Completion and navigation** for the kinds and fields your project's extensions declare: go to definition, find references, rename, hover, outline.
- **The graph.** **SpecForge: Show Graph** draws the entities and their links, grouped by kind, with filter chips; click to expand, double-click to jump to the definition.
- **Entity explorer, stats, coverage and trace** for the whole project, without leaving the editor.
- **Extensions.** Add or remove an extension from the command palette (**SpecForge: Add Extension**).
- **AI agents.** The same project is served to Claude or any MCP client by `specforge mcp`; the walkthrough shows how to connect one.

## Requirements

The `specforge` and `specforge-lsp` binaries, on your `PATH` or in the directory the `specforge.binDir` setting names. **SpecForge: Run Doctor** checks the setup.

## Get started

Open the walkthrough (**Help → Welcome → Get Started with SpecForge**), or run **SpecForge: Initialize Project** in an empty folder. It creates `specforge.json` and a first `spec/hello.spec` that checks clean.

## Commands

| Command | Does |
|---|---|
| Initialize Project | Create `specforge.json` and a first spec |
| Check Project | Compile and report every diagnostic |
| New Spec File | Add a `.spec` file inside the workspace |
| Show Graph / Focus Entity in Graph | Open the graph panel |
| Show Entity Explorer / Show Outline | Browse entities by kind or by file |
| Inspect Entity / Trace Entity / Search Entities | Look up one entity, its upstream chain, or a text match |
| Show Stats / Show Coverage / Show Model | Project counts, verify coverage, the declared kinds and fields |
| Explain Error | The meaning and fix of a diagnostic code |
| Export Graph / Copy Entity as JSON | The compiled graph for other tools |
| Add Extension / Remove Extension | Change what the project's extensions declare |
| Run Doctor / Restart Language Server | Check and reset the setup |

## Settings

| Setting | Default | Meaning |
|---|---|---|
| `specforge.binDir` | empty (use `PATH`) | Directory holding `specforge` and `specforge-lsp`; machine scope only |
| `specforge.graph.autoUpdate` | `true` | Redraw the graph as the project changes, saved or not |
| `specforge.lsp.trace` | `off` | Log LSP traffic, for debugging |

## Learn more

The SpecForge repository has the docs, the builtin extensions, and example projects in several domains: <https://github.com/leaderiop/SpecForge>.
