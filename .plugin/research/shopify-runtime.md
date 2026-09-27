# Case Study — Shopify's Plugin Runtime: from sandboxed Ruby Scripts to WebAssembly Functions

**Researcher:** `research-shopify-runtime` · **Date:** 2026-09-27
**Question:** How does Shopify sandbox untrusted merchant/app code, what languages are supported, what did the Ruby→Wasm migration look like, what are the limitations, and what should SpecForge learn?
**Method note:** `web_search` was down for most of this session; every claim below is grounded in direct reads of primary sources (shopify.dev docs, GitHub repos/source, archived changelog posts via Wayback CDX, crates.io/npm registries).

## 1. The two generations

**Generation 1 — Shopify Scripts (Ruby, 2018–2025).** Scripts were "customizations written in Ruby" — a "stripped-down version of Ruby" — authored in the Script Editor app, hosted on Shopify's servers, and mutating cart/checkout objects directly (input cart/customer → Ruby mutations → output) [https://github.com/Shopify/shopify-scripts]. Three tightly-scoped types existed (line-item, shipping, payment), only **one script of each type could be published at a time**, which pushed merchants into monolithic scripts [same README, "Noteworthy Limitations"]. The sandbox was interpreter-level: capability stripping (no regex, no date/time, no network by construction) plus "memory, CPU, and character limitations imposed for security and performance reasons" [same README]. This is the shared-address-space, subtract-the-stdlib sandbox model — the model SpecForge's D05 analysis rejects for R-2 (`.plugin/evidence.md` §4, `.plugin/decision.md` "Why" #1).

**Generation 2 — Shopify Functions (Wasm, 2022–present).** Announced in developer preview June 22, 2022 for discount targets [https://web.archive.org/web/20220702113909/https://shopify.dev/changelog/introducing-shopify-functions], rolled out to developers/merchants Oct 2022 [https://shopify.dev/changelog/shopify-functions-begins-rollout-to-developers-and-merchants (CDX snapshot 2022-10-27)]. Functions are Wasm modules registered against typed **targets** (cart-transform, discount, fulfillment-constraints, order-routing, delivery-customization, payment-customization, cart-and-checkout-validation, pickup/local-pickup, …) that Shopify invokes inside checkout itself — "never invoked directly by URL" [https://shopify.dev/docs/api/functions/latest, https://shopify.dev/docs/apps/build/functions].

**Sandbox mechanics (verified in source, not marketing):** function-runner, the reference execution harness, runs modules on **wasmtime** with `consume_fuel(true)` and `epoch_interruption(true)`; CPU is metered as **fuel**, and the documented "execution instruction count" limit is exactly fuel consumed (`STARTING_FUEL − store.get_fuel()`) [https://raw.githubusercontent.com/Shopify/function-runner/main/src/engine.rs]. Modules get **WASI preview1** with `deterministic_wasi_ctx::replace_scheduling_functions` (clocks/randomness neutralized), a `ResourceLimiter` capping memories, and **validated imports** — only `shopify_function_v1/v2` and `shopify_functions_javy_v*` module namespaces are accepted; unknown import combinations are rejected pre-instantiation (`validated_module.rs`, `invalid_import_combination.wat` fixture) [same repo]. The Wasm API is a small import surface using 64-bit NaN-boxed values for **lazy** reads/writes (no JSON parser embedded in the guest) with fixed error/status codes [https://shopify.dev/docs/apps/build/functions/programming-languages/webassembly-for-functions]. Network exists only as a host-mediated **fetch target** — "Shopify makes the HTTP call on your behalf", restricted to Enterprise custom apps [https://shopify.dev/docs/api/functions/latest#fetch-target-limited-access]. Determinism is contractual: "Shopify doesn't allow nondeterminism in functions … you can't use any randomizing or clock functionality" [https://shopify.dev/docs/api/functions/latest#limitations]. And the App Store "doesn't permit apps that provide dynamic editing and execution of function code" [same section].

## 2. Languages

- **Rust** — first-class, `shopify_function` crate v1.0+; "strongly recommended as the most performant language choice to avoid your function failing with large carts" [https://shopify.dev/docs/apps/build/functions/programming-languages].
- **JavaScript/TypeScript** — official via **Javy**, "our JavaScript-to-WebAssembly toolchain": the CLI bundles with ESBuild, then Javy emits a Wasm module containing the app code **and an embedded QuickJS engine** (ES2020). No event loop (`async/await` compiles but throws at runtime), no `fetch`/`crypto`/`setTimeout`/Node globals [https://shopify.dev/docs/apps/build/functions/programming-languages/javascript-for-functions].
- **Anything that compiles to Wasm** (Zig, TinyGo, …) provided the module meets the import/API spec [https://shopify.dev/docs/apps/build/functions/programming-languages].

The load-bearing design decision: **the host never embeds a second runtime.** Multi-language is achieved by making each guest carry its interpreter (QuickJS compiled into the module; Javy can also emit a 220-byte dynamic module that imports a shared `javy_quickjs_provider` linked at instantiation — function-runner reserves exactly 2 memories: "1 for the module, 1 for Javy's provider") [https://shopify.engineering/javascript-in-webassembly-for-shopify-functions, engine.rs]. Why an interpreter rather than V8: "WebAssembly's architecture makes JIT-ing impossible … the memory storing the instructions is completely inaccessible to the module itself. This is by design" — so they colocate a fast C interpreter (QuickJS) in the module [same engineering post]. Javy is now a Bytecode Alliance project with **2,752 stars** [https://github.com/bytecodealliance/javy]; Shopify contracted Igalia to bring SpiderMonkey to Wasm for future JS performance [engineering post].

**Adoption proxies (no official app counts published):** crates.io `shopify_function` shows **625,580 total downloads** (fetched 2026-09-27 via crates.io API); npm `@shopify/shopify_function` is at 2.0.1 with 19 published versions [https://crates.io/api/v1/crates/shopify_function, https://registry.npmjs.org/@shopify/shopify_function].

## 3. The migration timeline (Ruby → Wasm)

| Date | Event | Source |
| --- | --- | --- |
| 2018–2022 | Ruby Scripts era; one script per type, checkout.liquid era | [https://github.com/Shopify/shopify-scripts] |
| 2022-06-22 | Functions dev preview (Rust-first; discounts) | [archived changelog, URL above] |
| 2023-02-13 | Checkout Extensibility announced effective; **checkout.liquid declared dead for in-checkout pages effective 2024-08-13** | [https://web.archive.org/web/20230320021521/https://shopify.dev/changelog/checkout-liquid-will-no-longer-work-for-in-checkout-pages-starting-august-13-2024] |
| 2023-02-09 | Javy engineering post: original constraints were "≤256 KB module, ≤5 ms, JSON via stdin/stdout"; 5 ms wall-clock admitted machine-dependent, "exploring a gas-like approach" → became fuel/instructions | [https://shopify.engineering/javascript-in-webassembly-for-shopify-functions] |
| 2023-03/04 | JavaScript local dev preview; "Write Shopify Functions in JavaScript" | [CDX snapshots 2023-03-25, 2023-04-10] |
| 2023-05-08 | Input JSON limit 64,000 bytes; metafields ≤10,000 bytes | [https://web.archive.org/web/20230508211340/https://shopify.dev/changelog/shopify-functions-input-limit-updates] |
| 2024-10 | "JavaScript Shopify Functions are now 40% faster" (Javy provider v3) | [https://shopify.dev/changelog/javascript-shopify-functions-are-now-40-faster (CDX 2024-10-10)] |
| 2024-08-13 | checkout.liquid stops working for Information/Shipping/Payment pages | [same 2023-02 announcement] |
| 2025-01 | Input limit raised 64 kB → **128 kB** | [https://shopify.dev/changelog/shopify-functions-input-size-limit-increased-to-128kb (CDX 2025-01-24)] |
| 2025-06 | 25-functions-per-app limit introduced | [https://shopify.dev/changelog/shopify-functions-25-functions-limit (CDX 2025-06-11)] |
| **2025-08-28** | **Shopify Scripts removed** — "deprecated and will be removed on August 28, 2025" | [https://github.com/Shopify/shopify-scripts README banner → https://changelog.shopify.com/posts/shopify-scripts-deprecation] |

The arc: **~7 years** from embedded-Ruby sandbox to full removal, with a 3-year Functions runway, explicit sunset dates, and quarterly versioned GraphQL schemas as the compatibility contract (`api_version` in `shopify.extension.toml`, `shopify app function schema` codegen) [https://shopify.dev/docs/api/functions/latest#graphql-schema-and-versioning].

## 4. Limitations (current, all from [https://shopify.dev/docs/api/functions/latest#limitations])

| Resource | Limit |
| --- | --- |
| Compiled binary size | 256 kB |
| Runtime linear memory | 10,000 kB |
| Runtime stack | 512 kB |
| Logs | 1 kB (truncated) |
| Instruction count (fuel) | 11 million, **scales with cart size** past 200 line items |
| Function input | 128 kB (dynamic) |
| Function output | 20 kB (dynamic) |
| Input query | ≤3,000 bytes, cost model capped at 30, list args ≤100 |

Qualitative: no clock/randomness (determinism), no direct network (host-mediated fetch target only), no stdout/stderr logging (dedicated logging import), no guest-side dynamic code (store policy + W^X), and dynamic languages hit the fuel ceiling sooner — Javy functions measured "about 3x slower" than the equivalent Rust module in Shopify's own tests [engineering post]. Note the honest evolution: the original **5 ms wall-clock** limit was publicly acknowledged as "machine-dependent and situational" and replaced by fuel [same post] — precisely the C7-10 class of bug SpecForge's auditor flagged from the other direction (`max_execution_ms` documented but never enforced).

## 5. What SpecForge should learn

Against `.plugin/evidence.md` (R-1..R-6, C7 cluster) and `.plugin/decision.md`:

1. **The industry's largest untrusted-code deployment converged on exactly SpecForge's architecture**: one host engine (wasmtime), imports-are-capabilities (validated import namespaces ≈ `host_emit_diagnostic`/`host_read_file` in `crates/specforge-extism/src/host_context.rs`), fuel-bounded execution, memory limits, determinism by construction. This is direct external validation of KEEP_WASM's core (D05, D09) — not from a blog post, from the reference runner's source.
2. **"MULTI" is solved with toolchains, not host runtimes.** Shopify supports two-plus languages while running a single engine, by compiling each language to Wasm and letting the **guest carry its interpreter** (QuickJS-in-module, dynamic provider linking). SpecForge's LUA/MULTI options (`.plugin/evidence.md` §4) embed mlua/PyO3 **in the host** — the model Shopify explicitly moved *away* from. If SpecForge ever wants Lua/TS authoring, the Shopify-pattern path is a Lua→wasm build target, which also satisfies the D12 consolidation dissent *inside* the sandbox; this is exactly the first "condition for revisit" in `.plugin/decision.md`.
3. **Enforce the budget the docs promise.** Shopify ships fuel+epoch in the runner and surfaces per-execution calculated limits in a Dev Dashboard and CLI; SpecForge documents `max_execution_ms = 30_000` while nothing enforces it (C7-10), and the "AOT cache" is a byte-copy (C7-02) where function-runner uses wasmtime's `Cache` for real. These are the gaps that make wasm look slower than it is.
4. **Payload shaping beats runtime swapping.** Shopify kept the input contract small with GraphQL input queries (cost-capped at 30) instead of shipping whole carts — the same move as fixing SpecForge's ignored `query_scope` (C7-09): declare what you need; host enforces cost.
5. **Versioned typed contracts, not stringly JSON.** Quarterly GraphQL schemas + generated types (Rust codegen, TS typegen) is the answer to C7-03's drift — a WIT/IDL is the equivalent fix already planned in `.plugin/decision.md`.
6. **Deny-by-default and no ambient authority at distribution time.** App Store bans dynamic function code entirely; C7-04's allow-by-default `file_system_access` is the exact anti-pattern Shopify's import allowlist makes structurally impossible.
7. **Sunsets need dates.** Shopify killed its Ruby sandbox only with a published deadline (2025-08-28) and a multi-year runway. SpecForge is pre-registry, pre-1.0: deleting the native mirrors (C7-11) now is cheap; it never gets cheaper than this moment.

**Counter-evidence worth recording:** Shopify's JS support took ~18 months of dev preview (Feb 2023 → deploy GA in 2023) and a 3x-slower interpreter tax; a Wasm-only host with Rust-first SDK *did* deter the JS majority until Javy shipped [engineering post]. SpecForge's registry bet should expect the same dynamic-language demand curve, and the answer is a guest-side toolchain, not a second host runtime.

## Bottom line

**KEEP_WASM** — confidence **5** — Shopify, running the highest-stakes third-party-code workload in commerce, independently converged on SpecForge's exact model (single wasmtime host, capability imports, fuel, deterministic contract, guest-carried interpreters for extra languages), proving both the KEEP_WASM baseline and that LUA/MULTI language demands are met by compile-to-Wasm toolchains rather than embedding interpreters in the host.
