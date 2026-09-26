# D02 — LUA embedding assessment (mlua / Lua 5.4)

**Analyst:** 015 — Björn Linse (Neovim core)
**Dimension:** `LUA` — replace Extism/Wasmtime with an embedded Lua interpreter

Neovim's own history is the sales pitch and the warning. Lua won editor scripting because embedding cost is near-zero and the authoring loop is edit-and-run — no toolchain, no target triple, one text file. But Neovim *trusts* its plugins; SpecForge must not (R-1, R-2). The question is whether `mlua` + Lua 5.4 carries a single-runtime, untrusted workload. The workload itself is favorable: evidence.md §2 shows the four guests are **~97% generated manifest, ~3% logic**, and the only logic-bearing guest (`extensions/formal/src/lib.rs`, 478 lines) is four passes that filter entity arrays and emit diagnostics — `condition_check` is a `kind == "behavior"` loop pushing warnings (lib.rs:72–94), `layering_verify` a refinement-edge walk. That is table iteration and string formatting: Lua's home turf. The host-heavy parts (1.7k-entity graph, registry, LSP) stay Rust regardless.

## Sandboxing under R-2

mlua (0.12) makes capability selection an explicit constructor argument, which is exactly the deny-by-default model C7-04 found missing in the wasm sandbox:

- `Lua::new()` loads `StdLib::ALL_SAFE` — but per mlua's `src/stdlib.rs` that set *still includes `io`, `os`, `package`* (it excludes only the `debug` library and LuaJIT FFI, returning `SafetyError` on both, and disables C-module loading when `package` is present, `src/state.rs`). Same lesson as C7-04: defaults are permissive. SpecForge must construct states with `new_with(ALL_SAFE & !IO & !OS)` — the mechanism is first-class and enforced at load time.
- **CPU limit:** `set_hook(HookTriggers::every_nth_instruction(n))` — mlua's own docs name this as the execution-limit mechanism. This *implements* the never-enforced `max_execution_ms = 30_000` (C7-10).
- **Memory limit:** `set_memory_limit` / `used_memory` (Lua 5.2+; unavailable on LuaJIT) gives an OOM roof per plugin.
- No threads, no sockets, no processes exist unless mounted. Collector-style I/O becomes a host-provided function — capability-scoped access by construction (R-2).
- Escalation path: mlua also exposes Luau's `sandbox(true)` (read-only globals/metatables, safeenv). Strongest interpreter sandbox in the ecosystem, at the cost of the 5.4 stdlib and corpus.

Precedents: Redis (sandboxed Lua — io/os stripped, deterministic script replication), World of Warcraft, ComputerCraft, Roblox/Luau. Neovim is the *embedding-DX* precedent, not a sandbox one.

## Distribution, startup, hot reload (R-3, R-5)

The `vendored` feature compiles Lua's C sources statically via the `lua-src` crate — no system packages, arm64-macOS and linux-x64 covered by mlua's CI matrix; single-binary distribution survives, and wasmtime 43 plus the 1.4 MB of vendored guest blobs (evidence §1.1) leave the dependency tree. State creation is microseconds-scale, so a warm `Lua` per extension held in-process is the *obvious* architecture — the thing `EnginePool` only pretended to be (C7-08). The broken AOT cache (C7-02) has no analog to fix: there is nothing to precompile. Hot reload (R-5) becomes drop-state/recreate-state on file change in `specforge-watch` — the safest reload story possible, since a fresh sandbox cannot leak globals between runs. Skip LuaJIT for the host: memory control unavailable, FFI is a sandbox hole, and GC64/linker friction on macOS arm64 is exactly the kind of per-platform scab R-3 forbids. The Lua 5.4 interpreter is ample for O(n) passes over 1.7k entities.

## Determinism under R-6

Diagnostics flow as arrays — output order is code order. The formal passes iterate arrays (`ipairs`) and read named fields, so their semantics port 1:1 and stay snapshot-testable. The real trap is `pairs()`: iteration order over hash-shaped tables is unspecified. Any plugin emitting output in `pairs` order breaks snapshots — so the host must sort/dedupe diagnostics at the boundary and the authoring guide must mandate sorted-key emission. Exclude the `os` library and fix `math.random` seeding in the host, and the classic Redis-style determinism hazards are closed at the source.

## Authoring for AI agents

The product's stated primary author is an AI agent (evidence §1.5). Lua's training corpus (Neovim configs, game mods, Redis/OpenResty scripts) is enormous relative to the language's size, and the edit loop collapses to: agent writes `.lua` → host sandbox-runs it against fixture graphs → line-numbered errors return as diagnostics. No Rust toolchain, no `wasm32-unknown-unknown` target, no registry publish round-trip. The weakness is honest: dynamic typing surfaces typos at run time, not compile time. Mitigation is that manifests are data the host schema-validates anyway, and trial execution at install/verify time is cheap enough to be a registry step.

## Migration path from the four wasm builtins

1. **Host API v2:** one typed Rust definition exposed to plugins as a `specforge` module (`specforge.entities(kind)`, `specforge.diagnose(...)`, `specforge.json_decode`) — generated from a single source, the anti-C7-03 move. Keep the 9 `describe_*.json` categories as the manifest payload, returned as tables.
2. **Port order:** manifests (mechanical) → collectors and `validate__*` hooks → formal's four passes, one at a time, gated by the existing `pass_tests` fixtures (`extensions/formal/src/lib.rs:309–478`) and the 3,070-test workspace re-run against the Lua implementations. Note the C6-11/C10-10 fix (custom rules dispatched natively in `crates/specforge-emitter/src/compile.rs` because wasm `validate__*` never fired) becomes obsolete: an in-process call cannot silently miss.
3. **Convergence:** builtins ship as ordinary `.lua` through the same sandboxed load path as third-party plugins — R-1 satisfied literally. The native mirrors in `crates/specforge-emitter/src/builtins/` are deleted once ports pass; C7-11's three mechanisms collapse to one.
4. **Deletions:** `crates/specforge-wasm` (~9.4k LOC), extism/wasmtime, vendored blobs, the extension-sdk guest macros. Registry keeps sha256 + signing, now over plain source; reproducibility is trivial with no cross-compilation — C7-00's failure class disappears.

## Risks and what Lua does not solve

- **mlua bus factor:** one lead maintainer. Offset by the frozen-stable Lua C API and a thin wrapper; worst case is re-binding work, not redesign.
- **Hook granularity:** instruction-count limits are approximate and adversaries can strain hooks; the memory roof plus registry-signed distribution bound the exposure. Luau is the escalation if adversarial authors materialize.
- **No free IDL:** C7-03 must be re-solved *in* the Lua host API design; embedding does not automate it away.
- **Port risk:** formal's 478 lines get re-verified once — do it strictly behind the existing snapshot gate.

## Verdict

**Verdict:** LUA
**Confidence:** 4
**One-line rationale:** The workload is ~97% declarative manifest and ~3% table-crunching passes; mlua on Lua 5.4 delivers explicit deny-by-default std-lib selection, real instruction/memory limits, trivially warm in-process instances, static single-binary linking, and the richest AI-authoring corpus — dissolving C7-02/03/04/08/10/11 rather than patching them, at the one-time cost of porting formal's four passes behind existing snapshot tests.
