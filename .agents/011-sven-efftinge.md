# 011 — Sven Efftinge

**Cluster:** C2 — DSL & language design
**Roster role:** Xtext creator; Langium lead — textual DSL + IDE integration
**SpecForge anchors:** VS Code extension (`integrations/vscode`: walkthroughs, snippets, tmLanguage, webviews); LSP server (`crates/specforge-lsp/src/backend.rs`); authoring flow (`docs/spec-writing-flow.md`)

## Why this engineer
Efftinge built Xtext — the framework that made full textual-DSL tooling (grammar → parser → validator → IDE services) a configuration job instead of a research project — and now leads Langium, its LSP-native, TypeScript successor. SpecForge's surface is that architecture re-imagined for an agent-first audience: a textual .spec language with a language server, a VS Code extension with walkthroughs and snippets, and validation layered over the typed graph. Langium's server-centric decomposition is the closest open blueprint for how diagnostics, hovers, and linking should sit on top of specforge-graph.

## References for SpecForge
**Key works**
- [langium/langium](https://github.com/langium/langium) — GitHub, 2021. Canonical Langium: grammar, validation, and linker services composed around LSP — the modern blueprint for integrations/vscode + specforge-lsp.
- [Langium](https://langium.org) — official docs on the grammar DSL and LSP service layering.
- [Eclipse Xtext](https://www.eclipse.org/Xtext/) — Eclipse, 2008. Fifteen years of grammar-driven IDE tooling; its linking/scoping vs validation separation mirrors resolver/validator split.
- [eclipse/xtext](https://github.com/eclipse/xtext) — GitHub. Reference implementation from grammar to IDE services.
- **DSL Development with Xtext** — Sven Efftinge, talk (Vimeo), 2015. His own end-to-end walkthrough of DSL-to-IDE workflow.

## Study first
1. Langium validation + LSP service layering — diagnostics architecture
2. Xtext linking/scoping vs specforge-resolver link_references
3. integrations/vscode walkthroughs — onboarding as part of the language
