# In-Process vs Out-of-Process Plugin Execution — Evidence Report

**Fleet topic:** embed-vs-sidecar · **Batch:** 2 · **Date:** 2026-09-27
**Method note:** `web_search` was unavailable (upstream auth failure); all evidence below is from
directly fetched primary sources (project docs, security advisories, vendor engineering posts).
Context inputs: `.plugin/decision-brief.md`, `.plugin/evidence.md`.

---

## 1. The three architectures (what boundary is actually enforced)

| | A. In-process embedded interpreter | B. Sidecar process + IPC | C. In-process memory-sandboxed VM (WASM / isolate) |
| --- | --- | --- | --- |
| Examples | CPython via PyO3, mlua/LuaJIT, Emacs/Neovim Elisp-Lua, Redis embedded Lua, Obsidian plugins | HashiCorp go-plugin (Terraform/Vault), VS Code extension host, LSP servers, Datadog-agent-style helpers | Extism/Wasmtime, Cloudflare Workers (V8 isolates), Shopify Functions, Figma (QuickJS compiled to WASM) |
| Crash domain | **Shared.** A segfault in plugin C code kills the host | **Separate.** Plugin panic ≠ host panic | **Separate (memory-level), same process.** Traps abort the instance, not the host |
| Memory isolation | None (same address space) | Full (OS process) | Full (linear-memory sandbox per instance) |
| Payload cost | Zero-copy possible; direct struct access | **Serialization dominates** (see §3) | In-process host-call ABI; guest copy of payload (linear memory) |
| Sandbox story | Interpreter flags only — historically escapable | Strong by construction | Strong by construction; bugs handled by defense-in-depth + patch pipeline |
| Distribution | Requires matching interpreter runtime on machine (breaks single-binary) | Plugin is a self-contained binary; host needs only OS | Static engine in host; guest blobs are portable |

## 2. Real projects and outcomes

### A. In-process embedded interpreter — documented escape history

- **Figma (Realms shim, 2019).** Sandboxed plugin JS *inside the same JS VM* ("hide the globals" via
  `with` + `Proxy`). Several independent vulnerabilities "could have allowed code inside the sandbox
  to escape"; the root cause class: "the Realms shim uses the same JavaScript VM for all code both
  inside and outside the sandbox", so inside/outside objects can be confused. Figma permanently
  switched to **QuickJS cross-compiled to WebAssembly** — "not possible to confuse objects from
  outside with objects from inside because the object representations are too different" — accepting
  "somewhat slower for certain plugins, but intrinsically more secure."
  Sources: [An update on plugin security](https://www.figma.com/blog/an-update-on-plugin-security/),
  [How we built the Figma plugin system](https://www.figma.com/blog/how-we-built-the-figma-plugin-system/).
- **Redis embedded Lua (Debian build), CVE-2022-0543.** Lua sandbox escape → remote code execution,
  CVSS v3.1 **10.0**, present in the CISA Known Exploited Vulnerabilities catalog. The escape
  leveraged leftover globals in the dynamically-linked Lua library (`package`/`loadlib`) — i.e.
  interpreter-level sandboxing failed at the library boundary.
  Source: [NVD CVE-2022-0543](https://nvd.nist.gov/vuln/detail/CVE-2022-0543).
- **Datadog Agent (Go binary with embedded Python checks).** Decades-long real deployment, but the
  host is version-coupled to the interpreter: current docs require a *specific* interpreter
  (Python 3.13) matching "the one used by the latest Agent" for integration development — the
  embedding owns the runtime version.
  Source: [Datadog Agent Integration Developer Tool](https://docs.datadoghq.com/extend/integrations/python/).
- **Outcome pattern:** ergonomic and fast; sandbox = interpreter conventions, not a memory boundary.
  Every surveyed same-address-space sandbox that hosted *untrusted* code eventually either escaped
  (Figma, Redis) or lives with a full-trust model (editors, Obsidian-style apps that gate by review
  instead of isolation).

### B. Sidecar process + IPC — strong isolation, payload-shaped costs

- **HashiCorp go-plugin** (Packer, Terraform, Nomad, Vault, Boundary, Waypoint; "millions of
  machines", >4 years in production at time of writing). Stated rationale:
  "Plugins can't crash your host process: A panic in a plugin doesn't panic the plugin user";
  "The plugin only has access to the interfaces and args given to it, **not to the entire memory
  space** of the process"; checksum verification + TLS; protocol versioning; reattach.
  Crucially, HashiCorp *concedes the perf point on purpose*: "Shared libraries have one major
  advantage … much higher performance. In real world scenarios … **we've never required any more
  performance**." And in Vault, "dynamic library loading is not acceptable for security reasons."
  Source: [hashicorp/go-plugin README](https://github.com/hashicorp/go-plugin).
- **Cloudflare's counter-measurement.** For their request-shaped workload, "when using strict
  process isolation in Workers, the CPU cost can easily be **10x** what it is with a shared
  process"; thousands of tenants per machine "rapidly switch between these guests thousands of
  times per second" is infeasible per-process. Cloudflare still **drops to a separate process**
  for elevated-risk features (e.g. a Worker under the devtools inspector).
  Source: [Workers security model](https://developers.cloudflare.com/workers/reference/security-model/).
- **The IPC trap is payload size, not round-trips.** Figma measured browser message-passing at
  "on the order of **0.1ms per round-trip** … ~1000 messages per second" (fine), but serializing a
  copy of a large document to the sandboxed plugin took **14 seconds** for Microsoft's design-system
  file "before the plugin could even run" — which killed their iframe sidecar-style design.
  Source: [How we built the Figma plugin system](https://www.figma.com/blog/how-we-built-the-figma-plugin-system/).
- **Outcome pattern:** the canonical choice when plugins are (a) big, self-contained binaries and
  (b) call patterns are coarse-grained. Fails when payloads are large graph snapshots (serialization
  blowup) or when the sidecar language requires a per-machine runtime (breaks single-binary hosts).

### C. In-process memory-sandboxed VM (WASM / isolates) — the industry convergence point

- **Wasmtime** (embedded in-process by hosts like Extism/SpecForge today). Spec-level sandbox:
  inaccessible callstack, pointers as offsets into bounds-checked linear memory, typed control
  flow, "All interaction with the outside world is done through imports and exports … no raw
  access to system calls", no undefined behavior. Defense-in-depth because "bugs or issues
  inevitably arise": 2GB guard regions, guard pages, instance memory zeroing, safe Rust API,
  CFI work in progress. WASI filesystem access is capability-based. Spectre: partial mitigations,
  explicitly "ongoing research". Sources: [Wasmtime security](https://docs.wasmtime.dev/security.html),
  [Bytecode Alliance: Security and correctness](https://bytecodealliance.org/articles/security-and-correctness-in-wasmtime).
- **Cloudflare Workers (V8 isolates)** — in-process sandboxing at maximum scale: "run many isolates
  within a single process … essential for an edge compute platform"; plus layered defense that
  *assumes the VM will leak*: frozen `Date.now()` (no local timing), no multi-threading, trust-tiered
  "cordons" (Free-plan customers never share a process with Enterprise), an outer namespaces+seccomp
  sandbox with an entirely empty filesystem and all I/O mediated over UNIX sockets by a supervisor
  process, and a V8 **patch gap under 24 hours**. Notably: "the V8 team at Google has stated that
  V8 itself cannot defend against Spectre" ([arXiv:1902.05178](https://arxiv.org/abs/1902.05178)).
  Source: [Workers security model](https://developers.cloudflare.com/workers/reference/security-model/).
- **Shopify Functions** — untrusted third-party WASM running in the checkout hot path. Choices that
  mirror SpecForge's needs: input narrowed by a GraphQL query the host executes (not "whole world to
  plugin"); a **structured binary ABI** (64-bit NaN-boxed values, lazy reads) that "eliminates the
  overhead of embedding a JSON parser into your compiled binary"; modules are multi-call executables
  with exports mapped to targets; Rust officially recommended because it is "the most performant
  language choice to **avoid your function failing with large carts**" — WASM CPU overhead is real
  and managed by ABI + limits, not wished away.
  Sources: [About Shopify Functions](https://shopify.dev/docs/apps/build/functions),
  [WebAssembly for Functions](https://shopify.dev/docs/apps/build/functions/programming-languages/webassembly-for-functions).
- **Figma (final state)** — QuickJS-in-WASM on the main thread, plugin↔host API behind an audited
  ~500-LOC "membrane" that passes only handles/primitives across the boundary; kept precisely
  because the 2019 incident proved the swap path was needed.
  Sources: both Figma posts above.
- **Outcome pattern:** isolation comparable to a process (memory boundary) without the per-plugin
  process; the recurring costs are engine weight (wasmtime dependency tree — see evidence.md,
  C7 cluster), guest CPU overhead, and the discipline of a fast engine patch pipeline.

## 3. Cross-cutting findings

1. **The tradeoff axis is not "fast vs safe" — it is *where the memory boundary sits*.**
   Sidecars and WASM/isolates both draw a hardware-or-engine-level memory boundary; embedded
   interpreters without one have a documented escape record (Figma Realms, Redis Lua).
2. **IPC overhead is bimodal:** fine-grained calls are survivable (~0.1 ms/msg in Figma's browser
   case; HashiCorp ships gRPC-over-local-sockets to millions of machines), but *large-payload*
   crossing is the killer (14 s to copy one document). Workloads that pass whole-graph snapshots —
   SpecForge's compiler passes over ~1.7k-entity graphs — sit on the wrong side of that line.
3. **Process isolation costs an order of magnitude for request-shaped fan-out** (Cloudflare's 10x
   CPU figure), which is why high-fan-out platforms (Cloudflare, Figma, Shopify) all converged on
   in-process memory-sandboxed VMs and added *outer* OS sandboxes as defense-in-depth instead of
   using processes as the primary boundary.
4. **In-process sandboxing is a commitment, not a switch.** The working deployments pair the VM
   with: capability-scoped imports only (Wasmtime/WASI, Shopify ABI), no ambient timers/threads
   (Cloudflare), trust-tiered scheduling, an outer seccomp/namespace sandbox, and a <24 h engine
   patch pipeline. A bare `wasmtime.call_export` with an allow-by-default fs flag (SpecForge's
   C7-04) is the same anti-pattern Figma/Redis hit — the VM boundary must be the *only* path to
   ambient effects.
5. **Crash isolation differences are smaller than they look for a CLI host.** A sidecar's benefit is
   that a plugin segfault/hang doesn't kill the host; but Extism/wasmtime already converts guest
   faults into `Trap` results the host survives (evidence.md §1.1), and a CLI "analyze" run failing
   loudly on a bad plugin is acceptable UX. Sidesars additionally complicate hot reload (R-5) and
   determinism (R-6) across a serialization boundary.

## 4. Mapping to SpecForge constraints

| Constraint | A. Embedded interpreter | B. Sidecar | C. WASM in-process |
| --- | --- | --- | --- |
| R-1 all plugins equal | OK | OK | OK (builtins already wasm guests) |
| R-2 sandboxable untrusted | **Fail** by incident history (Figma/Redis class) | OK | OK with imports-only + fixed C7-04 |
| R-3 single binary | **Fail** (system Python; mlua OK but C-code crash domain) | **Fail** for language sidecars; OK for self-contained binaries | OK (static engine) |
| R-4 signed registry | OK | OK | OK (blobs, sha256 — already built) |
| R-5 hot reload | OK | OK (restart) | OK (re-instantiate; pairs with fixing C7-08 engine pooling) |
| R-6 deterministic output | Weak (ambient os/io libs; version coupling per Datadog) | OK | Strong (capability imports; no ambient time/env) |
| Perf: graph-snapshot payloads | Best (zero-copy) | **Worst** (serialization blowup, Figma 14 s case) | Good (guest linear-memory copy; Shopify-style structured ABI would fix C7-03) |
| CPU limits (C7-10) | Cooperative hooks only, escapable | Kill/restart process | Robust (fuel/epoch interruption in wasmtime) |
| Weight (criterion 10) | Small | Host small, plugins big | Heavy engine (wasmtime — acknowledged cost) |

**Hybrid escape hatch observed in industry:** keep one runtime (R-1 intact) but allow the *same*
runtime to be scheduled into a child process for elevated-risk cases — Cloudflare moves inspected
Workers to "a separate process with a process-level sandbox"; HashiCorp runs the identical gRPC
protocol whether local or reattached. SpecForge could later add `specforge analyze --isolate`
(spawn the same wasm host in a child) without forgoing the single-runtime model.

## 5. Honest caveats on the WASM path

- Wasmtime itself documents that "bugs or issues inevitably arise" — its defense-in-depth exists
  because sandbox-adjacent CVEs have occurred; safety depends on tracking engine security releases
  (Cloudflare-style patch discipline).
- Spectre-class side channels are *not* fully mitigable in-process (Wasmtime: "ongoing research";
  Google: V8/WASM "cannot defend" alone). For a local dev tool with human-local threat models this
  is a minor concern; it would matter for a shared hosted runner.
- Guest CPU overhead is real: Shopify's "failing with large carts" warning is the same shape as
  SpecForge's per-entity validation over 1.7k entities; mitigations are ABI design (lazy/structured
  reads instead of one giant JSON — directly addresses C7-03) and pooling (C7-08).

## Bottom line

Out-of-process (sidecar) buys real crash/memory isolation but is the wrong primary model for
SpecForge: its cost profile is dominated by large-payload serialization (Figma's 14-second document
copy is exactly the "compiler pass over a graph snapshot" shape), a language sidecar breaks R-3, and
the strongest adopters (Cloudflare) show process isolation costs ~10x for request-shaped fan-out.
A bare embedded interpreter (Lua/Python) is fast but sits on the documented failure path for R-2:
same-address-space sandboxes have a concrete escape record (Figma Realms escapes, Redis
CVE-2022-0543 at CVSS 10), and CPython embedding adds version-coupling pain (Datadog). Every
high-fan-out platform that hosts untrusted code in-process — Cloudflare (V8 isolates), Figma
(QuickJS-in-WASM), Shopify (Wasmtime-class) — converged on the third option: a VM with a
hardware/engine-level memory boundary plus capability-scoped imports, which is what SpecForge
already has; the failures in its own audit (C7-03/C7-04/C7-08/C7-10) are all fixable *inside* that
model rather than reasons to change models.

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** The industry record shows the in-process memory-sandboxed VM (wasm) is the
only model that satisfies R-2 and R-3 simultaneously; sidecars pay a payload-serialization tax that
SpecForge's graph-snapshot workload cannot afford, and interpreter-level sandboxes have a proven
escape history.
