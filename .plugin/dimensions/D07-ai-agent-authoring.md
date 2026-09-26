# D07 — AI-Agent Plugin Authoring

The primary plugin author is an AI coding agent working with a human. That inverts the usual ergonomics question. What limits an LLM author is not fluency — Python, TypeScript, Rust, and Lua are all abundantly represented in training data — but the quality of the **feedback loop**: how fast, how locally, and how unavoidably a wrong guess becomes a hard failure. Karpathy's Software 3.0 framing (YC AI Startup School, 2025) calls this the partial-autonomy loop: the agent generates, the machine checks, the agent repairs. Whatever runtime maximizes machine-checking per iteration wins this dimension. His "vibe coding" posture — acceptable for throwaway scripts, explicitly a much higher bar for serious work — is the right calibration: registry-distributed plugins that run inside users' analyze pipelines are serious work.

## 1. Hallucination surface = ambient surface + bespoke surface

The **bespoke** surface — the SpecForge-specific API a model has never seen, since the SDK is v0.1.0 — is runtime-independent and remarkably small. `crates/specforge-extension-sdk-macros/src/lib.rs` is 169 lines exposing two macros. `#[extension(name = ...)]` wraps a `Contributions::contribute(&mut builder)` impl and generates `__handshake`/`__describe` (lib.rs:50-102); `#[compiler_pass(name)]` wraps a typed `fn(&PassInput) -> Vec<PassDiagnostic>` in a JSON (de)serializing export (lib.rs:143-169). It even enforces the export name at compile time via a `const` check (lib.rs:162-166). Crucially, per evidence §2, ~97% of the guest payload is generated declarative manifest (9 JSON categories) and ~3% is real logic. Declarative data with a schema is the least hallucination-prone content that exists; the macros already generate it.

The **ambient** surface is where runtimes diverge sharply:

- **Rust → `wasm32-unknown-unknown`:** the compilation target has no ambient OS surface. `std::fs`/`std::net` are inert stubs that error at runtime and `std::process::Command` does not exist. A fully hallucinated ambient I/O call cannot touch the host — the platform ABI *is* the capability system, independent of host configuration. Hallucinated plugin logic (wrong types, missing trait impls, nonexistent builder methods) fails the build with precise, local errors that models repair extremely reliably.
- **TypeScript (Deno):** `deno check` against a host-provided `.d.ts` catches invented APIs as type errors — nearly as good. But the permission boundary is configuration, not structure. The project already demonstrated how config boundaries drift: C7-04 shipped the wasm sandbox `file_system_access` allow-by-default. QuickJS has neither types nor permissions.
- **Python:** the trap inverts fluency into risk. A model *will* write idiomatic `pathlib.read_text()` or `import os`; under PyO3 those imports succeed because CPython cannot be reliably sandboxed (evidence §4: "none by default"). The worst failure combination — fluent, plausible, silently capability-violating — is Python's native mode.
- **Lua:** absent keys read as `nil`, so a hallucinated host-call name does not error at the hallucination site; the error (or silent `nil` propagation into data) surfaces later at first use, detached from the cause, in an untyped script. The least localizable failure mode of the four.

## 2. API surface size and the context budget

Karpathy's LLM OS analogy (context window as RAM) has a direct corollary: the host API is a set of "drivers" the model must hold in its window, and every unspecified byte of surface is hallucination fuel. The Rust SDK's contract is single-sourced — the typed structs in `specforge-extension-sdk` *are* the manifest schema. C7-03 (no IDL, stringly JSON over `call_export`, "drift already happened") shows what happens when contracts live in prose; a TYPESCRIPT option would re-create that problem as hand-maintained `.d.ts` drift, and a MULTI option doubles the documentation an agent must page in and the review semantics a reviewer must master — institutionalizing C7-06's "one runtime" falsehood rather than converging per R-1.

## 3. Review-ability and what-you-review-is-what-runs

Honest pro-scripting point: `.lua`/`.ts`/`.py` ship as source, so the reviewed bytes equal the executed bytes. A wasm blob re-introduces a source↔artifact trust gap that R-4's reproducibility requirement must close (locked toolchain, pinned crates — achievable, not free). Against that: the primary reviewer here is also an AI, and typed Rust is where AI review is strongest; macro-generated exports shrink the reviewed surface to the 3% logic; and the registry already has integrity machinery (signing, sha256 pinning — evidence §1.4). Willison's position applies directly: LLM output must be reviewed as if from a keen but unreliable contributor, and generated code must be *sandboxed* — structurally, not advisory. Wasm makes the sandbox non-negotiable; Deno makes it a flag; CPython makes it a hope.

## 4. The spec/verify loop and R-6

Ranked by how mechanically wrongness is caught: Rust (build-time types + inert target + deterministic snapshots — hermetic by construction, so R-6 holds without discipline) ≥ TypeScript (`deno check`, permission-denied at runtime) > Python (runtime raises, leaky imports, hash-randomization/float hazards for snapshots) > Lua (use-time nil errors, silent propagation). The toolchain tax is real (evidence §1.5: Rust toolchain + wasm32 target required), but it is front-loaded and one-time per environment — and agents install toolchains routinely — whereas scripting's costs are back-loaded as silent failures and capability escapes. The project's own history proves the category: C6-11/C10-10 ("custom wasm rules can never fire / fail silent") was the cost of a path where wrongness stayed invisible.

Chase's reliability thesis (narrow, schema-validated tools; reliability = constrained degrees of freedom) and Willison's "confidently wrong" framing converge with Karpathy's loop on the same design rule: make the contract small, typed, and machine-enforced. The SDK macros already embody it.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** For AI authors, unverifiable fluency (Python/Lua) and config-gated safety (TypeScript) lose to a runtime where hallucinated capabilities are structurally inert and hallucinated logic fails a typed build — the fastest machine-checkable repair loop, with TypeScript (deno_core + single-sourced `.d.ts`) as the credible fallback if toolchain friction measurably blocks adoption.
