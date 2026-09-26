# D05 — Security & Capability Sandboxing

## The question, sharpened

R-2 is a capability-confinement requirement: untrusted plugins get no ambient filesystem, network, or process access, and any access is granted as a scoped, revocable token. Heiser's seL4 work gives the precise test a runtime must pass: a capability system requires (a) unforgeable references to specific objects and (b) **confinement** — no ambient authority the subject can exercise without a grant. The question is which candidate architecture can pass that test, not which one can be *configured* toward it.

## By construction vs. by convention

The wasm path meets R-2 **by construction**. A `wasm32-unknown-unknown` guest has no instruction that touches the outside world; every effect is a host import the embedder chose to link. Deny-by-default is the platform's compile-time property, not a setting. Today that surface is exactly three functions (`host_emit_diagnostic`, `host_read_file`, `host_query_graph` — `crates/specforge-extism/src/host_context.rs:56-62`), plus linear-memory isolation and fault isolation: a malicious or buggy guest returns `WasmCallResult::Trap` and becomes a diagnostic, while the host process survives. This is Varda's model from Cloudflare Workers: guest code is inert until handed capabilities; the security argument lives at the import boundary, which is small, auditable, and typed.

An embedded interpreter meets R-2 only **by convention**. mlua, PyO3/CPython, and QuickJS ship *with* the ambient surface: Lua's `io`, `os`, `require`, `debug`; CPython's `open`, `os.system`, `ctypes`, `importlib` (evidence.md: "pip ecosystem is ambient-capability"). Sandbox here means *subtracting* — stripping modules, overriding globals, policing FFI entry points — and every subtraction is a runtime configuration over a library the guest shares an address space with. Confinement fails by class: any memory-safety bug in the interpreter or in one missed override is a silent escape from the same process as the host's keys (tokens in the OS keyring, per the credential store). Wasmtime's TCB is a memory-safe Rust verifier+compiler; the interpreter TCB is a C codebase plus your hand-rolled module stripper plus every CVE in the stdlib. `deno_core` is the honest exception — Deno's `--allow-read=/path` permission model is genuinely capability-shaped, and it works because a dedicated platform team maintains deny-by-default checks over every API. Adopting it means adopting that maintenance burden and the V8 binary weight that strains R-3. Embedded CPython fails outright: its own documentation disclaims untrusted-code safety, and a `while True` plugin with the GIL stalls the host. CPython-as-sidecar works only behind an OS sandbox (seatbelt/seccomp), which forfeits R-3's single-binary distribution and R-5's hot reload.

## C7-04: the audit's own demonstration

The sharpest evidence against interpreter sandboxing comes from the current code's failure mode. `default_sandbox_policy()` sets `file_system_access: Some(true)` with empty `allowed_paths` (`crates/specforge-wasm/src/sandbox.rs:22, 141-143`), so `is_path_allowed("/any/path")` returns true — and the project's own invariant test pins that as expected (`src/invariants.rs:21-22`). Worse, the policy ledger is decorative: `is_path_allowed`/`is_domain_allowed` have **zero production callers** (tests and re-exports only), and the one real gate, `host_read_file_check` (`crates/specforge-wasm/src/host_functions.rs:66-148`), is fed `default_sandbox_policy()` directly (`host_context.rs:140`) — the three-layer manifest/project merge never reaches the enforcement point. The gate itself is sound (call-site whitelist, `..` rejection, canonicalize-before-prefix for symlinks, spec_root confinement), so guests can in fact only read under the spec root through one audited import. But the audit found exactly what a capability model decays into when policy and mechanism live in different layers. The same decay repeats in miniature at `sandbox.rs:97-99`: the project-override layer *replaces* `allowed_output_extensions` wholesale, bypassing the E030 code-extension filter the manifest layer enforces.

The fix inside KEEP_WASM is mechanical: default `file_system_access` to `Some(false)`, thread the merged policy into `build_host_functions`, wire or delete the dead checkers, and drop the unnecessary `.with_wasi(true)` (`crates/specforge-extism/src/runtime.rs:89` — no preopens are configured, so WASI grants nothing except a surface). The fix generalizes into the dimension's verdict: an engine that lets you *delete* the ambient surface cannot be matched by interpreters that start with it.

## C7-10: limits as engine primitives

The ledger promises `max_execution_ms: Some(30_000)` (`sandbox.rs:8`); grep confirms no fuel, epoch, or timeout wiring anywhere in `specforge-wasm`/`specforge-extism` — the audit's "never enforced" is accurate, and `max_memory_mb` is equally unenforced (`Manifest::new([Wasm::data(...)])`, `runtime.rs:87-92`, ignores Extism's memory options). But the remedy is engine-native: wasmtime fuel metering or epoch interruption applies to untrusted code without its cooperation, and the Extism manifest takes memory caps directly. Compare the interpreters: Lua offers debug-hook instruction counting (cooperative, workable, adds latency); V8 has its own budget via `deno_core`; embedded CPython has nothing — you must add thread supervision and process kills, i.e., become a sidecar. Resource limits are configuration on wasm and architecture surgery on Python.

## C8-09: supply chain and the dependency-graph shadow

Hinds' Sigstore discipline — sign at build, verify at install, bind artifact to identity, publish verification evidence to a transparency log — has a structural precondition: **the verified artifact must be the executed artifact**. The wasm path satisfies it: the registry blob is the code, sha256-checked at download, signature + key_id stored, keys pinned client-side (evidence.md §1.4); C8-09's fix made this real. An interpreter registry distributes *source*, and the trust boundary then jumps to load time: a Python plugin's `import requests` pulls transitively ambient-capable code into the host process that no signature covered. Signing one plugin while its dependency graph resolves unsigned is verification that stops at the first hop — Hinds' exact anti-pattern. Bundling dependencies re-creates single-artifact distribution by hand, which is what wasm already is. The registry's own capability model needs the same tightening as the runtime's: unscoped tokens are wildcards (`None => true`, `crates/specforge-registry-server/src/auth.rs:51-53`), `--no-expiry` tokens bypass expiry checks (`auth.rs:11-12, 41-46`), and the fast SHA-256 token hash is acceptable only because raw tokens are 256-bit random (`auth.rs:62-73`). Same disease as C7-04, different organ: defaults grant more than the model needs.

## Scorecard for R-2

| Runtime | Confinement | Limits | Fault isolation | Supply chain fit |
| --- | --- | --- | --- | --- |
| KEEP_WASM | by construction; gaps are one-layer fixes (C7-04/C7-10) | engine primitives, unimplemented | trap, not crash | artifact = code |
| LUA | by subtraction; `require`/FFI/`debug` widen TCB | debug hooks only | process death | dependency shadow |
| PYTHON | fails embedded; sidecar + OS sandbox only | none | process death | worst (ambient ecosystem) |
| TYPESCRIPT (deno_core) | by permission shim, proven | V8 budget | isolate-level | V8 weight vs R-3 |
| MULTI | every tier inherits the weakest runtime's TCB | — | — | — |

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** R-2 confinement is enforced by construction in wasm (no import, no power) but only by convention in any embedded interpreter; the audit's real findings — C7-04's decorative policy ledger, C7-10's unenforced limits — are one-layer fixes to an already capability-shaped architecture, whereas matching that safety in Lua/TS means rebuilding Wasmtime's TCB, and in Python means not embedding at all.
