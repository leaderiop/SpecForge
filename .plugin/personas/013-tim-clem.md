# Position — Tim Clem (parser & search infrastructure; tree-sitter at GitHub scale)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. **Wasm is the only option with a real binary-artifact story, and SpecForge already bet on it twice.** RES-30 (`spec/research/RES-30-tree-sitter-wasm-feasibility.md`) chose host-provided parsing because grammar `.wasm` files (SpecForge grammar: 15 KB; Rust: 819 KB) are small, compiled-once, registry-verifiable artifacts — Zed's model. Interpreter runtimes distribute source text and interpret it: R-4 verifiability and R-6 reproducibility weaken structurally, and Architecture-B's grammar-distribution plan — which presupposes Extism guests — is orphaned.

2. **The workload is 97% manifest, 3% logic** (evidence.md: 629 guest lib.rs lines, `@specforge/formal` 478). That doesn't justify a language runtime; it justifies fixing execution gaps — C7-02 (AOT cache is a byte-copy), C7-08 (EnginePool is a ledger), C7-10 (`max_execution_ms` unenforced) are wasm-model bugs, not wasm-model failures.

3. **Determinism under R-6 is structural, not bolted on.** `wasm32-unknown-unknown` has no ambient clock, RNG, or I/O — snapshot-testable analyze output falls out of the sandbox. Lua/Python/JS must amputate their stdlibs per-capability, and each runtime reimplements that policy badly (C7-04 shows SpecForge already got allow-by-default wrong once).

## Biggest risk in my verdict
The registry bets on third-party authors; requiring a Rust toolchain + `wasm32-unknown-unknown` target raises the authoring bar. If adoption stalls on toolchain friction, KEEP_WASM preserves the wrong moat.

## What would change my mind
Evidence that logic-bearing plugins (formal-class passes) dominate the payload and Rust compile cycles throttle AI-agent authoring iteration — then a TypeScript tier behind the same protocol beats purity. Nothing less.
