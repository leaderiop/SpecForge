# 012 — Max Brunsfeld

**Cluster:** C3 — Parsing & grammar infrastructure
**Roster role:** created tree-sitter; incremental parsing for programming tools; Zed co-founder
**SpecForge anchors:** crates/tree-sitter-specforge (grammar.js → generated parser); error-tolerant recovery in crates/specforge-parser/src/parse.rs (push_error_node, `expected` hints)

## Why this engineer
Every `.spec` file SpecForge compiles is parsed by machinery Brunsfeld invented: crates/tree-sitter-specforge turns grammar.js into an incremental GLR-style parser, and parse.rs leans on its error recovery to turn broken sources into actionable diagnostics ("unclosed block — missing closing `}`") plus an `expected` hint instead of failing hard. Tree-sitter's core promises — a full concrete tree under any input, reparse after keystroke-sized edits, S-expression queries — are exactly what the LSP, watch daemon, and parse_incremental pipeline already depend on. His design notes are the operating manual for evolving the grammar without breaking the 219-file self-spec corpus.

## References for SpecForge
**Key works**
- [tree-sitter](https://github.com/tree-sitter/tree-sitter) — GitHub, tree-sitter/tree-sitter, 2018–present. Canonical implementation: grammar DSL, error recovery, incremental parsing — the engine under crates/tree-sitter-specforge.
- [Tree-sitter documentation](https://tree-sitter.github.io/tree-sitter/) — official docs, 2018–present. Using-parsers/creating-parsers guides incl. the query system this repo uses (queries/highlights.scm, tests/queries.rs).
- **Tree-sitter — a new parsing system for programming tools** — Strange Loop 2018 (talk). First-person design rationale: uniform C API, per-keystroke reparses, resilient trees.
- **Tree-sitter** — FOSDEM 2018 (talk). Early public walkthrough of grammar design and the GitHub.com/Atom integration.

## Study first
1. Error-recovery semantics: ERROR vs MISSING nodes — parse.rs maps ERROR to typed ParseError today; MISSING nodes are an unexploited diagnostic source
2. The query system as the sanctioned read path over the CST (SpecForge's queries/ directory and its tests)
3. Incremental contracts: what a correct InputEdit must contain for parse_incremental's tree reuse to stay sound
