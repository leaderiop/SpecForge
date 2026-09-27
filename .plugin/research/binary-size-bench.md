# Binary Size & Memory Overhead — Embedding Each Plugin Runtime in a Rust Binary

Research dimension: **evaluation criteria 4 & 10** (distribution under R-3, binary size / dependency weight).
Method: first-party measurements (cargo-built hello-world embeddings) cross-checked against public release artifacts and Docker Hub registry data. All local numbers measured 2026-09-27.

## Method

- Host: Apple M3 Pro, macOS (Darwin 25.6.0) **arm64**, rustc 1.98.1, cargo release profile with `strip = true`, **default crate features** (no size tuning), one minimal "initialize runtime → execute hello-world" binary per runtime.
- Binary size: `stat` bytes of the linked executable. Memory: peak RSS via `/usr/bin/time -l` (median of 3 runs), which includes a process baseline.
- Baseline: identical `println!` binary with zero deps. Deltas below are (runtime − baseline).
- Every binary was actually run and observed to execute its script/program successfully.
- Versions: wasmtime/wasmtime-wasi **43.0.2** (the exact version SpecForge pins via extism 1.30.0), mlua **0.12.1** (`lua54`,`vendored`), pyo3 **0.29.2** (`auto-initialize`, dynamically linked to CPython **3.12.11** python-build-standalone via uv), rquickjs **0.14.0** (bundles **quickjs-ng**; confirmed in `rquickjs-sys-0.14.0` vendored sources), deno_core **0.412.0** (V8 static).

## Measured: macOS arm64 (hello-world embedding)

| Runtime | Binary (bytes) | Δ binary vs base | Peak RSS (bytes) | Δ RSS vs base |
| --- | --- | --- | --- | --- |
| bare Rust (baseline) | 342,000 | — | 1,540,096 | — |
| **mlua** (Lua 5.4, vendored static) | 723,968 | **+0.38 MB** (2.1×) | 2,031,616 | **+0.49 MB** |
| **rquickjs** (quickjs-ng, vendored static) | 1,377,280 | **+1.03 MB** (4.0×) | 2,539,520 | **+1.0 MB** |
| **wasmtime + wasi** (cranelift JIT, default feats) | 13,275,296 | **+12.9 MB** (38.8×) | 14,548,992 | **+13.0 MB** |
| **deno_core** (V8, static) | 44,892,768 | **+44.6 MB** (131×) | 19,120,128 | **+17.6 MB** |
| **pyo3** (dynamic libpython) | 381,584 | **+0.04 MB** (1.1×) | 17,973,248 | **+16.4 MB** |

Reading the pyo3 row correctly: the *executable* barely grows because CPython is a shared library — the real cost is the payload you must ship next to it (below) plus per-process RSS. pyo3 RSS was measured after `Py_Initialize` + executing a statement with the stdlib path configured (imports `encodings`; no site-packages).

**Measured failure mode (pyo3 embedding):** without pointing the process at a Python home, `Py_Initialize` dies with `Fatal Python error: init_fs_encoding: failed to get the Python codec of the filesystem encoding` — the embedding only ran after `PYTHONHOME` was set to the interpreter prefix. This is the classic "embed+ship CPython is fragile" tax, now demonstrated first-hand.

### What must ship alongside the host binary (R-3 distribution payload)

| Runtime | Extra files required at runtime |
| --- | --- |
| mlua | **none** (Lua source compiled into the binary) |
| rquickjs | **none** (quickjs-ng compiled in) |
| wasmtime | **none** (guest `.wasm` blobs are `include_bytes!`-able) |
| deno_core | **none** (V8 static lib linked in) |
| pyo3 | **libpython3.12.dylib = 19,406,928 B (19.4 MB)** + **stdlib = 27.1 MB** (uv `cpython-3.12.11-macos-aarch64-none` measured: 53.3 MB total install incl. exe/test dirs) **+ correct `PYTHONHOME`/path config or init aborts** |

## Public artifact sizes (cross-check, all fetched 2026-09-27)

| Artifact | Size | Source |
| --- | --- | --- |
| Wasmtime CLI v49.0.1, aarch64-macos `.tar.xz` | 9,566,152 B compressed; **59,980,400 B unzipped CLI** (measured after download) | wasmtime GitHub release |
| Wasmtime CLI v49.0.1, x86_64-linux `.tar.xz` | 11,615,796 B compressed | wasmtime GitHub release |
| Wasmtime "minimal embedding" ladder (libwasmtime.so, Linux x64, 2024-12): release 19 MB → no-default-features **2.1 MB** → +LTO **1.2 MB** → nightly+std rebuild extreme **698 KB**; docs note WASI impl ≈ **1 MB+** and clap ≈ **200 KB** of the CLI | — | docs.wasmtime.dev examples-minimal |
| Deno CLI v2.9.7, aarch64-apple-darwin `.zip` | 38,469,316 B compressed; **80,982,000 B unzipped CLI** (measured); denort (runtime-only) 28.7/34.4 MB zipped | denoland/deno GitHub release |
| quickjs-ng v0.17.0 `qjs` release binaries | darwin-arm64 **1,320,208 B**; linux-x64 **2,562,504 B**; linux-aarch64 2,546,520 B | quickjs-ng GitHub release |
| python-build-standalone 20260924, cpython-3.12.14 `install_only.tar.gz` | aarch64-apple-darwin **25,153,879 B**; x86_64-unknown-linux-gnu **66,890,910 B** compressed (≈50–110 MB+ installed, consistent with local 53.3 MB) | astral-sh/python-build-standalone GitHub release |
| Docker Hub `full_size` (compressed pull size) | `python:3.12` = **411,054,586 B**; `python:3.12-slim` = **43,241,091 B**; `denoland/deno:latest` = **72,530,362 B** | Docker Hub registry API |

## Interpretation for SpecForge

- **Ranking on binary size:** mlua (+0.4 MB) < quickjs-ng (+1.0 MB) ≪ wasmtime (+12.9 MB) < deno_core (+44.6 MB). pyo3's binary delta is meaningless without its ~46 MB interpreter payload + path configuration, making it effectively the heaviest to distribute as a single self-contained binary (R-3).
- **Ranking on per-process memory (idle hello-world):** mlua (+0.5 MB) < quickjs-ng (+1.0 MB) ≪ wasmtime (+13.0 MB) < CPython (+16.4 MB) ≈ V8 (+17.6 MB). Real workloads grow these (V8 heap, CPython imports, wasmtime per-instance memories), but the idle floor ordering is decisive for a CLI/LSP/MCP host that spawns plugin calls frequently.
- **wasmtime cost is tunable but floored:** the official size ladder shows default-feature embeddings ~13–19 MB shrinking to ~1.2 MB only by dropping the compiler (precompiled `.cwasm` only), disabling WASI/logging, LTO, and nightly std rebuilds — each trade cutting against SpecForge's plugin-compile workflow (and C7-02's AOT story). A realistic kept-JIT embedding stays ≈8–15 MB.
- **Context vs current repo:** the four vendored guest blobs total 1.44 MB (324–416 KB each) and the wasmtime host pull is the audit's noted dependency-weight driver (locked deps 485→511). A mlua/rquickjs host would ship the runtime for *less than one current wasm blob* and delete the cranelift dep tree; a deno_core host would add ~45 MB to every distributed binary.
- **Caveats:** one host OS/arch (macOS arm64); Linux x64 expected in the same order of magnitude (release-asset cross-checks agree: quickjs-ng linux-x64 2.56 MB, wasmtime linux tarball 11.6 MB, deno linux zip 41.6 MB). Default features favor correctness over size; size-tuned builds can shrink wasmtime/V8 somewhat but not across the 30–100× gaps to Lua/QuickJS. RSS floor ≠ loaded-workload RSS.

## Sources

- Local builds: `/tmp/plugin-size-bench/{base,mlua,pyo3,rquickjs,wasmtime43,deno-core}` (rustc 1.98.1, crates.io versions above); every binary executed and output observed.
- Wasmtime release assets: https://github.com/bytecodealliance/wasmtime/releases/tag/v49.0.1
- Wasmtime minimal-embedding size ladder: https://docs.wasmtime.dev/examples-minimal.html
- Deno release assets: https://github.com/denoland/deno/releases/tag/v2.9.7
- quickjs-ng release assets: https://github.com/quickjs-ng/quickjs/releases (v0.17.0)
- python-build-standalone: https://github.com/astral-sh/python-build-standalone/releases/tag/20260924
- Docker Hub tag API: https://hub.docker.com/v2/repositories/library/python/tags/ , https://hub.docker.com/v2/repositories/denoland/deno/tags/
- Repo facts: `.plugin/evidence.md` (blobs 1.4 MB total; wasmtime 43.0.2; deps 485→511).

## Verdict

**Verdict:** LUA
**Confidence:** 4
**One-line rationale:** Measured on this codebase's exact wasmtime version, Lua 5.4 via mlua is ~32× smaller in binary weight (+0.4 MB vs +12.9 MB) and ~26× lighter in idle memory (+0.5 MB vs +13 MB) than the WASM path while shipping zero extra files; Python and V8 are the heaviest to distribute (≈46 MB payload / 45 MB binary respectively).

## Bottom line

**Verdict:** LUA (on the binary-size & memory criterion specifically — full decision must weigh sandboxing R-2, where wasmtime is strongest and Lua is interpreter-level).
**Confidence:** 4/5 — all headline numbers are first-party measurements of real, executed embeddings plus public release artifacts; only Linux x64 relies on cross-checks rather than a local build.
**Key numbers:** mlua +0.38 MB binary / +0.49 MB RSS · rquickjs +1.03 MB / +1.0 MB · wasmtime 43 +12.9 MB / +13.0 MB · deno_core (V8) +44.6 MB / +17.6 MB · pyo3 +0.04 MB binary but +19.4 MB libpython +27.1 MB stdlib payload and a measured `PYTHONHOME` init failure mode.
