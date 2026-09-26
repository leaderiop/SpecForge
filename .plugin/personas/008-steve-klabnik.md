# Position — Steve Klabnik (Rust documentation & language adoption)

**Verdict:** LUA
**Confidence:** 3

## Arguments
1. The adoption funnel is the product. The team itself routes real logic around the wasm boundary: `NativeCustomRules` in `crates/specforge-emitter/src/compile.rs` executes custom rules natively, and the guest `validate__*` exports are never exercised at runtime (evidence §1.2). With ~97% of guest payload being generated manifest (evidence §2), TRPL's bar — working output in one chapter — is unmeetable with rustup + wasm32-unknown-unknown + proc-macro SDK + release blob. A `plugin.lua` meets it.
2. The `extension_json_sync` and `builtin_blob_sync` guard tests exist to keep two representations honest. One Lua artifact collapses manifest and logic into reviewable, diffable text — excerptable verbatim in `docs/guides/extending-specforge.md` and `authoring-spec-files.md`, every inline example CI-runnable, exactly the rust-lang/book docs-as-code model. Wasm blobs are verifiable under R-4 but unreviewable; source review is the supply-chain documentation that matters.
3. R-1 and R-3 eliminate the rest: MULTI re-institutionalizes C7-11 as a supported feature and forks every guide; PYTHON fails R-3 (system interpreter); TYPESCRIPT adds a compile step plus V8 weight. Vendored `mlua` keeps the single binary, a closed-stdlib interpreter plus instruction hooks targets R-2, and reload-a-file gives R-5 a compile-free watch loop.

## Biggest risk in my verdict
Porting `@specforge/formal`'s four passes (~478 lines of typed, tested Rust analysis) into an untyped interpreter endangers the most correctness-critical code in the product; and Lua sandboxing is interpreter convention, not memory isolation — C7-04 (fs allow-by-default) shows defaults are exactly where this repo's discipline slips.

## What would change my mind
Evidence that mlua's instruction hooks cannot reliably enforce CPU limits (R-2/R-6); a KEEP_WASM authoring path collapsed to one manifest file with no toolchain; or deno_core's permission model plus agent-native TypeScript at acceptable binary size. If AI agents are provably the only authors, the human chapter-1 bar weakens considerably.
