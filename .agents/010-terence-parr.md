# 010 — Terence Parr

**Cluster:** C2 — DSL & language design
**Roster role:** ANTLR creator; parser patterns & error recovery
**SpecForge anchors:** `crates/tree-sitter-specforge/grammar.js`; parser error recovery (`crates/specforge-parser`, parse_incremental); grammar feasibility research (`spec/research/RES-30-tree-sitter-wasm-feasibility.md`)

## Why this engineer
Parr spent three decades on exactly the hard problems the .spec grammar faces: ambiguity resolution, incremental reparsing, and error recovery that degrades gracefully instead of cascading. Language Implementation Patterns is the plain-language catalog behind a block-structured grammar like SpecForge's, and ANTLR 4's error-listener/recovery design is the benchmark for how specforge-parser should survive a malformed block and still compile the rest of the graph. His ALL(*) work explains, at a theoretical level, why tree-sitter-style conflict resolution matters in grammar.js.

## References for SpecForge
**Key works**
- [Language Implementation Patterns](https://pragprog.com/titles/tpdsl/language-implementation-patterns/) — Pragmatic Bookshelf, 2009. Token streams, AST construction, and error recovery as named patterns — the mental model for grammar.js.
- [The Definitive ANTLR 4 Reference](https://pragprog.com/titles/tpantlr2/the-definitive-antlr-4-reference/) — Pragmatic Bookshelf, 2013. Error listeners, recovery strategies, and incremental-friendly design to mirror on tree-sitter parse failures.
- **Adaptive LL(*) Parsing: The Power of Dynamic Analysis** — with S. Harwell, K. Fisher; OOPSLA 2014. How adaptive parsing kills grammar ambiguity — background for diagnosing grammar.js conflicts.
- [antlr/antlr4](https://github.com/antlr/antlr4) — GitHub. Canonical implementation reference for recovery and grammar semantics.
- [antlr.org](https://www.antlr.org) — tutorials and Parr's USF course notes on parser patterns.

## Study first
1. Language Implementation Patterns error-recovery chapters
2. ANTLR 4 reference, error handling chapter
3. grammar.js precedence/conflict annotations — where recovery actually bites
