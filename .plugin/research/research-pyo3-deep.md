# PyO3 Deep Research — Embedding CPython in Rust

Fleet researcher: `research-pyo3-deep` · Date: 2026-09-27 · Scope: maturity, GIL in multi-threaded hosts, packaging (ship CPython in a binary?), startup, memory, what Polars/pydantic-core/cryptography actually do, failure modes.

**Method note.** The `web_search` provider was erroring for this session; per Main's directive I substituted direct reads of primary sources (GitHub API/raw files, crates.io mirrors, pyo3.rs, docs.python.org, py-free-threading.github.io) **plus fresh local benchmarks on this machine** (macOS arm64, Apple M3 Pro, CPython 3.14.7 via Homebrew). All numbers below are either cited or measured; a correction to D03's sub-interpreter claim is flagged in §6.

## 1. Maturity and adoption

PyO3 is the most mature Rust↔Python bridge by a wide margin — but its maturity is almost entirely in the **extension-module direction** (Rust compiled into a Python wheel), not embedding:

- **16,179 stars / 1,024 forks / 402 open issues** on GitHub (github.com/PyO3/pyo3, fetched 2026-09-27).
- **20,594,977 downloads/month** on crates.io; **used in 2,428 crates** (2,257 directly); **100 releases since 0.1.0 (2017-07-23)**; current **0.29.2 (2026-08-05)**; 55k SLoC; #3 crate in the FFI category (lib.rs/crates/pyo3).
- Release cadence is roughly quarterly and current: 0.23.0 2024-11-15 → 0.26.0 2025-08-29 → 0.27.0 2025-10-19 → 0.28.0 2026-02-01 → 0.29.0 2026-06-11 (CHANGELOG.md, github.com/PyO3/pyo3).
- Supports CPython 3.9+, PyPy 7.3 (3.11+), GraalPy 25.0+; requires Rust 1.83+ (github.com/PyO3/pyo3 README).
- **No published RustSec advisories**: `https://rustsec.org/packages/pyo3.htm` returns 404 (that path exists only for packages with advisories; checked 2026-09-27). I found no CVE history for PyO3 itself.

The embedding side of the API is real but small and comparatively niche: `auto-initialize` feature, `Python::initialize`, `Python::attach`/`detach`. The README's own framing is telling: extension modules get a rich guide (maturin, abi3, free-threading); embedding gets "you need a Python shared library — `sudo apt install python3-dev`" (github.com/PyO3/pyo3 README, "Using Python from Rust").

## 2. GIL handling in multi-threaded apps

For a GIL-enabled interpreter (what every distro ships):

- Every Rust worker thread must `Python::attach` to touch Python; in the GIL build that is the old GIL acquisition — **all "parallel" plugin calls serialize on one global lock**. PyO3's free-threading guide states the GIL "serializes access to the Python runtime … a fundamental limitation to parallel scaling" (pyo3.rs/main/free-threading).
- **Detach discipline or deadlock**: the guide requires `Python::detach` for long-running native work and warns that failing to detach causes "hangs and deadlocks" — free-threaded builds trigger global GC synchronization events that a still-attached thread blocks (pyo3.rs/main/free-threading, "Detaching to avoid hangs and deadlocks"). Rust locks held across `attach` are the classic deadlock shape.
- **Shutdown is a hazard**: CPython 3.14's own docs say any thread other than the finalizing thread that attaches during interpreter shutdown enters "a permanently blocked state" until process exit — "Gross? Yes." — and can deadlock finalization (docs.python.org/3/c-api/interp-lifecycle.html, "Cautions regarding runtime finalization"). PyO3 mirrors this: since 0.26 `Python::attach` **panics** if the interpreter is shutting down (PR #5317), and 0.27/0.24 fixed shutdown UB into deliberate hangs ("hang instead of `pthread_exit`", PRs #6085, #4874; CHANGELOG.md lines 180, 466, 633).
- **Free-threaded builds** (no GIL): supported by PyO3 since 0.23; since 0.28 PyO3 defaults to assuming its modules are thread-safe (`gil_used` slot, `GILProtected` removed). But concurrency bugs move rather than vanish: mutable `#[pyclass]` access under threads raises `RuntimeError: Already borrowed`, and special `PyOnceLock`/`OnceExt` APIs exist specifically because plain `OnceLock` can deadlock during interpreter-global sync events (pyo3.rs/main/free-threading). Critically, free-threaded CPython 3.13/3.14 has **no stable ABI** (abi3 ignored with a warning; `abi3t` only arrives in 3.15) and remains a non-default build (py-free-threading.github.io/faq, "Py_LIMITED_API is currently incompatible with Py_GIL_DISABLED"; py-free-threading.github.io — "supported" per PEP 779 in 3.14 but "not yet the default interpreter build").

Net: in a multi-threaded Rust host on stock CPython, plugin execution is GIL-serialized; on free-threaded CPython it is parallel but requires the non-default interpreter, no abi3, and manual `Send`/`Sync`/locking audits.

## 3. Packaging: can you ship CPython inside your binary?

**No — not in any maintained, supported way.**

- PyO3 embedding **links a version-specific shared `libpython`** at build time; the README's install step for embedding is `apt install python3-dev` / `python3-devel` (github.com/PyO3/pyo3). The abi3 stable ABI "is an extension-module facility" — irrelevant to embedding. The host binary is born dependent on a matching interpreter on the target machine.
- The one project that set out to do exactly this — **PyOxidizer** ("single file executable, with a copy of Python statically linked and resources embedded") — is effectively dead: 6,154 stars but **last commit 2024-11-03** (github.com/indygreg/PyOxidizer; github.com/indygreg/PyOxidizer/commit/1ceca866), single-maintainer, with 361 open issues. Nothing has filled the "CPython inside a Rust binary" hole since.
- What the ecosystem actually ships is the **opposite**: relocatable CPython *distributions unpacked on disk*. python-build-standalone (astral-sh) latest release `20260924` ships per-platform tarballs of **25.8 MB (macOS aarch64) to 43.9 MB (Linux aarch64)** compressed — download-and-extract payloads managed by uv, not embeddable bytes (api.github.com/repos/astral-sh/python-build-standalone/releases/latest, asset `cpython-3.10.21+20260924-aarch64-apple-darwin-install_only.tar.gz` = 25,832,478 B; `...-aarch64-unknown-linux-gnu-install_only.tar.gz` = 43,926,859 B). And a statically-linked-libpython trick still leaves stdlib extension modules as separate `.so` files that must exist on disk at import time — `include_bytes!`-ing a working CPython is not a thing (structural fact of CPython builds; consistent with .plugin/dimensions/D03-python-embedding.md:31-43).

For SpecForge's R-3 (single-binary host) this is disqualifying: system-`libpython` fragility *or* a ~30–100 MB non-embedded payload.

## 4. Startup time and memory overhead (measured here)

All measured today on this M3 Pro / macOS arm64 / CPython 3.14.7 (Homebrew):

| Measurement | Result |
| --- | --- |
| `python3 -c pass` process spawn+init, 20-run loop | **~49 ms/run** (`-S`: ~45 ms; `import json,re`: ~54 ms) |
| In-process `Py_Initialize()` from C (`/tmp/pyinit.c`, `python3-config --embed`), cold | **13.7–18.1 ms** |
| Warm re-init (same process, after `Py_FinalizeEx`) | **~6.8 ms** |
| `import json, re` after init | +0.7–1.9 ms |
| Max RSS `python3 -c pass` | **15.1 MB** (12.5 MB with `-S`) |
| Max RSS embedded C binary (init + `import json,re`) | **15.4 MB** |
| Max RSS `import ssl, zlib` | **18.1 MB** |

Interpretation: interpreter init alone is tolerable (~7–18 ms) but the **floor cost of hosting Python is ~12–15 MB resident before any plugin code runs**, and plugin imports grow it (every plugin's `sys.modules` is shared global state). Compare the current runtime's numbers in `.plugin/dimensions/D06-performance-distribution.md:73-74` (wasmtime instantiation ~10–30 ms, per-blob 324–415 KB in `extensions/*/wasm/`). Per-call wasm instantiation amortizes; Python's 12–15 MB is paid once but forever, and GIL serialization caps concurrent calls regardless.

## 5. What Polars, pydantic-core, and cryptography actually do

All three flagship PyO3 users compile **Rust into Python**, not Python into Rust — every one is a `cdylib` extension module shipped as a wheel and loaded *by* CPython:

- **Polars**: `crates/polars-python` depends on `pyo3` with `features = ["abi3-py310", …]` (raw.githubusercontent.com/pola-rs/polars/main/crates/polars-python/Cargo.toml). The heavy engine is pure Rust; the Python process hosts it.
- **pydantic-core**: `crate-type = ["cdylib", "rlib"]`, `pyo3 = "0.26"` — with the maintainers' own comment that the `py-clone` feature "can panic" and they'd like to remove it (raw.githubusercontent.com/pydantic/pydantic-core/main/Cargo.toml, lines 23-24, 46-47).
- **cryptography**: `crate-type = ["cdylib"]` wrapping OpenSSL (raw.githubusercontent.com/pyca/cryptography/main/src/rust/Cargo.toml).

**Consequence for this decision**: the "PyO3 success stories" do not demonstrate *embedding* at all. Their packaging is solved by wheels/pip/maturin — machinery SpecForge would discard, not inherit. The only widely-cited embedding attempt (PyOxidizer) is stalled (§3), and the best-known embedders of CPython today (uv) deliberately deploy it as an **unpacked sidecar directory**, not an embedded library.

## 6. Failure modes (catalog)

1. **Version coupling**: binary built against libpython X fails at load on a machine with X±1 or none; the fix is shipping an interpreter (§3), which contradicts single-binary distribution.
2. **Sub-interpreters don't exist in PyO3.** Correction to `.plugin/dimensions/D03-python-embedding.md:47` (which says 0.22 "removed" them): PyO3 **never supported** them — since PR #2523 a PyO3 module initialized in a second interpreter raises `ImportError` as a deliberate soundness measure, 0.22's GIL rework (PR #4188) additionally broke the raw-FFI workarounds projects like arrow-udf used (issue **#4570**, closed *not planned*, 2024-11-19), and support remains an open multi-year redesign requiring "a substantial redesign of PyO3's API" (tracking issues **#3451**, open since 2023, and **#576**, open since 2019 — github.com/PyO3/pyo3/issues). Downstream breakage is real: cryptography hit it (pyca/cryptography#9016). Even CPython-side, isolated sub-interpreters only accept multi-phase-init extension modules, and CPython docs concede the insulation "isn't perfect" (docs.python.org/3/c-api/subinterpreters.html).
3. **Interpreter lifecycle is one-shot.** `Py_FinalizeEx` → re-init is not a supported cycle: CPython docs document shutdown as a one-way regime where non-finalizing threads that attach are "permanently blocked"; post-finalize use is the segfault class (PyO3 fixed "segfault when dropping `PyBuffer` after the interpreter has been finalized", PR #5242; historical: dropping the atexit-finalize hook "resolves a number of issues with incompatible C extensions causing crashes at finalization", CHANGELOG line 1972). My 5-cycle init/finalize toy loop *survived* (rc=0, warm 6.8 ms) — but that is exactly the hello-world case the docs' warnings are about. R-5 hot reload therefore degrades to `importlib.reload` semantics: stale `sys.modules`, zombie module state.
4. **Process-level blast radius.** Wasm traps are structured results (`WasmCallResult`, `WasmTrapInfo`, crates/specforge-wasm/src/runtime.rs:6,38-47); a Python plugin can `abort()` the host via `ctypes`, crash in a native extension, or hang the GIL — none catchable. PyO3's own soundness machinery panics (`py-clone` "can panic"; `Python::attach` panics at shutdown).
5. **Threaded-host hazards**: GIL serialization (§2), detach/deadlock discipline, `Already borrowed` under free-threading, no abi3 on free-threaded builds (§2).
6. **No sandbox**: CPython is not a security boundary; embedded plugins share the host address space with `open`/`os.system`/`ctypes` available (docs position consistent with pyo3.rs docs making no sandboxing claims; see also .plugin/evidence.md:115 — "pip ecosystem is ambient-capability"). R-2 fails by construction in-process.

## 7. Fit to SpecForge's integration surface

The `WasmRuntime` trait (`Send + Sync`; `load_module`, `call_export(&self, name, export, &[u8]) -> WasmCallResult`, `has_cached_module`, crates/specforge-wasm/src/runtime.rs:38-47) is mechanically implementable over PyO3 — `call_export`'s bytes-in/bytes-out JSON contract matches `py.eval` marshalling, and `ExtismRuntime` already serializes calls behind a `Mutex` (crates/specforge-extism/src/runtime.rs:18-22), so the GIL would not make the current call pattern slower. What cannot be implemented on this trait's terms: per-plugin isolation (one shared interpreter, `sys.modules` shared across all plugins — R-1's equal-treatment becomes equal *contagion*), crash containment (no trap analog; failure mode #4), destroy-and-recreate reload (failure mode #3), and R-3 packaging (§3).

## Bottom line

**Verdict: KEEP_WASM** · **Confidence: 5** · PyO3 itself is superb and battle-tested, but its maturity lives almost entirely in the Rust-as-extension-module direction — embedding CPython means a version-locked system libpython or a stalled 2-year-dead PyOxidizer path, a GIL that serializes any multi-threaded host, no sandbox, no crash containment, one-shot interpreter lifecycle, and none of the flagship projects (Polars, pydantic-core, cryptography) actually embed it.
