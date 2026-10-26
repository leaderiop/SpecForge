# The editor client asks the language server for every entity fact

**Status:** accepted (2026-10-26, architecture round 28, plan 05). Reopens [ADR 0249](0249-the-editor-client-is-an-adapter-over-the-language-server.md) D1 and its "runs `specforge` for the graph views" consequence. Follows [ADR 0023](0023-the-lsp-reads-a-document-through-one-module.md) D5.

## Context

- Round 27 made the VS Code client an adapter for hover, completion, lenses and symbol search; three entity facts stayed the client's own.
- Four commands (Inspect, Trace, Copy Entity JSON, Focus in Graph) named "the entity here" with the manifest's word pattern: a word inside a contract string named whatever entity shared its spelling, where the server's `Cursor::target` names nothing.
- The tree and the graph read the project on disk through `specforge export` and `specforge query` in a second process and refreshed only on a VS Code save: an unsaved entity was missing, a change made by `specforge edit`, `git checkout` or an agent never showed.
- Four hand-kept kind tables (the tree's icons and order, the graph's colours, the hover's icons) disagreed with what the extensions declare: 15 of 22 kinds in the tree, 21 of 22 in the hover; a third-party kind got nothing.
- The graph view counted its own degree from the raw edges, so an entity naming only itself read "2 connections" where every host view calls it unconnected ([ADR 0270](0270-an-edge-from-an-entity-to-itself-counts-in-no-edge-count.md)).
- A kind declaring no `lsp_icon` answered `Variable`; its spec says `Object`.

```text
BEFORE  cursor ─wordPattern─▶ word ─specforge query─▶ node        tree, graph ─specforge export (disk)─▶ on save
AFTER   cursor ─specforge/entity─▶ Cursor::target ─▶ node         tree, graph ─specforge/graph (session)─▶ on specforge/graphChanged
```

## Decision

- **D1. Two requests.** `specforge/graph` (no params) answers `{graph, kinds, degrees}`: the Graph Protocol document `export --format=graph --no-schema` writes over the session (open buffers included; while a rebuild runs, the last complete compile), each kind's `SymbolKind` (LSP numbering), and each node's `Graph::degree(id).total()` (a self-edge counts for nothing, so no client counts edges). `specforge/entity`, at a text document position, answers the node `query <id> --depth 0` writes for the entity `Cursor::target` names there, `null` where it names none. `backend::service()` builds the service with both; `main` and the tests call it. A client that always sends params to `specforge/graph` gets `-32602`.
- **D2. A notification.** After each applied change the server sends `specforge/graphChanged` (no params) to a client that declared `experimental.specforgeGraphChanged`, as it asks a `refreshSupport` client to refresh lenses and tokens; no other client is sent a method it does not know.
- **D3. The client names an entity only by `specforge/entity`.** [ADR 0023](0023-the-lsp-reads-a-document-through-one-module.md) D5 is the rule; the `wordPattern` of `language-configuration.json` serves word motions only. Inspect and Trace with no entity under the cursor offer a pick list of the session's ids; Trace keeps the CLI's report, on the server's id.
- **D4. A kind's icon, colour and order are the client's rendering of its `SymbolKind`.** One table (`symbols.ts`) maps a `SymbolKind` to VS Code's codicon and theme colour; groups sort by kind name. A kind declaring no `lsp_icon`, or one the server does not map, is `Object` (the spec's rule, now the code's).
- **D5. The pickers' builtins are pinned.** Initialize Project and Add Extension keep their list of the four builtins that declare kinds (Initialize Project runs before a project exists); `docs_truth` compares it to `builtin-vocabulary.json`.

## Rejected

- `workspace/symbol ""` for the tree: no title, fields or edges, and it leaves out the entities of a buffer typed since the compile, by design.
- `workspace/executeCommand` with server commands: `vscode-languageclient` registers each as a palette command, arguments are untyped, `initialize` gains a capability, and the refresh still needs a notification.
- A client refresh timer: it races the server's debounce and misses every change made outside the editor.
- Extension-declared codicons and colours: an editor-specific word in the extension protocol ([ADR 0015](0015-read-views-are-operations-over-the-project-view.md) I4), every builtin re-declared and re-vendored.
- Keeping the CLI for the tree and the graph: the lag and the missed changes stay.

## Consequences

- Breaking (pre-release, no shim): a kind with no or an unknown `lsp_icon` is `Object` (19), not `Variable` (13), in document and workspace symbols; the four commands act on no word in a string, a comment, a keyword or a field name; the tree and the graph show the buffers and refresh on every rebuild; the graph's "N connections" and its ranking count no self-edge; icons and colours change for 15 kinds in the tree and 21 in the hover; `Editor` (specforge-lsp) gains `graph_changed`.
- The client runs `specforge` only for project-level commands (check, export, stats, doctor, model, outline, analyze, explain, trace, init, add, remove).
- Nothing here has run in a VS Code host (as [ADR 0249](0249-the-editor-client-is-an-adapter-over-the-language-server.md) records); the `docs_truth` scans, `tsc --noEmit` and the language server's tests of both requests prove it.

## What would reopen it

- A second editor client that needs these requests (then the method names move to the spec's port).
- The graph answer's size becoming a latency problem (a paged or diffed answer).
