# PRD-006: VS Code Extension

## Overview

A comprehensive VS Code extension for SpecForge that provides full language support, graph visualization, and AI integration. Designed through a 20-expert panel evaluation.

## Architecture

**Extension identifier:** `specforge.specforge`
**Location:** `integrations/vscode/`

**Activation events:**
- `onLanguage:specforge`
- `workspaceContains:specforge.json`
- `workspaceContains:**/*.spec`

**Binary resolution order:**
1. `specforge.lsp.path` user setting
2. Platform-specific binary bundled in extension (`bin/{platform}-{arch}/specforge-lsp`)
3. `specforge-lsp` on `$PATH`

**Key technology choices:**

| Decision | Choice | Rationale |
|----------|--------|-----------|
| LSP transport | stdio | Already implemented in `specforge-lsp`, standard for binary language servers |
| Graph rendering | Cytoscape.js | Interactive, fast, compound graphs, rich layout options, MIT license |
| Extension bundler | esbuild | Fastest TS bundler, standard for VS Code extensions |
| TextMate grammar | Hand-written from tree-sitter queries | 1:1 mapping from existing `highlights.scm` |
| AI integration | 3-layer (MCP + Copilot participant + LM tools) | Covers Claude Code, Copilot, and any MCP client |
| Binary distribution | Platform-specific vsix | Follows rust-analyzer pattern, no install step |
| Testing | @vscode/test-electron + mocha | VS Code standard, runs in real VS Code instance |

## File Structure

```
integrations/vscode/
  .vscodeignore
  package.json
  tsconfig.json
  esbuild.mjs
  src/
    extension.ts          # activate/deactivate, lifecycle
    lsp-client.ts         # LanguageClient setup + binary resolution
    mcp-client.ts         # MCP server process management
    graph-webview.ts      # Cytoscape graph panel
    entity-tree.ts        # Entity explorer TreeDataProvider
    codelens.ts           # Reference count + verify status CodeLens
    status-bar.ts         # Health indicator + language status
    commands.ts           # All command registrations
    onboarding.ts         # Walkthrough + welcome views
    ai-integration.ts     # Copilot participant + LM tools
    config.ts             # Settings reader + specforge.json watcher
    telemetry.ts          # Opt-in usage analytics
  syntaxes/
    specforge.tmLanguage.json
  snippets/
    specforge.json
  schemas/
    specforge.schema.json
    specforge-report.schema.json
  media/
    specforge-icon.svg
    graph-panel/
      index.html
      graph.js            # Cytoscape + layout plugins
      graph.css
    walkthrough/
      step1-init.png
      step2-write.png
      step3-diagnostics.png
      step4-graph.png
      step5-ai.png
  test/
    suite/
      extension.test.ts
      lsp.test.ts
      grammar.test.ts
    fixtures/
      sample-project/
```

## Feature List (Prioritized)

### P0 -- v1.0: Core Language Support

| # | Feature | Implementation |
|---|---------|---------------|
| 1 | Language registration (.spec files) | `package.json` `languages` contribution |
| 2 | TextMate grammar (fallback highlighting) | `syntaxes/specforge.tmLanguage.json` |
| 3 | LSP client (all 15 methods) | `vscode-languageclient/node`, stdio transport |
| 4 | Semantic tokens (9 types, 2 modifiers) | LSP semantic tokens provider, theme mapping |
| 5 | Diagnostics in Problems panel | LSP `textDocument/publishDiagnostics` |
| 6 | Hover (entity info + graph edges + fields) | LSP `textDocument/hover` |
| 7 | Completion (entity IDs + keywords + fields) | LSP `textDocument/completion` |
| 8 | Go-to-definition | LSP `textDocument/definition` |
| 9 | Find references | LSP `textDocument/references` |
| 10 | Rename with prepare | LSP `textDocument/rename` + `textDocument/prepareRename` |
| 11 | Code actions (add verify, add import, create stub) | LSP `textDocument/codeAction` |
| 12 | Document symbols (outline) | LSP `textDocument/documentSymbol` |
| 13 | Workspace symbols | LSP `workspace/symbol` |
| 14 | Formatting + range formatting | LSP `textDocument/formatting` + `rangeFormatting` |
| 15 | Entity kind snippets (all 22 kinds) | `snippets/specforge.json` |
| 16 | `specforge.json` schema validation | `jsonValidation` contribution |
| 17 | Status bar health indicator | `vscode.window.createStatusBarItem` |
| 18 | Basic commands (init, check, graph, format, doctor) | `commands` contribution |
| 19 | File watcher for external changes | LSP `workspace/didChangeWatchedFiles` |
| 20 | Settings (lspPath, codeLens, formatOnSave, etc.) | `configuration` contribution |

### P1 -- v1.1: Visualization + Explorer

| # | Feature | Implementation |
|---|---------|---------------|
| 21 | Entity Explorer tree view (sidebar) | `TreeDataProvider` + Activity Bar icon |
| 22 | Interactive graph webview (Cytoscape.js) | `WebviewPanel` + `cose-bilkent` layout |
| 23 | CodeLens (reference count + verify status) | `CodeLensProvider` |
| 24 | Language status item (entity count, LSP version) | `vscode.languages.createLanguageStatusItem` |
| 25 | Getting Started walkthrough (5 steps) | `walkthroughs` contribution |
| 26 | Welcome view for empty explorer | `viewsWelcome` contribution |
| 27 | "Focus in Graph" command (Alt+G) | `specforge.focusInGraph` command |
| 28 | Graph layout switcher (force/dagre/concentric/grid) | Webview toolbar dropdown |
| 29 | Graph filtering (by kind, extension, depth) | Webview toolbar controls |
| 30 | Explain Error command | Opens `specforge explain` output in editor |

### P2 -- v1.2: AI Integration + Polish

| # | Feature | Implementation |
|---|---------|---------------|
| 31 | MCP server auto-start | Process management in `mcp-client.ts` |
| 32 | Copilot Chat participant (@specforge) | `vscode.chat.createChatParticipant` |
| 33 | Copilot LM Tool registration | `vscode.lm.registerTool` for 6 tools |
| 34 | Claude Code MCP config generation | Write to `.claude/settings.local.json` |
| 35 | Extension public API | `api` field in `package.json` |
| 36 | Custom SpecForge themes (dark + light) | `themes` contribution |
| 37 | Graph accessibility table view | Toggle in graph webview |
| 38 | Performance telemetry in status bar | Parse time, entity count display |
| 39 | "AI-powered quick fix" code action | Routes to Copilot Chat with context |
| 40 | Automatic AI context enrichment | Context injection for AI tools |

## UI Layout

```
+----------------------------------------------------------+
|  Activity Bar  |  Sidebar (SpecForge Explorer)            |
|                |                                          |
|  [SF icon]  -> |  ENTITIES                                |
|                |    > Behaviors (12)                      |
|                |      - authenticate_user                 |
|                |      - process_payment                   |
|                |    > Features (8)                        |
|                |    > Events (15)                         |
|                |    > Types (20)                          |
|                |    ...                                   |
|                |                                          |
|                |  DIAGNOSTICS                             |
|                |    ! 3 errors, 2 warnings                |
|                |                                          |
|                |  EXTENSIONS                              |
|                |    > @specforge/software (5 kinds)       |
|                |    > @specforge/product (9 kinds)        |
|                +------------------------------------------+
|                |  Editor                                  |
|                |                                          |
|                |  3 references | 2/3 verified | Graph     | <-- CodeLens
|                |  behavior authenticate_user "..." {      |
|                |    contract "Validates credentials..."   |
|                |    types [UserCredentials, AuthToken]     |
|                |    invariants [session_valid]             |
|                |    verify unit "happy path"               |
|                |  }                                       |
|                |                                          |
|                +------------------------------------------+
|                |  Panel (Graph Visualization)             |
|                |                                          |
|                |  [Force] [Dagre] [Grid] | Filter: [All] |
|                |                                          |
|                |     [authenticate_user] ---> [AuthToken] |
|                |           |                              |
|                |           v                              |
|                |     [session_valid]                      |
|                |                                          |
+----------------------------------------------------------+
|  Status Bar                                              |
|  [check] SpecForge | 142 entities | 4 extensions         |
+----------------------------------------------------------+
```

## Graph Visualization Design

**Technology:** Cytoscape.js 3.30+ with extensions:
- `cytoscape-cose-bilkent` (v4.1.0) -- Force-directed layout
- `cytoscape-dagre` (v2.5.0) -- Hierarchical layout
- `cytoscape-popper` (v2.0.0) -- Tooltips via Tippy.js

**Data pipeline:**
1. Extension calls `specforge.export --format=graph` (via MCP tool or CLI)
2. JSON graph is transformed to Cytoscape elements format
3. Sent to webview via `postMessage`

**Node rendering:**
- Shape by entity kind (ellipse=behavior, rectangle=type, diamond=event, hexagon=feature, triangle=invariant)
- Color by extension (software=#4285F4, product=#34A853, governance=#FBBC04, formal=#7B61FF)
- Size proportional to edge count (min 30px, max 80px)
- Label: entity ID (truncated to 20 chars)
- Badge: green check (all verified), red X (unverified), gray circle (no verify)

**Edge rendering:**
- Solid line for reference edges
- Dashed line for verify/trace edges
- Arrow at target end
- Label: edge type (on hover)
- Color: gray by default, highlighted on hover

**Interactions:**
- Click node: navigate to entity in editor
- Double-click node: expand neighborhood (fetch 1 more hop)
- Right-click node: context menu (Show References, Inspect, Copy ID, Focus)
- Pan: mouse drag on background
- Zoom: scroll wheel
- Keyboard: Tab cycles nodes, Enter opens, Arrow keys traverse edges, Escape exits

**Layout algorithms:**
- **Force (default):** `cose-bilkent` with `idealEdgeLength: 100`, `nodeRepulsion: 4500`, `animate: true`
- **Dagre:** Top-to-bottom hierarchy, `rankSep: 100`, `nodeSep: 50`
- **Concentric:** Rings by PageRank centrality
- **Grid:** Sorted alphabetically by kind then ID

**Performance:**
- Lazy loading: initial view shows 50 most-connected nodes, "Show All" button loads rest
- Web worker for layout computation (prevents UI freeze)
- Incremental updates: diff old/new graph, animate only changes
- Target: <200ms for graphs with <200 nodes, <1s for <1000 nodes

## AI Integration Design

### Three-layer AI integration

**Layer 1: MCP Server (Claude Code, any MCP client)**
- Auto-start `specforge mcp` as a child process
- Generate `.claude/settings.local.json` with MCP server config
- The MCP server exposes 27 tools, 7 resources, 4 prompts over stdio JSON-RPC
- Claude Code and other MCP clients discover it automatically

**Layer 2: Copilot Chat Participant (@specforge)**
- Register via `vscode.chat.createChatParticipant('specforge', handler)`
- Intent routing:
  - "query/find/show/what is" -> `specforge.query` / `specforge.inspect`
  - "check/validate/errors" -> `specforge.validate`
  - "trace/follow/dependency" -> `specforge.trace`
  - "coverage/gaps/untested" -> `specforge.coverage`
  - "explain/why/what does X mean" -> `specforge explain`
  - "graph/visualize/diagram" -> `specforge.model --format=mermaid`
  - "search" -> `specforge.search`
- Responses include Markdown with entity links (clickable, open in editor)

**Layer 3: Copilot LM Tools (automatic context)**
- Register tools via `vscode.lm.registerTool`:
  - `specforge_query` -- Query entity neighborhoods
  - `specforge_validate` -- Check for errors
  - `specforge_search` -- Fuzzy search entities
  - `specforge_inspect` -- Get full entity detail
  - `specforge_coverage` -- Coverage status
  - `specforge_trace` -- Traceability chain
- Copilot calls these tools automatically when relevant

**Context enrichment flow:**
1. User opens `auth.ts` and asks Copilot to implement authentication
2. Copilot detects SpecForge tools available
3. Copilot calls `specforge_search("authenticate")` -> finds `behavior authenticate_user`
4. Copilot calls `specforge_inspect("authenticate_user")` -> gets contract, types, invariants
5. Copilot generates code that matches the spec's contract, uses declared types, respects invariants
6. Result: AI output aligned with specification (the core value proposition)

## Onboarding Flow

**First-time user journey (target: productive in 60 seconds):**

1. **Install** (0s): User finds "SpecForge" on marketplace, clicks Install
2. **Detection** (0-2s): Extension checks workspace for `specforge.json` or `.spec` files
3. **Initialize** (2-10s): QuickPick for extension selection, creates config + starter files
4. **First edit** (10-30s): Syntax highlighting, snippets, diagnostics on save
5. **Graph exploration** (30-60s): `Ctrl+Shift+G` opens interactive graph panel
6. **AI integration** (60s+): `@specforge` in Copilot Chat, MCP auto-start for Claude

**Walkthrough (5 steps):**
1. Create Your First Project (completionEvent: `onCommand:specforge.init`)
2. Write Your First Entity (completionEvent: `onLanguage:specforge`)
3. Understand Diagnostics (completionEvent: `onView:workbench.panel.markers`)
4. Explore the Entity Graph (completionEvent: `onCommand:specforge.showGraph`)
5. Supercharge Your AI Agent (completionEvent: `onCommand:workbench.action.chat.open`)

## Extension Settings

```json
{
  "specforge.lsp.path": "",
  "specforge.lsp.trace": "off",
  "specforge.codeLens.enabled": true,
  "specforge.graph.defaultLayout": "force",
  "specforge.graph.autoUpdate": true,
  "specforge.format.onSave": false,
  "specforge.lint.profile": "default",
  "specforge.mcp.autoStart": true,
  "specforge.entityExplorer.groupBy": "kind"
}
```

## Commands (26 total)

| Command | ID | When | Keybinding |
|---|---|---|---|
| Initialize Project | `specforge.init` | `!specforge:projectDetected` | -- |
| Check Project | `specforge.check` | `specforge:projectDetected` | `Ctrl+Shift+B` |
| Export Graph | `specforge.export` | `specforge:hasEntities` | -- |
| Show Graph | `specforge.showGraph` | `specforge:hasEntities` | `Ctrl+Shift+G` |
| Show Entity Explorer | `specforge.showExplorer` | always | -- |
| Show Stats | `specforge.showStats` | `specforge:projectDetected` | -- |
| Format File | `specforge.formatFile` | `editorLangId == specforge` | `Shift+Alt+F` |
| Explain Error | `specforge.explainError` | `specforge:projectDetected` | -- |
| Run Doctor | `specforge.doctor` | always | -- |
| New Spec File | `specforge.newFile` | `specforge:projectDetected` | -- |
| Focus Entity in Graph | `specforge.focusInGraph` | `editorLangId == specforge` | `Alt+G` |
| Inspect Entity | `specforge.inspect` | `editorLangId == specforge` | `Alt+I` |
| Show Coverage | `specforge.coverage` | `specforge:hasEntities` | -- |
| Trace Entity | `specforge.trace` | `editorLangId == specforge` | -- |
| Search Entities | `specforge.search` | `specforge:projectDetected` | `Ctrl+Shift+S` |
| Show Model | `specforge.model` | `specforge:projectDetected` | -- |
| Add Extension | `specforge.addExtension` | `specforge:projectDetected` | -- |
| Remove Extension | `specforge.removeExtension` | `specforge:projectDetected` | -- |
| Restart LSP | `specforge.restartLsp` | always | -- |
| Copy Entity as JSON | `specforge.copyEntityJson` | `editorLangId == specforge` | -- |
| Show Outline | `specforge.outline` | `specforge:projectDetected` | -- |

## Marketplace Presentation

**Title:** SpecForge
**Subtitle:** Structured Specs for AI Agents
**Publisher:** specforge
**Categories:** Programming Languages, Linters, Visualization
**Tags:** spec, dsl, entity-graph, ai-agents, specification, behavior-driven, domain-model, structured-context, copilot, mcp

**Description:**
"Write .spec files to build typed entity graphs that give AI agents structured context. Full language support: highlighting, diagnostics, completion, graph visualization, and AI integration."

**Screenshots plan (7 images):**
1. Syntax highlighting with semantic tokens (dark theme)
2. Diagnostics with quick fix lightbulb
3. Entity graph visualization (Cytoscape)
4. Entity explorer sidebar
5. Hover info showing graph edges and fields
6. Copilot Chat @specforge interaction
7. Full workspace layout

## Implementation Phases

### Phase 1: Core Language Support (2-3 weeks)
- Scaffold extension project in `integrations/vscode/`
- Language registration, TextMate grammar
- LSP client with binary resolution
- Semantic token mapping, all LSP features wired
- Snippets (22 kinds), JSON schema validation
- Status bar, commands, settings
- Basic test suite, marketplace listing

### Phase 2: Visualization + Explorer (2 weeks)
- Entity Explorer tree view with Activity Bar icon
- Interactive graph webview (Cytoscape.js)
- Graph layout switcher and filtering
- CodeLens provider
- Language status item, walkthrough

### Phase 3: AI Integration + Polish (2 weeks)
- MCP server auto-start
- Claude Code MCP config generation
- Copilot Chat participant + LM Tool registration
- Graph accessibility, custom themes
- Platform-specific binary packaging, CI pipeline

### Phase 4: Post-launch (ongoing)
- Graph "time machine" (evolution over git history)
- "Spec from clipboard" smart paste
- AI-powered quick fixes
- Automatic context enrichment
- Extension ecosystem support

## NPM Dependencies

```json
{
  "dependencies": {
    "vscode-languageclient": "^9.0.1"
  },
  "devDependencies": {
    "@types/vscode": "^1.95.0",
    "@vscode/test-electron": "^2.4.0",
    "@vscode/vsce": "^3.2.0",
    "esbuild": "^0.24.0",
    "mocha": "^10.8.0",
    "typescript": "^5.7.0",
    "cytoscape": "^3.30.0",
    "cytoscape-cose-bilkent": "^4.1.0",
    "cytoscape-dagre": "^2.5.0",
    "tippy.js": "^6.3.7"
  }
}
```

Note: Cytoscape and plugins are bundled into the webview JavaScript (not extension host), so they go in `devDependencies` and are bundled via esbuild into `media/graph-panel/graph.js`.

## Expert Panel Highlights

The design was produced by a 20-expert panel covering: VS Code Extension Architecture, LSP Integration, Webview/UI, Tree View, DX, Graph Visualization, AI/Agent Integration, Performance, Accessibility, Syntax Highlighting, Code Actions, Testing/Quality, Marketplace, Command Palette, Status Bar/Notifications, CodeLens, Snippets/Templates, Configuration, Onboarding, and Cross-Extension Integration.

Key "wow factor" features identified by experts:
- **Auto-download platform-specific binaries** (like rust-analyzer)
- **Dual-server architecture** (LSP + MCP side-by-side)
- **"Focus mode"** graph navigation (right-click -> Show in Graph with 2-hop neighborhood)
- **Inline search in entity tree** with fuzzy matching
- **"30-second demo mode"** for marketplace presentation
- **Graph "time machine"** showing evolution over git history
- **"Spec-Driven Code Review"** comparing PR diffs against entity graph
- **Coverage sparkline CodeLens** with Unicode block characters
- **Automatic AI context enrichment** for any AI tool in the workspace
