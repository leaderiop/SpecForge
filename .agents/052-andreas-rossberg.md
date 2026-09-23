# 052 — Andreas Rossberg

**Cluster:** C7 — Wasm plugin runtimes & the extension bet
**Roster role:** Wasm formal semantics; spec editor (SpecTec)
**SpecForge anchors:** spec/research/RES-21-specforge-format-design.md (format design), crates/specforge-wasm/src/protocol/types.rs (informal protocol types), tree-sitter-specforge/grammar.js (informal grammar)

## Why this engineer
Rossberg wrote the Wasm specification and its executable OCaml reference interpreter — a formal semantics that runs, so spec and implementations cannot silently drift — and led SpecTec, a spec-authoring toolchain adopted by the Wasm CG in 2025 to generate reference interpreters and tests from one source. SpecForge's .spec grammar (grammar.js) and its extension protocol (protocol/types.rs) are specified only informally; RES-21's format decisions are exactly the kind of thing his methodology would pin down: one executable definition from which parser, emitter, and protocol tests are derived.

## References for SpecForge
**Key works**
- Bringing the Web Up to Speed with WebAssembly — PLDI 2017. The founding paper: first mainstream language designed with formal semantics from day one; the argument for formalizing the protocol too.
- [WebAssembly specification documents](https://webassembly.github.io/spec/) — Wasm CG, 2017+. Normative text plus executable reference interpreter — the spec+oracle pattern to imitate.
- SpecTec has been adopted — webassembly.org, 2025. His report on the CG adopting SpecTec: spec language → generated interpreter, tests, and documents.
- Who's Afraid of the Turnstile? — BOB Konf talk. How they closed the gap between the formal spec and implementable pseudocode — demystifying formal semantics for practitioners.

## Study first
1. The reference interpreter as differential oracle for SpecForge's parser/emitter
2. SpecTec's spec-as-toolchain flow — a candidate shape for formalizing the handshake protocol
3. Applying executable-semantics discipline to RES-21's format-design decisions
