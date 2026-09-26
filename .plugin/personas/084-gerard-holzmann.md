# Position — Gerard J. Holzmann (formal methods, model checking)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. Determinism is the substrate R-6 stands on. The one logic-bearing guest, `extensions/formal/src/lib.rs`, implements `pass_event_graph_analyze` as a pure map `&PassInput -> Vec<PassDiagnostic>` — deterministic by construction inside wasm: no clock, filesystem, or network except explicit host imports. CPython or V8 embeds arrive with ambient modules (`os`, `io`, `fetch`) that must be blacklisted back; blacklists leak.
2. Wasmtime can enforce what exists only on paper today: C7-10 (`max_execution_ms` never enforced) is closable with fuel/epoch interruption — a CPU budget that itself fires deterministically. Python has no reliable preemption of runaway compute; standard Lua debug hooks are coarse. Under R-2, unbounded untrusted compute is a soundness hole, not a performance nit.
3. R-1 forecloses the seductive escape hatch: the formal guest's real logic (event graph, ordering checks, `sync.timeout` cycle unblocking — Promela-shaped by design) would need rewriting per runtime. Converging C7-11's three parallel implementations into one wasm mechanism is strictly less work and keeps R-4's byte-verifiable signed-blob distribution intact.

## Biggest risk in my verdict

Authoring ergonomics: demanding a Rust toolchain plus `wasm32-unknown-unknown` for AI-agent plugin authors is real friction, and C7-03/04/08 show the host side is partly vaporware — KEEP_WASM pays only if those gaps close.

## What would change my mind

A scripting runtime demonstrating all of: capability-gated by default, deterministic instruction budget with hard preemption, byte-reproducible artifacts through the signed registry — MULTI behind a determinism gate, never a second trusted tier.
