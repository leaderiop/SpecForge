# Position — Peter Chen (ER model, 1976; graph engine & algorithms)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. **The boundary is a data model, and it currently has none.** C7-03 — stringly-typed JSON over `call_export`, no IDL — is what my 1976 thesis says to fix first: define the schema of what crosses the boundary (entity-graph snapshot, spans, diagnostics — the `ModelIntermediate` shape of `crates/specforge-emitter/src/model/mod.rs`), then treat runtimes as renderers. Notation is a view; the model is the contract. Swapping wasm for Lua without a typed contract reproduces the disease.
2. **MULTI repeats the C7-11 mistake.** One schema → many renderers (mermaid/dbml/dot in `model/`) works because renderers share one IR; multiple runtimes without one IR is the three-parallel-implementations failure, doubled.
3. **The workload is ER-shaped, not language-shaped.** Evidence.md §2: the guest payload is ~97% generated manifest — entity kinds, edge types, fields are entity sets and relationships, i.e. data, mechanically checkable as `model/cardinality.rs` proves. The ~3% real logic (formal's 4 compiler passes) is what needs wasm's memory isolation (R-2), determinism (R-6), and static embedding (R-3).
## Biggest risk in my verdict
The graph snapshot stays untyped and slow crossing wasm linear memory, and C7-10 (`max_execution_ms` never enforced) lets a buggy pass hang analyze; wasm wins only if the IDL, execution limits, and pooling (C7-08) actually get fixed.
## What would change my mind
Evidence that a typed IDL plus an instruction-counted interpreter (Lua/Luau) carries formal's ~478 lines of pass logic within R-2/R-6 cheaper — if the logic surface stays this small, isolation-by-interpreter may beat isolation-by-VM.
