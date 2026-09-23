# 006 — Anders Hejlsberg

**Cluster:** C2 — DSL & language design
**Roster role:** Turbo Pascal/C#/TypeScript creator; LSP-first language design
**SpecForge anchors:** typed .spec fields (`crates/specforge-registry/src/registries/field.rs` FieldRegistry); LSP-first design (`crates/specforge-lsp/src/backend.rs`, `docs/lsp-server-capabilities-catalog.md`)

## Why this engineer
Hejlsberg has shipped three generations of languages where tool experience drove adoption — Turbo Pascal's single-pass speed, C#'s first-class tooling, TypeScript's graduated typing — and is now leading the Go-native TypeScript rewrite. SpecForge's core bet is his TypeScript playbook: a small typed vocabulary whose types are portable intelligence, served to editor and compiler alike through an LSP from day one. His rewrite campaign is also the precedent for re-implementing SpecForge's Rust internals while keeping the .spec surface stable.

## References for SpecForge
**Key works**
- [A 10x Faster TypeScript](https://devblogs.microsoft.com/typescript/a-10x-faster-typescript/) — Microsoft DevBlog, 2025. Rewriting a compiler core under a frozen language contract — the model for evolving specforge crates without breaking .spec users.
- **Static Typing Where Possible, Dynamic Typing When Needed: The Case for TypeScript** — OOPSLA 2014. Graduated typing rationale — mirrors .spec's "read like docs, compile like code" duality.
- **The C# Programming Language** (with Torgersen, Wiltamuth, Golde) — Addison-Wesley, 4th ed, 2010. Annotated language-spec form: grammar, semantics, and commentary in one document — a pattern for documenting .spec authoritatively.
- [microsoft/TypeScript](https://github.com/microsoft/TypeScript) — GitHub, 2012. Reference for language-service layering: a compiler API consumed by an LSP server — exactly specforge-lsp over the compiler crates.
- **TypeScript, C# and Turbo Pascal with Anders Hejlsberg** — Pragmatic Engineer Podcast, 2026. Four decades of language-evolution tradeoffs, in his own words.

## Study first
1. A 10x Faster TypeScript — rewrite-under-contract strategy
2. TypeScript language-service architecture (tsserver/LSP layering)
3. OOPSLA 2014 "case for TypeScript" paper
