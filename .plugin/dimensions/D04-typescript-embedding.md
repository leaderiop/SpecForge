# Dimension D04 — TypeScript embedding assessment

Candidates: `deno_core` (V8), QuickJS (`rquickjs`/quickjs-ng), Boa, Node/Deno/Bun sidecar.
The workload any engine must carry is the one visible in `extensions/software/src/lib.rs`:
~97% declarative manifest (the nine `include_bytes!("describe_*.json")` blobs) and ~3% logic
(`@specforge/formal`'s 4 compiler passes, 478 lines of guest Rust), executed against JSON
graph snapshots of ~1.7k entities. Today that runs on wasmtime 43.0.2 behind ~9.4k LOC of
`crates/specforge-wasm` host machinery.

## Option-by-option

**deno_core (V8).** The most complete package: V8, a snapshot system, async ops, and the
polished deno permissions model. TS transpile is solved in-house via `deno_ast`/SWC type
stripping. Costs: V8 adds tens of MB per platform binary (R-3 tension; Deno ships ~100MB
binaries), build time depends on prebuilt rusty_v8 static libs downloaded at build time
(reproducibility friction for R-4 — vendoring is possible but heavy), `deno_core` APIs move
fast with frequent MSRV bumps, and compile times are long. Startup is fine with a snapshot
(~ms per isolate) but baseline isolate memory is MBs, not KBs. V8 runs JIT — a real
in-process attack surface (n-day exploits are common in the wild) that a capability-op
layer does not eliminate.

**QuickJS / quickjs-ng via `rquickjs`.** Bellard's interpreter, now best maintained as the
quickjs-ng fork; `rquickjs` is the mature Rust binding and is in production at AWS (LLRT).
~1–3MB binary addition, sub-millisecond context creation, KBs–low-MBs baseline memory,
**no JIT** (interpreter only — eliminates V8's largest exploit class), built-in
`JS_SetMemoryLimit` and an interrupt handler for wall-clock/CPU cutoffs. Two gaps: no TS
(you embed a transpiler — but see below, this is solved in Rust), and throughput ~10–50x
below V8 on compute-bound loops. For this workload the bottleneck is JSON marshalling and
graph walks, not numeric crunching; an interpreter easily covers per-entity rule checks
and 478-line passes.

**Boa.** Pure Rust, no C vendor step, pleasant embedding API — but conformance is still
incomplete and performance trails QuickJS. For a host whose plugins must run real
third-party JS, its spec-gap risk is disqualifying next to quickjs-ng at similar binary
cost. Research-grade, not yet infrastructure-grade.

**Node (or Deno/Bun) sidecar.** Cleanest rejection: violates R-3 outright (system Node per
user machine), adds process supervision, IPC marshalling of large graph payloads, and
version skew between host and sidecar. `deno compile` single-file sidecars are self-
contained but mean shipping a second ~100MB binary per platform. Rejected on R-3.

## Sandboxing vs R-2

Here is the honest core: **neither bare QuickJS nor a bare V8 isolate has any ambient
capability at all** — no fs, no net, no process — because there is no stdlib I/O in the
context. Capabilities exist only as host functions the host registers, exactly like Extism
host imports today. Deno's permissions model is a pre-audited, pre-named layer over that
pattern; with QuickJS you build a smaller one yourself, which `crates/specforge-wasm/src/
sandbox.rs` already is in spirit. The C7-04 lesson (sandbox policy `file_system_access`
allow-by-default) is a host config bug class that exists identically in either engine —
the fix is default-deny at the op/capability-registration layer, not engine choice.
Resource limits: QuickJS gives memory limits and interrupts natively; deno_core gives
isolate memory limits and `terminate_execution`. Both can enforce the `SandboxPolicy`
declared in `extensions/software/src/lib.rs:30-38` (256MB / 5000ms / no net / no fs) —
closing audit C7-10 (`max_execution_ms` never enforced), which is a host bug, not a wasm
deficit. Honest asymmetry: wasmtime's memory-isolation sandbox is formally stronger than
any in-process JS engine against a malicious plugin; a JS engine escape compromises the
host process. Mitigation: interpreter-only QuickJS materially shrinks that surface, and
R-4's signed-registry gating is the outer perimeter either way.

## TS toolchain for authors

A bare engine does not run TS, so the toolchain must be defined either way. Three
workable shapes: (1) embed type stripping in-process — `oxc` or `swc_core` are Rust
crates, deterministic, no build step for authors; (2) transpile at publish time in the
registry pipeline; (3) require authors to ship pre-stripped JS. Option 1 is best: source
distributed and verified (R-4 checks the .ts source), stripped deterministically at load —
microseconds per file. Crucially, the plugin API surface becomes a `.d.ts` shipped with
the host — that **is** the IDL audit C7-03 says is missing. Today the contract is nine
loose JSON categories and a stringly `call_export`; under TS it is typed functions the
editor and AI agent can validate before the host ever loads the plugin.

## QuickJS vs V8 for a CLI host — straight answer

| | quickjs-ng (rquickjs) | deno_core (V8) |
| --- | --- | --- |
| Binary delta | ~1–3MB | tens of MB/platform |
| Cold start | sub-ms context | ms (snapshot) + platform init |
| Baseline memory | KBs–low MBs | MBs per isolate |
| Compute throughput | 10–50x slower than V8 | JIT-fast |
| Attack surface | interpreter, no JIT | JIT + 15 years of n-days |
| Maintenance | quickjs-ng + rquickjs active (AWS LLRT) | Deno Land, fast-moving API |
| TS | needs embedded transpiler | first-party via deno_ast |

For a CLI run repeatedly per command, QuickJS wins R-3 decisively. V8's throughput edge
only matters if plugins do heavy numeric work — nothing in the four builtins does; the
formal passes walk a JSON snapshot. AI agents authoring TS don't care which engine
executes it. Net: **QuickJS is the honest pick for a single-binary CLI host; deno_core is
the pick if you expect V8-grade workloads or Deno-grade module/npm semantics.**

## Consequences under the brief

- **R-1/C7-11:** builtins become plain TS source — the vendored blobs (C7-00), the
  byte-copy "AOT" cache (C7-02), the ledger EnginePool (C7-08), the native mirrors, and
  the blob-sync guard tests all delete. One mechanism: engine + typed sources.
- **R-5:** hot reload is trivially re-creating a context from edited source — no compile
  step, which is *better* than the wasm path's broken AOT story.
- **R-6:** `JSON.parse`, key insertion order, and shortest-round-trip float formatting are
  deterministic; host must pin `Date`/`Math.random` for snapshot tests (same discipline
  wasm needs for WASI clocks).
- **Ecosystem alignment:** the collect story already targets TS-native tooling — the
  planned `@specforge/vitest` reporter adapter (per the Fu/C11 analysis) is a TS module
  by construction, and TS plugins put plugin authors in the same language as the
  reporters, specs, and MCP consumers they interoperate with.
- **Cost:** rewriting `@specforge/formal`'s 4 passes in TS, a new TS SDK + `.d.ts`,
  registry blob-type change (signing/integrity machinery is format-agnostic), and
  ~9.4k LOC of wasm host replaced by a much smaller engine wrapper. The npm-ecosystem
  temptation must be resisted initially — bundling at publish with a lockfile is a
  later, deliberate addition, not a day-one promise.

## Verdict

**Verdict:** TYPESCRIPT
**Confidence:** 3
**One-line rationale:** QuickJS (rquickjs/quickjs-ng) + in-process TS type-stripping gives single-binary R-3, capability-op sandboxing equal to Extism's model, trivial hot reload, and a typed plugin API that fixes the missing-IDL audit gap — at the cost of rewriting the formal passes and accepting a weaker-than-wasmtime isolation boundary.
