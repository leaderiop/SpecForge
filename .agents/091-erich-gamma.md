# 091 — Erich Gamma

**Cluster:** C11 — Testing & BDD (verify statements, collect, report protocol)
**Roster role:** JUnit co-creator; Eclipse JDT and VS Code lead
**SpecForge anchors:** integrations/vscode extension architecture (extension.ts, lsp-client.ts, graph-webview.ts, entity-tree.ts, codelens.ts, commands.ts)

## Why this engineer
Gamma co-created JUnit with Kent Beck, led Eclipse JDT — the first IDE built on a language service rather than ad-hoc parsing — and then built VS Code as a thin shell delegating all semantics to a language server. integrations/vscode follows that exact playbook: a small extension surface that forwards work to specforge-lsp (tower-lsp) and renders read-only views like the entity tree and graph webview. Nobody has more scar tissue about where logic belongs in an extension: in the server, not the client.

## References for SpecForge
**Key works**
- **Design Patterns: Elements of Reusable Object-Oriented Software** — Addison-Wesley, 1994. The GoF vocabulary; the extension's commands/providers/views are organized around these roles.
- **JUnit: A Cook's Tour** (with Kent Beck) — Java Report, 1999. Designing a framework by applying its own patterns; the runner/reporter split report adapters inherit.
- [microsoft/vscode](https://github.com/microsoft/vscode) — GitHub. Reference architecture for extension host + language-server separation that integrations/vscode mirrors toward specforge-lsp.
- **Inside VS Code: How we build and ship it** — Microsoft keynote. Practical lessons on shipping a thin-shell IDE product on top of a server backend.
- **Project Ticino interview** — The Register, 2021. Why VS Code pivoted from a failed online editor; the architecture lesson that kept the extension layer dumb.

## Study first
1. VS Code extension architecture: extension host, LSP client, webview isolation
2. JUnit: A Cook's Tour — patterns applied to test-framework design
3. Eclipse JDT history — where "model in the server, UI in the client" came from
