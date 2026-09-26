# Position — Kevin Knapp (clap creator; CLI/UX surface)

**Verdict:** LUA
**Confidence:** 3

## Arguments
1. **Authoring UX is the CLI contract.** `crates/specforge-cli/src/main.rs` ships a full extension lifecycle (`Add`, `Publish`, `Search`, `ExtensionAction::{Init,Build,Validate}`), but `extension init` scaffolds a cargo project and `Build` presumes a Rust toolchain plus `wasm32-unknown-unknown`. For the primary author — AI agents — every plugin becomes a toolchain orchestration problem the CLI can't see or validate well. A single `.lua` file makes `specforge new --extension` scaffold, `extension build` validate, and completions stay meaningful with zero external deps.
2. **R-3/R-5 flip decisively.** evidence.md: PyO3 needs system Python ("notoriously fragile"); deno_core/V8 is a heavy static dep; mlua vendors C and stays single-binary. Hot reload in `specforge watch` (R-5) is re-reading a script, versus wasm where C7-08 shows the warm-engine story is vaporware and C7-02 the AOT cache is a byte-copy — the current path has no working reload substrate.
3. **Migration is smaller than it looks.** evidence §2: guests are ~97% generated manifest, ~3% real logic; only `@specforge/formal` (478 lines) carries weight. The registry (sha256 + signing, C8-09 fixed) is format-agnostic — blobs stay verifiable under R-4.

## Biggest risk in my verdict
Porting formal's four graph-analysis passes from typed Rust to Lua's thin stdlib loses static safety and debuggability on the one workload with real logic.

## What would change my mind
Proof that guest workloads genuinely need Rust-expressible structure, or C7-02/C7-08 landing for real (warm instances, enforced `max_execution_ms`) making wasm overhead and reload non-issues.
