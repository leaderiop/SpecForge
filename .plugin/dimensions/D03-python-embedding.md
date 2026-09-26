# D03 — Embedding Python (CPython) as the Plugin Runtime

Analyst: 042 Samuel Colvin (validation/serialization cluster). I built a validation
engine twice — once in Python, once as pydantic-core in Rust. That history is the
whole assessment in miniature: Python is a superb authoring surface and a terrible
thing to put in your hot path.

## The workload Python must carry

Evidence §2: the four builtin guests are ~97% generated manifest, ~3% real logic —
the only logic-bearing guest is `@specforge/formal` (~478 lines of graph analysis in
4 `__pass_*` exports). Under R-1 the builtins are plugins like any other, so
"embed CPython" means: every analyze run executes the formal passes as pure Python,
GIL-serialized, in-process with the host. The declarative rules stay in the host's
`validation_engine.rs` either way; the Python-specific danger is the `Custom` escape
hatch, which `execute_pattern` calls **per entity, per rule**
(`crates/specforge-registry/src/compilation/validation_engine.rs:263-402`). Over a
~1.7k-entity graph, that is a Rust→Python boundary crossing with dict marshalling in
the innermost loop. That is precisely the loop I moved out of Python in 2021.

## Route 1: PyO3 against the system interpreter

PyO3 embedding requires linking a version-specific `libpython`. The abi3 stable ABI
is an extension-module facility — it does not apply to embedding. So the host binary
built against, say, CPython 3.12 dies with an undefined-symbol error on a machine
whose `python3` is 3.11 or 3.13, or absent entirely (bare macOS arm64 has no system
python3 until Xcode CLT is installed; Linux distros ship 3.8–3.13). R-3 says "a
runtime that requires system packages per user machine fails." This route fails R-3
by definition. Nothing to negotiate.

## Route 2: static CPython inside the single binary

The blunt truth: you cannot `include_bytes!` a working CPython. Statically linking
libpython gives you an interpreter whose stdlib extension modules (`_ssl`, `zlib`,
`array`, …) are still separate `.so`/`.dylib` files loaded at import time — `import
ssl` fails out of the box. To make Python actually *work* you must bundle the stdlib
`.py` files plus per-platform compiled extension modules in a relocatable layout:
that is what PyInstaller does, and PyOxidizer attempted and is effectively dead.
The python-build-standalone distributions (which uv downloads) are 30–50 MB
compressed per platform and expect to be unpacked on disk. Your host binary goes
from ~15–30 MB to 100 MB+ per target, or you ship a binary plus a Python payload
directory — which is no longer "single binary." R-3 is violated either way; R-4
reproducibility additionally inherits interpreter-build variance across platforms.

## Route 3: subinterpreters

Dead on arrival. PyO3 removed sub-interpreter support in 0.22 (2024) when CPython
3.12 broke the underlying C API, and it has not returned; the PyO3 maintainers
explicitly do not support running multiple subinterpreters. PEP 684's
per-interpreter GIL (3.12+) does not rescue it: C extension modules with global
state are unsafe there, and the ecosystem — the entire reason to choose Python —
is exactly those modules. And even per-interpreter isolation is *memory* isolation,
not a security boundary. Subinterpreters also cannot deliver R-5: `Py_Finalize()`
followed by re-initialization is unsupported and leaks module state, so "destroy
and recreate the interpreter" hot reload does not exist; you are left with
`importlib.reload`, i.e. stale `sys.modules` and the classic zombie-state bugs.

## Route 4: uv-run sidecar

This is the only honest Python deployment: an out-of-process interpreter with OS
boundaries. But then you have not embedded Python — you have invented an IPC plugin
protocol (stdin/stdout or a socket), a process lifecycle manager, and a first-run
dependency on downloading CPython via uv, i.e. a network dependency and a per-machine
cache that R-3 and R-4 both frown at. And note the audit pattern already present:
C14-03/C14-04 flag *blocking I/O in async contexts* as a recurring failure mode in
this codebase. A subprocess-per-plugin-call sidecar inside an async host is that
finding, industrialized.

## Sandboxing reality (R-2)

CPython has no sandbox. The maintainers' position, unchanged for a decade, is that
the interpreter is not a security boundary. Stripping `__builtins__` is performance
art, not confinement: `ctypes.CDLL(None).system(b"...")` reaches libc directly, and
any embedded extension module is a jailbreak. PEP 578 audit hooks are observability,
not enforcement. To honor R-2 you must fall back to the sidecar plus OS-level
sandboxing (seccomp/seatbelt) — at which point Python contributes the *least*
isolatable engine of all five candidates at the highest integration cost. Compare
the wasm path's one real gap, C7-04 (fs allow-by-default): a config fix. Python's
equivalent gap is the language itself.

## Performance and determinism

Batch workloads survive: the formal passes receive one `PassInput` JSON snapshot and
return diagnostics; CPython does that in tens of milliseconds. The killers are
(a) per-entity `Custom` dispatch (see above — GIL-serialized marshalling at ~50–100x
the wasm cost) and (b) the host's async executor: every `Python::attach` from worker
threads serializes on the GIL, so "parallel" plugin execution is fiction. The
free-threaded 3.14 build (PEP 779) is real but a niche interpreter variant — not
what any distro or PyO3 default ships — with a single-thread speed penalty and an
ecosystem still catching up. R-6 determinism holds for pure Python, but the *point*
of Python is the ecosystem, and the ecosystem (numpy BLAS threads, float formatting
across versions, unpinned transitive deps) is nondeterministic by default. The host
would have to ban the very thing that justifies the runtime.

## The honest upside

AI agents author excellent Python — better, arguably, than the Rust+wasm32
toolchain the current SDK demands (`specforge-extension-sdk` v0.1.0 on crates.io:
Rust toolchain, wasm32-unknown-unknown target, vendored 1.4 MB blobs). A pydantic-shaped
`PassInput`/`PassDiagnostic` surface would be genuinely pleasant, and the evidence
shows the project already touches Python as a *format* (pytest collector alias,
`crates/specforge-cli/src/collect.rs:39`; the test-only `@specforge/python`
extension). But R-1 makes authoring DX irrelevant to the core question: whatever
runs the builtins is the runtime, and the builtins' real logic (formal's graph
passes, host-side native custom rules since the C6-11 fix) would be dragged through
the GIL on every analyze. Choosing Python for a 3%-logic workload means paying
CPython's full packaging and isolation bill to author 3% of it in Python.

## Where Python could still belong

Not as THE runtime. A defensible future is a MULTI tier: out-of-process, capability-
denied, batch-only Python for collectors and prompt/text contributions — where
startup cost amortizes, no per-entity hot loop exists, and OS sandboxing is
available. That is a deliberate second mechanism with its own protocol, which
C7-11 warns the project already has two too many of. It should not be improvised
as "PyO3 in-process."

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** Embedded CPython fails two hard requirements outright — R-2 (no Python sandbox exists; ctypes escapes anything in-process) and R-3 (system-libpython fragility or a 100 MB+ non-single-binary bundle) — while the GIL meets SpecForge's per-entity rule-dispatch hot path, so Python can only ever be a separate out-of-process tier, not the runtime.
