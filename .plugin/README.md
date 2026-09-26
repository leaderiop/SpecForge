# Plugin Runtime Decision — Full Report (Markdown)

**Question:** should SpecForge keep WASM (Extism/Wasmtime) as its plugin runtime, or
adopt a dedicated embedded plugin language — Lua, Python, or TypeScript?

**Governing constraint (R-1, from the project owner):** there is no first-party plugin.
All extensions — the four builtins and any third-party one — are plugins of the same
kind, under the same security model. No native/trusted tier exists or will be created.

## How to read this report

| File | Content |
| --- | --- |
| [decision-brief.md](decision-brief.md) | The question, the five options, hard requirements R-1..R-6, evaluation criteria, output contract |
| [evidence.md](evidence.md) | Verified facts: current architecture, audit record, measured numbers |
| [decision-matrix.md](decision-matrix.md) | Options × criteria matrix + hard-requirements check |
| [dimensions/](dimensions/) | 12 deep-dive analyses, one per decision dimension |
| [personas/](personas/) | 125 engineer-persona position statements |
| [decision.md](decision.md) | Final synthesis: tally, recommendation, conditions |

An HTML presentation of the same content lives at [index.html](index.html).

## The options

| Label | Meaning |
| --- | --- |
| `KEEP_WASM` | Stay with Extism/Wasmtime; fix the audit gaps inside the model |
| `LUA` | Embed Lua 5.4 (mlua) as the plugin language |
| `PYTHON` | Embed CPython (PyO3) as the plugin language |
| `TYPESCRIPT` | Embed a JS/TS engine (quickjs-ng / deno_core) as the plugin language |
| `MULTI` | Support several runtimes behind one protocol |

## Hard requirements

- **R-1** — all plugins equal, no first-party tier
- **R-2** — untrusted third-party plugins are sandboxable
- **R-3** — single-binary host (macOS arm64, Linux x64)
- **R-4** — signed registry distribution
- **R-5** — hot reload in watch
- **R-6** — deterministic, snapshot-testable analyze
