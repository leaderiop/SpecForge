# Position — Aleksey Kladov (rust-analyzer creator; incremental pipelines, LSP & diagnostics UX)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. Trap containment is the IDE contract. `crates/specforge-wasm/src/trap.rs` turns any plugin failure into an E028 Diagnostic plus Failed state (`handle_wasm_trap`, `should_skip_extension`); `invariants.rs` pins "extension trap does not affect other extensions". That is resilient parsing applied to plugins: the watch/LSP session never dies, diagnostics keep flowing. CPython/PyO3 fails this at the process level — native pip extensions can abort the host.
2. R-1 is my one-pipeline principle. rust-analyzer works because IDE and batch compile share one core; here the disease is C7-11's three parallel implementations (`crates/specforge-emitter/src/builtins/*.rs` mirrors, embedded blobs, `NativeCustomRules` bypassing guest `validate__*`). That convergence is mandatory under every option; KEEP_WASM is where the machinery (runtime trait, lifecycle, integrity, toposort, 3,070 green tests) already exists.
3. Determinism and latency: `IncrementalPipeline` promises CLI-byte-for-byte diagnostics (`pipeline.rs:83`), and the watch loop debounces at 50ms (`watcher.rs:45`). Wasm without granted capabilities is deterministic (R-6) and hash-reproducible (R-4). The real loop risks — no warm instances (C7-08), fake AOT (C7-02), unenforced `max_execution_ms` (C7-10) — are host bugs fixable in place, not reasons to swap runtimes and re-earn that work.

## Biggest risk in my verdict
Authoring ergonomics: editing a plugin means a Rust toolchain plus `wasm32` compile, so watch-loop authoring latency is rustc-scale, not editor-scale — and with guests ~97% manifest, the third-party registry bet may never materialize.

## What would change my mind
A scripting tier that becomes the single mechanism — all four builtins rewritten, mirrors deleted, custom rules routed through it — with capability-scoped sandboxing and proven deterministic snapshots; Lua-only could then win. MULTI stays out: it recreates C7-11.
