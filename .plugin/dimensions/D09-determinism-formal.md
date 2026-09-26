# D09 — Determinism & Formal Analysis (R-6)

**Persona:** Leonardo de Moura — determinism as a theorem, not an aspiration.
**Scope:** R-6 — "deterministic, snapshot-testable plugin output for the analyze pipeline."

## What R-6 requires, precisely

Analyze output must be a function of exactly two things: the spec files and the pinned extension versions (R-4 gives the pinning). Same inputs → byte-identical report, snapshot-able. Anything else — wall-clock, host filesystem, hash seeds, GC timing, locale — is a leak.

## What the host already gets right

The host treats ordering as a contract:

- `Graph` stores nodes in a `HashMap<Sym, Node>` but `nodes()` **sorts by id** (`crates/specforge-graph/src/graph.rs:140-143`); emitters sort edges by `(source, target, label)` (`crates/specforge-emitter/src/json.rs:89-101`).
- `detect_cycles` collects cycle members into a `HashSet`, then **sorts before emitting** (`crates/specforge-emitter/src/compile.rs:734-735`) — the HashSet-seeded DFS start order is deliberately laundered out.
- `toposort` breaks dependency ties by extension name and has a test asserting 100 runs produce the identical order (`crates/specforge-wasm/src/toposort.rs:6,111-123`); grammar composition has an explicit "ensures: result is sorted (deterministic)" test (`contributions.rs:1582-1586`).
- `order_passes` topologically orders declared passes with declaration-order fallback (`crates/specforge-cli/src/analyze.rs:34-36`).
- Snapshot culture exists: insta in `specforge-parser/tests/snapshot_tests.rs` and `specforge-emitter/tests/model.rs`; byte-sync guards (`extension_json_sync`, `builtin_blob_sync`).

## The live nondeterminism is in the plugin, not the runtime

`@specforge/formal`'s guest (`extensions/formal/src/lib.rs`) iterates Rust `HashMap`s when emitting findings, and Rust's default `RandomState` is seeded from environment entropy — the wasm spec does not fix it, so per-instance iteration order may vary run to run under wasmtime:

- `pass_layering_verify`: DFS roots iterate `refines.keys()` (line 198) and W031 findings iterate `&depth` (line 235). Worse than ordering: the E041 message `cycle through '{id}'` reports whichever node the DFS *first* re-encounters — **finding content varies with hash seed** when multiple cycles exist.
- `pass_event_graph_analyze`: W029 findings iterate `&produced` (line 272). Set is deterministic; sequence is not.

The host then ingests findings **as returned** — no canonicalization at the boundary (`crates/specforge-cli/src/analyze.rs:183-187`), and both the JSON report (`analyze.rs:343-346`) and text renderer preserve guest order. Conclusion: R-6 is violated today by the one logic-bearing guest, and the host's contract discipline stops exactly at the wasm boundary.

Two audit items are also determinism items. C7-04 (sandbox `file_system_access` allow-by-default) means a plugin *can* read ambient machine state — output is then not a function of inputs, a purity hole, not merely a security one. C7-10 (`max_execution_ms` unenforced): when a bound is added it must be **fuel**, not wall-clock — fuel is a machine-independent step budget; a timeout kills divergent passes non-deterministically across hardware, truncating findings differently per machine.

## Interpreter runtimes: what each leaks

- **KEEP_WASM.** Rust guests on `wasm32-unknown-unknown` have linear memory — no GC pauses, nothing allocates under a collector. The current ABI imports nothing ambient: no clock, locale, or RNG crosses the boundary. Wasm core semantics are deterministic (fixed IEEE-754; no relaxed-SIMD in these binaries; the passes use only integer arithmetic — `MAX_LAYERING_DEPTH` counting is `usize`). Per-call fresh instances (C7-08 aside) mean no cross-run guest state. The only leak class is the one found above: guest-side hash seeding — which is a *host-language* property, not a wasm property, and would survive any runtime swap.
- **PYTHON.** Worst in class. Ambient capabilities everywhere (fs/net/process are one `import` away). GC pauses are real and interact with any enforcement timeout. Set iteration order is hash-seed dependent per process (`PYTHONHASHSEED`) — sets are idiomatic for membership tests, so the exact bug the formal guest has today would be *idiomatic Python*. Wall-clock, locale-dependent formatting, and PyO3 embedding fragility (GIL, subinterpreter limits) round it out. Determinism would be enforced by convention only, and conventions are what AI-authored plugins will violate.
- **LUA.** `pairs()` order is unspecified but stable for string keys in a fixed build (Lua 5.4 unseeded string hashes; LuaJIT deterministic). Incremental GC. Floats (LuaJIT doubles) appear in numeric output formatting. Lower ambient hazard than Python, but again convention-enforced.
- **TYPESCRIPT.** V8's own-property and `Map`/`Set` iteration is spec-mandated insertion order (integer-like keys sorted) — deterministically ordered *by construction*, the best of the scripting tier on this axis. But GC pauses are real, and `Math.random`/`Date` are ambient imports a permission system must deny; QuickJS is deterministic and small if chosen instead.

The shared lesson: hash-container iteration for output ordering is a bug factory in every candidate. The differentiator is not which interpreter hides it best — it is which runtime lets you *mechanize* the guarantee.

## Where verdicts can be formally checked

The pass ABI is already shaped like a pure decision procedure: `PassInput` snapshot in, `Vec<PassDiagnostic>` out (`crates/specforge-extension-sdk/src/lib.rs:695-703`), a small closed ADT over JSON. That is exactly the form a proof assistant consumes. Concretely:

1. **Reference semantics.** All four formal passes are structural recursion + DFS + a depth bound over a list-of-records ADT — a few hundred lines, mechanizable in Lean with the output order quotient away (findings as a set). `condition_check` is first-order; `layering_verify` is cycle-freedom plus a depth bound over the refinement subgraph — expressible directly as SMT/inductive goals, the on-ramp to `analyze --prove` discharging condition entities with Z3.
2. **Property harness (cheap, now).** Run each pass on a fixed `PassInput`, then on permutations of the entity/edge order; canonicalize (sort findings by `(code, id)`); assert equality. This catches the E041-content bug class in CI today, independent of runtime. It is the missing sibling of the existing `extension_json_sync` guard.
3. **Ingest canonicalization.** Sort findings at the host boundary (`analyze.rs:183-187`) and snapshot the end-to-end analyze JSON. Host-side canonicalization makes finding *order* a non-issue even for third-party guests; the E041 *content* bug needs the guest fixed (`BTreeMap`, as the host's `detect_cycles` effectively does).

## Is a capability-restricted subset provably deterministic?

Yes — and wasm is the only candidate where the claim is machine-checkable. Impose four restrictions: (a) **import allowlist** = deterministic host functions only (graph snapshot reads; deny fs/network/time/random — closing C7-04 fixes purity too); (b) **deterministic-profile wasm** (reject threads/atomics-wait and relaxed-SIMD); (c) **fuel metering** as the termination bound — total, machine-independent; (d) **host canonicalization** of output order. Under these, the determinism theorem is: *output is a pure function of (module bytes, input bytes, fuel)* — and R-4's registry sha256 pins the module bytes, so a report is reproducible bit-for-bit from the lock file. For CPython or QuickJS the analogous statement requires trusting interpreter semantics across versions; for wasm it is a property of a formally specified machine. That is the difference between a lint rule and a theorem, and it is the strongest formal-analysis argument in KEEP_WASM's favor.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** R-6's failures are contract bugs (guest HashMap iteration, no host canonicalization, fs-allow purity hole), fixable within wasm, which is the only candidate where determinism is mechanically enforceable — pure function of pinned module bytes + input + fuel — rather than a convention enforced on every plugin author.
