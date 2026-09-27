# Case Study: Envoy Proxy Wasm Filters in Production

*Fleet input for the SpecForge plugin-runtime decision — case: KEEP_WASM vs embedded Lua/Python/TypeScript vs MULTI. Researcher: `research-envoy-wasm`. All claims carry inline citations (URLs or repo paths).*

Envoy is the highest-stakes production deployment of embedded Wasm plugins: per-request network filtering at line rate, in C++, with untrusted-ish third-party modules. It is the closest thing to a longitudinal field study of the architecture SpecForge already has.

## 1. How Envoy embeds Wasm

**Architecture: in-process VMs behind a stable ABI.** Envoy's Wasm story exists because it has no stable C++ extension ABI — every extension change meant a rebuild, re-release, and rolling restart of a statically linked binary. The fix was the **Proxy-Wasm ABI** ("WebAssembly for Proxies"): a proxy-agnostic contract of exported/host functions over shared linear memory, now an independent spec with 607 stars and host implementations in Envoy, NGINX (Kong's WasmX), Apache Traffic Server, MOSN, and OpenResty (https://github.com/proxy-wasm/spec).

**Runtime engines are pluggable and Envoy ships one by default.** `VmConfig.runtime` selects among `envoy.wasm.runtime.v8`, `.wasmtime`, `.wamr`, and `.null` (module compiled natively into the binary — a "null sandbox" used for testing/debugging). The search order at build time is **v8 → wasmtime → wamr**, and notably *Wasmtime and WAMR are not enabled in Envoy's official build*; V8 is the production default (https://www.envoyproxy.io/docs/envoy/latest/api-v3/extensions/wasm/v3/wasm.proto).

**VM lifecycle model.** Envoy instantiates one persistent VM **per worker thread** per module (a listener thread count × module multiplier), shareable across plugins via `vm_id` to cut memory — with the explicit caveat that sharing "may have security implications." The design doc enumerates the alternatives and their costs: per-request ephemeral VMs are "prohibitively expensive," and out-of-process sandboxes are recommended only for untrusted multi-tenant deployments needing Spectre-grade isolation (https://github.com/proxy-wasm/spec/blob/main/docs/WebAssembly-in-Envoy.md; wasm.proto `vm_id` docs).

**Policy machinery is rich:** `fail_open` (bypass filter) vs default `FAIL_CLOSED` (503), `FAIL_RELOAD` with backoff, per-VM capability allowlists, WASI syscall gating, `nack_on_code_cache_miss` for remote loads, and checksum/signature recommendations (wasm.proto above). Two gaps stand out: the sanitization half of capability restrictions is "currently unimplemented," and Envoy's own API metadata still marks the wasm extension as having "not had substantial production burn time" and an "unknown security posture … only used in deployments where both the downstream and upstream are trusted" (wasm.proto; the HTTP filter itself is labeled "experimental": https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/wasm_filter).

**Distribution:** modules load from local disk, inline xDS, remote URL, or OCI images. Istio wraps this with its agent + Extension Configuration Discovery Service because naive remote fetch is dangerous (see lesson 4).

## 2. Performance overhead

Envoy's numbers are conservative commitments from the original design doc rather than marketing benchmarks (WebAssembly-in-Envoy.md, "Drawbacks"):

- **CPU-bound plugins: slowdown expected below 2× vs native code** — the host's own upper bound.
- **Payload-transcoding plugins pay copy costs in and out of the sandbox** — the dominant penalty, not instruction speed.
- **Memory scales per VM** ("many virtual machines, each with its own memory block"), and the runtime adds **~10 MB (V8) to ~20 MB (WAVM)** of binary size.
- **VM creation and draining is "expensive"** (Envoy issue #11547, https://github.com/envoyproxy/envoy/issues/11547) — hence persistent per-worker VMs, never per-request.

Cross-proxy corroboration for the *in-process vs out-of-process* trade: Kong ranks its plugin tiers as **native Lua plugins (in-process LuaJIT) fastest, then in-process Proxy-Wasm filters, and slowest are out-of-process PDK plugins** that pay IPC per call (https://web.archive.org/web/20240506195419/https://konghq.com/blog/engineering/proxy-wasm). Istio's original motivation was the same axis: its out-of-process Mixer extension model caused "significant resource inefficiencies that impacted tail latencies," which in-process Wasm replaced (https://web.archive.org/web/20210110191206/https://istio.io/latest/blog/2020/wasm-announce/).

No authoritative public end-to-end latency number for a simple Envoy Wasm header filter exists that I could verify; the honest summary is: per-call ABI crossings are cheap, per-VM memory and payload copies dominate, and everything hinges on amortizing VM startup via pooling.

## 3. SDK languages

Officially listed SDKs: **Rust, C++, Go (TinyGo), AssemblyScript** (https://github.com/proxy-wasm/spec, "Implementations"). In theory "over 30 languages" compile to Wasm (wasm-announce); in practice the surface is much narrower:

- The Go SDK (Tetrate) is the most-starred at **701 stars, 180 forks — and archived**, with development moved to the proxy-wasm org fork (119 stars) (https://api.github.com/repos/proxy-wasm/proxy-wasm-go-sdk).
- **No Python-based Proxy-Wasm SDK exists** — Kong's answer to "Can I write a filter in Python?" is flatly no (Kong proxy-wasm blog, Oct 2023). The design doc also warns that non-C/C++/Rust targets carry host-environment assumptions (early Go-on-wasm expected a JavaScript host).
- Kong maintains its own AssemblyScript fork "temporary SDK"; community examples exist for it, but first-class status is aspirational (Kong/ngx_wasm_module README, https://github.com/Kong/ngx_wasm_module).

Six years in, the actually maintained Proxy-Wasm SDKs are Rust and Go/TinyGo, with C++ legacy-maintained — even when the host is an organization (Google, Tetrate, Kong, Red Hat) that could afford more.

## 4. Production lessons

1. **The distribution/fetch pipeline is the CVE surface, not the sandbox.** Envoy Gateway shipped three CVEs in Sept 2026, all in its Wasm module *fetching and extraction* path: an untrusted tar-header size drives pre-validation allocation (CVE-2026-53717, CVSS 6.5), an unbounded gzip decompression bomb (CVE-2026-53716), and an unsynchronized map race killing the controller (CVE-2026-53715) — "persistent cross-tenant control-plane outage" (https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch=envoy%20wasm). Nobody exploited the Wasm VM; they crashed the controller *around* it.
2. **Remote distribution needed a redesign.** Inlining MB-sized modules in xDS caused ADS head-of-line blocking; a failed URL fetch after ACK "can be an unrecoverable configuration error" (issue #11547). Istio's fix — agent-side fetch + ECDS, agent rejects bad updates before Envoy sees them — exists precisely because a fail-closed Wasm plugin with a bad remote fetch "will stop Envoy from serving" (https://web.archive.org/web/20210305210827/https://istio.io/latest/blog/2021/wasm-progress/). OCI images plus the WasmPlugin CRD arrived only in Istio 1.12 (2021), with **signature verification still listed as future work** (https://web.archive.org/web/20211218123023/https://istio.io/latest/blog/2021/wasm-api-alpha/).
3. **Failure states are sticky and observability lagged adoption.** "Any unrecoverable error in the Wasm VM will panic it, and all coming requests will get 503. Users have to reload the wasm filter or restart the envoy process" — and users had no health endpoint to detect it (issue #34881, open since June 2024, https://github.com/envoyproxy/envoy/issues/34881). A per-runtime `crashed` metric (covering "wasm oom or panic") only materialized as a PR in Aug 2025 (https://github.com/envoyproxy/envoy/pull/40882).
4. **Engine choice is real but each engine has gaps.** Envoy defaults to V8 and excludes Wasmtime/WAMR from official builds; Kong defaults to Wasmtime while also embedding Wasmer and V8; and switching engines surfaced bugs — Envoy + WAMR + concurrent plugin execution produced "out of bounds memory access" traps (issue #39532, https://github.com/envoyproxy/envoy/issues/39532). Runtime abstraction is worth it; per-engine conformance testing is part of the price.
5. **Status honesty is a feature.** Envoy kept the filter "experimental" and its API docs flag "unknown security posture," even as Istio ran it fleet-wide since 2020. Production readiness came from *constrained deployment posture* (trusted modules, single-tenant), not from the sandbox alone being enough for arbitrary multi-tenancy (design doc's out-of-process caveat).
6. **Ecosystem gravity beats language pluralism.** Despite the "any of 30 languages" pitch, tooling consolidated on a registry (WebAssembly Hub), one distribution format (OCI), and two live SDK languages — platform work, not language work, absorbed the effort (wasm-announce; wasm-api-alpha).

## 5. Comparison to SpecForge's use case

| Axis | Envoy network filters | SpecForge spec analysis |
|---|---|---|
| Call pattern | Per-request, millions/sec, µs budgets | Batch CLI/validator/collector calls; seconds-scale budget (`max_execution_ms = 5000`, `docs/extension-sdk.md:95`) |
| Failure blast radius | Whole proxy: 503s per request until reload (issue #34881) | One CLI run fails; process exits — sticky-failure class largely absent |
| Isolation need | Third-party filters, multi-tenant, "unknown security posture" | Untrusted registry uploads anticipated (`.plugin/dimensions/D01-wasm-status-quo.md`:7-11) |
| Distribution | xDS/HTTP/OCI, required agent redesign + CVEs | Content-addressed SHA-256 vendored blobs, pinned keys (`D01`:11) |
| Determinism | WASI clock/random gated per-VM (wasm-api-alpha) | No host-imported clock/random; determinism is an R-6 requirement (`D01`:12) |
| SDK breadth | 4 SDKs → 2 actively maintained after 6 years | Rust-first SDK; polyglot inherited from `wasm32` target (`D01`:14) |

The Envoy experience validates SpecForge's current architecture on the dimensions that hurt Envoy most, and shows where the marginal risks lie:

- **The hard lessons are all outside the runtime.** Envoy's six-year pain was distribution integrity, sticky failure states, and observability — not "Wasm was too slow." SpecForge's SHA-256-pinned registry and short-lived batch executions mean its risk profile starts past Envoy's 2021 state; the corresponding must-keeps are deny-by-default sandboxing (Envoy shipped allow-ish defaults and an unimplemented sanitization half) and per-plugin time/memory enforcement (SpecForge's C7-04/09/10 findings mirror exactly the machinery Envoy needed: wasm.proto `CapabilityRestrictionConfig`, `FAIL_RELOAD`).
- **Warm pooling is the correct borrowed pattern.** Envoy's "ephemeral per-request VM is prohibitively expensive" is the citation for SpecForge's C7-08 fix: keep instantiated plugin handles across the handshake-describe-call sequence instead of re-instantiating (`.plugin/dimensions/D01-wasm-status-quo.md`:22). Envoy also shows V8-vs-Wasmtime engine churn is survivable but never free — a reason to keep the Extism/Wasmtime abstraction rather than accumulate engines (MULTI).
- **The 2×-worst-case CPU bound is irrelevant here.** SpecForge payloads are KB-scale graph snapshots over seconds-long budgets; the Envoy-relevant costs are memory copies at the boundary — which the single-snapshot `PassInput`/`Entity` design already minimizes — and per-VM memory, bounded at 256 MB.
- **SDK breadth is a trap Envoy already fell into.** Four SDKs shrank to two maintained ones despite billion-dollar backers. SpecForge's Rust-only SDK with a versioned handshake (`PROTOCOL_VERSION`, `__handshake`) matches how Proxy-Wasm actually converged; adding a Lua/TS tier (LUA/TYPESCRIPT/MULTI) re-creates the "temporary fork" maintenance pattern Kong documented for AssemblyScript.

Counterpoint to keep honest: Envoy's model needed an entire ecosystem (agent, ECDS, OCI, crash metrics) because plugins live forever inside a long-running daemon. SpecForge's process-scoped execution gets several of those properties for free and therefore cannot claim Envoy as evidence that its engineering burden will match Envoy's — only that the architecture survives the strongest stress test the industry has run.

## Bottom line

**Verdict:** KEEP_WASM — **Confidence: 4** — Envoy's six years of production evidence show the Wasm-in-process architecture failing only at the edges Envoy created for itself (hot-path pooling, xDS distribution, multi-tenant posture) while consistently beating out-of-process alternatives on latency and safety, which is precisely the trade SpecForge's batch, content-addressed, deny-by-default design sits on the right side of.
