# Sandbox Escape CVE Record per Candidate Runtime

**Analyst:** research-sandbox-cves · **Date:** 2026-09-27 · **Feeds:** OSV API, NVD API 2.0, CISA KEV, vendor advisory pages (all queried live on this date)

**Question answered:** How many real sandbox escapes / security incidents does each candidate runtime (wasm/Wasmtime, Lua, Python, JS: V8 & QuickJS) carry, and which has the best track record for executing untrusted code under R-2?

## Method and honesty notes

- Full advisory lists pulled from the **OSV API** (`crates.io/wasmtime`, `crates.io/wasmer`, `crates.io/mlua`, `crates.io/deno_core`, `PyPI/jinja2`) and **NVD API 2.0** (keyword/CPE/CVE queries). Counts below are from those API responses, not from blog posts.
- Cross-ecosystem counts are **not perfectly comparable**: Chrome files a CVE for nearly every security fix; the Lua and Python upstreams CVE almost nothing themselves (third parties file); Wasmtime treats host panics/DoS as security advisories while Lua's project does not. Directional conclusions are robust; raw totals are not.
- Claims I checked and **discarded** because they were wrong: `CVE-2021-44967` is LimeSurvey (not the Lua interpreter); `CVE-2019-10193` is Redis hyperloglog (not the Lua sandbox); CISA KEV contains **zero** of the 18 famous exploited V8/Redis CVEs I checked (KEV skews to enterprise/edge software — absence from KEV ≠ not exploited).
- Could not verify (search rate-limited): Project Zero's 2024/2025 in-the-wild 0-day aggregate; WAMR CVE count (exact-phrase NVD query returned 0 — inconclusive, not evidence of absence). Neither is load-bearing here.

## 1. WASM — Wasmtime (SpecForge's current engine, 43.0.2)

**Total: 39 unique GitHub security advisories / 39 unique CVEs, 2021 → Sep 2026** (OSV, deduped across GHSA/RUSTSEC/PYSEC aliases). By year: 2021: 3 · 2022: 7 · 2023: 4 · 2024: 5 · 2025: 3 · 2026: 17 (through Sep 27).

**Guest→host memory-isolation escapes (the class that matters): 6 advisories**

| CVE(s) | Year | What it was | Severity |
| --- | --- | --- | --- |
| CVE-2021-39216/39218/39219 | 2021 | OOB read/write + invalid free via `externref`s and GC safepoints | High |
| CVE-2022-23636 + CVE-2022-31169 | 2022 | Cranelift miscompilation of constant division on AArch64 | High |
| CVE-2022-39392 | 2022 | OOB read/write with zero-memory-pages configuration | 7.4 |
| **CVE-2023-26489** | 2023 | Guest-controlled OOB read/write on x86_64 — worst advisory in project history | **9.4 Critical** |
| **CVE-2026-34971** | 2026 | Cranelift miscompiled guest heap access → sandbox escape on aarch64 | **9.0 Critical** |
| **CVE-2026-34987** | 2026 | Winch backend sandbox-escaping memory access (aarch64) | **9.0 Critical** |

**Partial escapes / cross-instance leaks (3):** CVE-2022-39393 (pooling-allocator data leak between instances), CVE-2026-34945 (host data leakage, Winch 64-bit tables), CVE-2026-34988 (pooling-allocator leak, low). **Host-side OOB read:** CVE-2026-34941 (component-model string transcoding, 6.9). **Capability-layer (WASI fs) escape:** GHSA-vqjp-4c8c-hfgg (High, 2026 — fs sandbox escape via trailing slashes/symlinks). Sibling engine **Wasmer**: 1 advisory ever — CVE-2024-38358, symlink filesystem-sandbox bypass.

Everything else (≈28 advisories) is host panic / DoS / fuel-accounting — annoying, not escapes.

**What the record shows:**

- Escapes are **JIT-compiler miscompilations and `unsafe` Rust bugs**, found by fuzzing, formal verification, and — in April 2026 — an LLM-driven audit that produced **12 advisories in one day** ("triple the total number of advisories published in all of 2025, doubles the number of Critical severity advisories published in the project's history" — Bytecode Alliance). Assume the higher 2026 discovery rate is the new normal, not a fluke.
- Both 2026 Critical escapes are on **aarch64 / Winch** paths; the Alliance states Cranelift-on-x86_64 users were unaffected by both. SpecForge targets macOS arm64 + Linux x64 — **one of the two Criticals (CVE-2026-34971) is on the arm64 Cranelift path**; SpecForge's locked wasmtime 43.0.2 (patched Apr 9, 2026: releases 43.0.1/42.0.2/36.0.7/24.0.7) is not affected by that batch. The Sep 24, 2026 batch (4 Moderate/Low DoS-class advisories) landed after; verify 43.0.2 covers it.
- **Zero Wasmtime escapes are reported exploited in the wild.** Discovery is coordinated: private report → patch release → advisory (documented process; Miri over `unsafe`, `cargo vet`, continuous fuzzing, formally verified Cranelift lowering — with the honest caveat that the formal model missed a 32.0.0-lowered bug and there is no continuous aarch64 fuzzing).
- Two independent real-world wasm engines in browsers show wasm itself is not bug-free: **CVE-2024-2887 is a Pwn2Own type confusion *in WebAssembly* in Chrome**, exploited at Pwn2Own 2024 (ZDI).

## 2. Lua (PUC-Lua 5.4 / LuaJIT via `mlua`)

**Interpreter (ISO C, in-process):** PUC-Lua has ~8 NVD CVEs 2014–2022 (found via source-file mentions in NVD descriptions: `ldebug.c`, `lapi.c`, `ldo.c`, `lparser.c`): CVE-2014-5461 (buffer overflow, `ldo.c`), CVE-2019-6706 (use-after-free, `lapi.c`), CVE-2020-15945/24369/24370 (SEGV/NULL-deref/overflow, `ldebug.c`), CVE-2021-43519 (stack overflow in `lua_resume`, DoS, all 5.1–5.4.4), CVE-2021-44647 (type confusion), CVE-2022-28805 (parser, uninitialized memory). **LuaJIT: 10 NVD CVEs** (CVE-2019-19391, CVE-2020-15890, CVE-2020-24372, CVE-2024-25176/77/78, CVE-2024-39702, CVE-2026-34444, CVE-2026-40959, CVE-2026-41196). These are crashes/memory-safety bugs **inside the host's own process** — an embedder inherits them with no second layer.

**Real Lua-sandbox escape incidents in production (the strongest data here — Redis runs untrusted-ish Lua in a stripped sandbox at scale):**

- **CVE-2022-24834**: "A specially crafted Lua script executing in Redis can trigger a heap overflow in the cjson library … heap corruption and potentially **remote code execution**" (NVD).
- **CVE-2024-31449**: "An authenticated user may use a specially crafted Lua script to trigger a **stack buffer overflow in the bit library, which may potentially lead to remote code execution**" (NVD).

Pattern: the *interpreter environment* is stripped, but the sandbox boundary is the C modules bound into it; C-module memory corruption = escape to native code in-process. This is exactly the class a `mlua`-based SpecForge sandbox would own: every host-exposed function (`cjson`-equivalents: the graph/registry APIs) becomes the attack surface.

**`mlua` specifics (verified on docs.rs):** `Lua::sandbox()` exists **only behind the `luau` feature** — i.e., for Roblox's Luau dialect, not Lua 5.4/LuaJIT. Vanilla-Lua sandboxing in mlua means hand-stripping the stdlib; and Luau's `sandbox()` itself only makes globals/libraries read-only — it is *environment* hygiene, not memory isolation. `mlua` has 0 security advisories on crates.io — meaning no audited sandbox mechanism to lean on, not a clean record.

## 3. Python (CPython via PyO3)

**There is no sandbox to escape — Python abandoned the concept in 2003.** Python 2.3 release notes: "The `rexec` and `Bastion` modules have been declared dead … New-style classes provide new ways to break out of the restricted execution environment provided by `rexec`, and no one has interest in fixing them or time to do so." Every CPython embedding since (PyO3 included) runs guest code with **full process privileges**; R-2 (no ambient fs/net/process) is unsatisfiable in-process by construction. `crates.io/cpython` carries RUSTSEC-2023-0076; PyO3 itself has none — again, no sandbox mechanism exists to audit.

**The closest thing to a managed Python sandbox — Jinja2's SandboxedEnvironment — has a breakout record:** **5 sandbox-escape CVEs in 9 years**: CVE-2016-10745, CVE-2019-10906 (both `str.format` escapes), CVE-2024-56201 (malicious filenames), CVE-2024-56326 (indirect `format` reference), CVE-2025-27516 (`attr` filter selecting `format`). Three of the five landed within 16 months (2024–2025) — the sandbox is in a permanent cat-and-mouse with the language's introspection power, which is the fundamental problem with escaping-proofing Python.

Real-world incident class: every "embedded Python runs untrusted code" system (sandboxed CI runners, notebook services) is documented to need OS-level containers precisely because CPython offers nothing in-process.

## 4. TypeScript/JS (V8 via `deno_core`; QuickJS)

**V8 — the highest-volume attack surface of any candidate by an order of magnitude.**

- NVD records with both "V8" and "Chrome/Chromium" in the description: **~443 CVEs since 2009**; recent years: 2021: 35 · 2022: 22 · 2023: 18 · 2024: 57 · 2025: 45 · 2026 (partial): 118. (NVD API pull, 2026-09-27.)
- **In-the-wild exploitation is routine.** Google's V8 team: "all Chrome exploits caught in the wild in the last three years (2021–2023) started out with a memory corruption vulnerability in a Chrome renderer process that was exploited for RCE. Of these, **60% were vulnerabilities in V8**" (v8.dev/blog/sandbox). Named ITW examples confirmed in NVD text (each "execute arbitrary code" / "arbitrary read/write"): CVE-2023-3079, CVE-2023-4762, CVE-2024-2887, CVE-2024-4947, CVE-2024-5274, CVE-2025-0291, CVE-2025-6554 (ITW per Google; Theori published a deep-dive of its full chain, which includes a **V8 sandbox bypass**).
- V8's own in-process sandbox — the defense an embedder would rely on — is explicitly **not done**: "there are still a number of issues to resolve before it becomes a strong security boundary" (v8.dev, Apr 2024; VRP-eligible but beta). It needs a build-time flag (`v8_enable_sandbox`, 64-bit only, ~1 TB VA space); bypasses are an active bounty category.
- **Deno's permission model — the exact mechanism SpecForge would lean on for R-2 under `deno_core` — has its own chronic bypass record**: a single batch on May 27, 2026 published **8+ advisories**: `fetch()` sandbox bypass via missing DNS check, WebSocket sandbox bypass via missing post-DNS check, APFS Unicode-normalization permission bypass (High), `--allow-read` bypass via `package.json` path traversal, `process.loadEnvFile()` bypassing env checks, command injection via `spawnSync` on Windows (High), plus TLS and crypto flaws. `deno_core` itself: 0 crate advisories — Deno's advisories live on the deno repo and largely *mirror V8 CVEs* for embedders.

**QuickJS/quickjs-ng:** **28 NVD CVEs 2020–2026** (API pull). Classification of all 28: essentially all are memory-safety bugs in the C engine — buffer/stack overflows, UAF (CVE-2023-48184, CVE-2025-62491), type confusion (CVE-2025-62494), OOB write (CVE-2026-88378, CVE-2025-46687/88), and **CVE-2026-37630: "allows an attacker to execute arbitrary code" in quickjs-ng 0.12.1**. Real embedding incident: **CVE-2026-63762 — SurrealDB's embedded QuickJS DoS** (fixed in SurrealDB 2.6.1/3.0.0-beta.3). QuickJS is often praised as "small = safe"; its record is a steady drumbeat of exploitable C bugs at ~5/year and accelerating (13 of the 28 are 2025–2026).

## 5. Aggregate comparison

| Runtime (engine) | Advisories/CVEs (window) | Memory-sandbox escapes | Worst single CVE | Exploited in the wild? | Isolation layers for guest code |
| --- | --- | --- | --- | --- | --- |
| **Wasmtime** (current) | 39 (2021–26), ~7/yr | **6 advisories** (+1 fs-capability escape) | 9.4 (2023) | **None reported** | 2: linear-memory confinement by construction + engine memory safety (Rust) |
| Wasmer | 1 (2024) | 0 memory; 1 fs-sandbox bypass | — | — | 2 |
| **Lua PUC 5.4** | ~8 CVEs (2014–22) | interpreter bugs land in-process; sandbox escapes via bound C modules (Redis: CVE-2022-24834, CVE-2024-31449 → RCE) | RCE (Redis) | Redis escapes used against authenticated tenants | 1 (env stripping; `mlua` `sandbox()` is Luau-only) |
| LuaJIT | 10 CVEs (2019–26) | same | — | — | 1 |
| **CPython** | n/a — sandbox abandoned 2003; Jinja2 sandbox: **5 breakout CVEs (2016–25)** | n/a | RCE via escapes | — | 0 in-process (OS container required) |
| **V8** (deno_core) | ~443 NVD CVEs w/ Chrome+V8 (2009–26); 45–57+/yr recent | RCE is the *normal* finding; V8 sandbox beta w/ active bypass bounty | many Critical | **Yes, repeatedly — 60% of ITW Chrome renderer RCEs 2021–23** | 1 in-process (+ beta V8 sandbox; + Deno perms with 8-bypass month) |
| **QuickJS** | 28 (2020–26) | n/a (in-process) | arbitrary code exec (CVE-2026-37630) | — | 1 |

## 6. What this means for R-2 (sandboxable untrusted plugins)

1. **Only WASM gives defense-in-depth.** Guest code is confined to a 4 GiB linear memory with guard regions by *construction*; every host touchpoint is an explicit import. An engine bug is *required* to escape. For the interpreter runtimes (Lua/Python/JS), guest code executes natively in the host process, and the "sandbox" is policy over the interpreter environment — a memory-safety bug in the interpreter or any bound C/host module is immediate native code execution with no second layer. That is not a hypothetical: it is the Redis CVE-2022-24834/CVE-2024-31449 pattern, the SurrealDB embedded-QuickJS pattern, and the V8 60%-of-ITW pattern.
2. **Wasmtime's escape rate is an order of magnitude lower and qualitatively tamer.** 6 memory escapes in ~5.5 years, none exploited in the wild, coordinated disclosure, Rust memory safety, formal verification of the JIT (with admitted gaps). V8 alone: dozens per year, exploited for money on a weekly cadence. QuickJS: exploitable C bugs at a rising rate with a 1-person-scale project. Lua/CPython: small CVE counts partly because nobody audits them the way Chrome is audited — absence of advisories is not absence of bugs (PUC-Lua's CVEs surface via third-party embedders like REFramework and NotepadNext).
3. **The capability layer is the chronic class for everyone — including the current codebase.** The real fs/network sandbox escapes in the wasm world are permission-layer: Wasmtime GHSA-vqjp-4c8c-hfgg (trailing-slash/symlink fs escape, 2026), Wasmer CVE-2024-38358 (symlink), Deno's 8-advisory bypass month (May 2026). SpecForge's own audit found the same genus locally: **C7-04 — sandbox `file_system_access` is allow-by-default**. Keeping WASM does not fix this; it only means the engine underneath the policy is sound. C7-04 must be fixed deny-by-default under any runtime, and under any non-wasm runtime there is *nothing underneath it*.
4. **2026 recalibration, honestly stated:** the April 2026 LLM-audit batch shows Wasmtime's advisory volume can triple overnight when someone actually hunts. Two Critical escapes landed in the same year — both off the default x86_64-Cranelift path, but one (aarch64 Cranelift) directly on SpecForge's macOS arm64 target. The correct reading is not "wasmtime is fragile"; it is "wasm escapes get found by audits and fixed in coordinated releases, and you must track wasmtime security releases as an operational commitment" — versus runtimes where the question is not *whether* guest-triggerable engine RCE exists but *how often it is already being used*.

## Sources

- OSV API queries (2026-09-27): `crates.io/wasmtime` (39 GHSAs/CVEs), `crates.io/wasmer`, `crates.io/mlua` (0), `crates.io/deno_core` (0), `PyPI/jinja2` (20 records); per-advisory summaries via `api.osv.dev/v1/vulns/<GHSA|PYSEC>`
- NVD API 2.0 (2026-09-27): keyword `quickjs` (28), keyword `LuaJIT` (10), Lua internals descriptions, keyword `v8` filtered to Chrome/Chromium descriptions (~443), cveId pulls for CVE-2022-24834, CVE-2024-31449, CVE-2025-6554, CVE-2024-2887, CVE-2021-44967 (negative), CVE-2019-10193 (negative)
- CISA KEV catalog (2026-09-27): 1,726 entries; 6 Google Chrome; 0 of 18 checked V8/Redis CVEs present
- [Wasmtime security advisories](https://github.com/bytecodealliance/wasmtime/security/advisories) (incl. Sep 2026 batch); [Bytecode Alliance, "Wasmtime's April 9, 2026 Security Advisories"](https://bytecodealliance.org/articles/wasmtime-security-advisories) (12-advisory batch, CVSS table, x86_64-Cranelift unaffected statement, aarch64 fuzzing gap)
- [Wasmer CVE-2024-38358 (OSV/GHSA-55f3-3qvg-8pv5)](https://github.com/wasmerio/wasmer/security/advisories/GHSA-55f3-3qvg-8pv5)
- [mlua `Lua::sandbox` — Luau-only](https://docs.rs/mlua/latest/mlua/struct.Lua.html)
- [Python 2.3 What's New — rexec/Bastion declared dead](https://docs.python.org/3/whatsnew/2.3.html)
- Jinja2 sandbox escapes: [CVE-2025-27516](https://github.com/pallets/jinja/security/advisories/GHSA-cpwx-vrp4-4pq7), CVE-2024-56201, CVE-2024-56326, CVE-2016-10745, CVE-2019-10906 (OSV records)
- [V8 blog: The V8 Sandbox](https://v8.dev/blog/sandbox) (60% ITW stat; "not yet a strong security boundary"; VRP inclusion)
- [Theori: V8 sandbox escape in the CVE-2025-6554 in-the-wild exploit](https://theori.io/blog/a-deep-dive-into-v8-sandbox-escape-technique-used-in-in-the-wild-exploit); [ZDI: CVE-2024-2887 Pwn2Own WebAssembly type confusion](https://www.thezdi.com/blog/2024/5/2/cve-2024-2887-a-pwn2own-winning-bug-in-google-chrome)
- [Deno security advisories](https://github.com/denoland/deno/security/advisories) (May 27, 2026 permission-bypass batch)
- SurrealDB embedded QuickJS: CVE-2026-63762 (NVD)

## Bottom line

Wasmtime's measured record — 39 advisories over 5.5 years, 6 memory-isolation escapes, 0 exploited in the wild, coordinated patch process, memory-safe implementation with a formally verified JIT — is the best security track record for untrusted code execution among the candidates by a wide margin. V8 is the most attacked and most frequently exploited engine in this comparison (60% of ITW Chrome RCE chains; 45–57 CVEs/yr; its own sandbox still not a strong boundary; Deno's permission layer shows recurring bypass batches). QuickJS carries exploitable C-engine CVEs including arbitrary-code-exec, with no isolation layer beneath it. Lua's interpreter is comparatively quiet but sandbox escape in practice happens through bound C modules (two Redis RCE CVEs), and `mlua` offers no sandbox mechanism at all for Lua 5.4/LuaJIT (`sandbox()` is Luau-only). Python has no sandbox to break — it was formally abandoned upstream in 2003, and the most-used Python sandbox (Jinja2's) accumulates breakout CVEs roughly annually. On this single criterion, WASM (Wasmtime) is the only candidate whose containment does not depend on the absence of bugs in a C/C++ interpreter running in the host's own process.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** Measured CVE records make Wasmtime the only candidate with structural two-layer containment and zero in-the-wild escapes, while V8/QuickJS/Lua/Python all put untrusted code natively in-process with engine bugs ≡ host RCE; the one shared chronic failure class (capability-layer escapes, cf. C7-04) must be fixed deny-by-default regardless of engine.

