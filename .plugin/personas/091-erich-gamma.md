# Position — Erich Gamma (extension architecture; JUnit co-creator, VS Code lead)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. R-1 is the VS Code lesson: every extension — ours included — runs on the same extension host via the same marketplace. Wasm is one equal mechanism for builtins and community plugins; Lua/TS tiers would recreate the trusted-native split audit finding C7-06 already caught three ways (`crates/specforge-emitter/src/builtins/`, embedded blobs, SDK crates).
2. Model lives in the host, not the shell. `integrations/vscode/src/extension.ts` stays a thin forwarder to specforge-lsp; the same discipline keeps guests small and declarative — evidence §2: guests are ~97% generated manifest, ~3% real logic (`formal`'s 478 lines the only code-bearing guest). That payload shape makes this an isolation-and-determinism decision (R-2, R-6), not a language-ergonomics one.
3. Fix the interface, not the runtime: C7-03's stringly-typed `call_export` JSON is the real debt — the same ad-hoc-JSON problem that produced LSP. An IDL in `specforge-extension-sdk-protocol-types`, deny-by-default sandbox (C7-04), and a genuine engine pool (C7-08) satisfy the criteria inside the current model.

## Biggest risk in my verdict

Unfixed C7-02/C7-08 (byte-copy "AOT", ledger-only engine pool) means per-call compile cost; Rust + wasm32 authoring makes R-5 hot reload feel sluggish, tempting a script-tier escape hatch that fragments the model.

## What would change my mind

Proof guests need ambient ecosystem access (collectors scraping live formats), or that pooling/AOT can't make per-call overhead acceptable — then embed Lua behind one typed protocol, never MULTI.
