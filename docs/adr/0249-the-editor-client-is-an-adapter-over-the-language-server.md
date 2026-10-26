# The editor client is an adapter over the language server and the specforge binary

**Status:** accepted (2026-10-26, architecture round 27, plan 07). Amends the code lens consequence of [ADR 0016](0016-navigation-is-one-module.md); keeps [ADR 0010](0010-operations-reach-the-registry-through-a-port.md) (the language server is its own binary). Reopened by [ADR 0268](0268-the-editor-client-asks-the-language-server-for-every-entity-fact.md): the entity under the cursor, the entity tree and the graph are the language server's answers; the client runs `specforge` only for project-level commands.

## Context

- The VS Code extension (`integrations/vscode`) was the one adapter nothing ran. It re-read the `.spec` grammar for its code lenses (two regular expressions, a list of 20 keywords, a brace counter), typed 21 argument lists by hand (two were wrong since the first commit: `init --extension` exits 2 and `search` is the registry search), picked its two binaries by two rules (the server was never looked up on PATH; both could run from `target/` of the workspace or ten ancestor directories), and marked the server's hover markdown trusted for every command.
- The extension is unpublished, so its settings, views and commands break no installed user.

```text
BEFORE  client: regex lenses, 2 resolvers, 3 execFile copies, hover isTrusted = true
AFTER   client: shows what specforge-lsp answers; binary.ts resolves and spawns both binaries once
```

## Decisions

- **D1.** The client reads no grammar. Every fact about a document's text is the language server's answer.
- **D2.** The server answers `textDocument/codeLens` (references and verify statements, from the outline, the graph and the navigator's tokens) and asks a client that declared `workspace.codeLens.refreshSupport` to refresh after each rebuild. `specforge.findReferences` is the one client command a server answer names; a client without it shows the titles.
- **D3.** Both binaries come from the machine-scoped setting `specforge.binDir`, else by name on PATH. Nothing inside or above the workspace is run. The release archive and the Homebrew formula carry `specforge-lsp` beside `specforge`. A `specforge lsp` subcommand stays rejected (ADR 0010): the editor process would link the registry client.
- **D4.** Every argv the client builds is a literal list the scan in `docs_truth.rs` checks against the CLI's help; the called commands are a pinned list, so a new call is added on purpose.
- **D5.** Hover markdown enables only `specforge.focusInGraph`; a spec title holding a `command:` link runs nothing else.
- **D6.** A terminal command is a `ProcessExecution` task: no shell, nothing quoted. Captured output has one runner and a 64 MiB buffer.
- **D7.** A declared setting, view or script is one the client provides; the same scan checks the manifest.

## Rejected

- The client hangs lenses on document symbols and drops the verify count: no request answers it.
- No code lens: the counts would need a hover.
- `target/` inside the workspace folder before PATH: a trusted repository would still choose the binary; a developer sets `specforge.binDir` to `<repo>/target/debug`.
- A node CI job with a `vscode` stub: a second toolchain for glue the Rust-side scan pins.

## Consequences

- Breaking (VS Code): `specforge.lsp.path` is `specforge.binDir`; `specforge.codeLens.enabled` (use `"[specforge]": {"editor.codeLens": false}`), `specforge.format.onSave`, `specforge.mcp.autoStart`, `specforge.entityExplorer.groupBy`, the Extensions view, the Show in Graph lens, the `cmd+shift+s` binding and `npm test` are gone.
- LSP: additive (`codeLensProvider`, `workspace/codeLens/refresh`); the `Editor` trait gains `refresh_lenses`.
- Release: the archive and the formula gain `specforge-lsp`.
- No new code, no re-vendor.
- Nothing here runs in a VS Code host; the client is checked by the scan, `tsc --noEmit` and the language server's own tests.

## What would reopen it

- The client needs a fact only the CLI's text output gives.
- A second editor client appears: its rules then move to the spec.
