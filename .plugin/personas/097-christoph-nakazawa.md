# Position — Christoph Nakazawa (Jest creator; testing & BDD)

**Verdict:** KEEP_WASM
**Confidence:** 3

## Arguments
1. Jest's core lesson: a runner stays ignorant of downstream consumers when it emits clean structured events — the plugin point is the protocol, not the language. SpecForge's real defect is C7-03 (no IDL; stringly-typed JSON over `call_export`; drift already happened). Fix that in place; swapping runtimes discards the wasm round-trip test suites without touching the weakness.
2. My anchor, the @specforge/vitest reporter emitting `specforge-report.json`, lives on deterministic, snapshot-testable output (R-6). Wasm guests give that by construction, and the registry already sha256-verifies blobs with pinned signing keys (R-4). Scripting weakens "reproducible artifact" into "re-executed source".

## Biggest risk in my verdict
I argue from the verification lens; primary authors are AI agents, and a Rust toolchain + `wasm32-unknown-unknown` per plugin may strangle the registry bet. `crates/specforge-wasm` is ~9.4k LOC serving ~629 guest lines, mostly manifest — wasm risks being a moat, not a platform.

## What would change my mind
Evidence that agent authors can't ship Rust guests against the SDK macros, or registry data showing zero third-party adoption. A versioned IDL that still drifts falsifies the single-interface bet, favoring MULTI: a scripting tier behind the same protocol.
