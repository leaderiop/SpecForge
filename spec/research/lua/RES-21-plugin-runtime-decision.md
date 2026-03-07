# RES-21: Plugin Runtime Architecture Decision

**Status:** Decision Override — Wasm/Extism Accepted
**Date:** 2026-03-03
**Method:** 5-agent expert panel (Lua specialist, Wasm specialist, Alternatives specialist, SpecForge architect, Neovim ecosystem analyst)

---

## The Question

Should SpecForge embed a scripting engine (Lua, Rhai, Wasm, etc.) to allow runtime extensibility for plugins and providers — similar to how Neovim uses LuaJIT?

## Decision Outcome

**Wasm/Extism is the unified plugin runtime for all extension types.**

The 5-agent panel split 3-2 against embedded scripting and recommended a tiered subprocess model. This recommendation was **overridden** — the project adopts Wasm/Extism as the sole plugin runtime. The panel's concerns about cold start and binary size are addressed by AOT compilation caching (CLI) and warm engine instances (LSP/MCP). The benefits of hardware-enforced sandboxing, multi-language authoring, and universal `.wasm` binary distribution outweigh the overhead costs.

See ADRs: `wasm_extism_plugin_runtime`, `wasm_peer_dependencies` in `spec/governance/decisions.spec`.

### Why Wasm Over the Panel's Recommendation

| Panel Concern | Resolution |
|---------------|------------|
| 50-150ms cold start for CLI | AOT compilation caches `.wasm` modules; amortized across runs |
| +5MB binary size | Acceptable tradeoff for hardware sandboxing and multi-language support |
| Toolchain complexity for authors | Extism PDKs exist for Rust, Go, JS, Python — friction is manageable |
| Subprocess JSON-RPC is simpler | Simpler but weaker — no sandboxing, no universal binary, IPC overhead per call |
| Overkill for simple validators | Uniform runtime is simpler to document and maintain than two protocols |

### Rejected Alternatives

- **Subprocess JSON-RPC** — No sandboxing, platform-specific binaries, IPC overhead
- **Lua 5.4 / mlua** — Single language, weaker sandboxing than Wasm
- **Rhai** — Single language, Rust-only ecosystem
- **Starlark / Deno** — Niche ecosystems, wrong fit
- **Tiered model (Declarative + Subprocess + SDK)** — Complexity of maintaining multiple protocols

---

## Expert Panel Summary (Historical)

The panel research below is preserved as context for the decision override.

| Expert | Recommendation | Rationale |
|--------|---------------|-----------|
| Lua specialist | Lua 5.4 via `mlua` | Good fit IF scripting is needed. ~5-10ms overhead. |
| Wasm specialist | Extism (Wasm) | Superior sandboxing, multi-language. +5MB binary. |
| Alternatives specialist | **Rhai** (Rust-native) | Pure Rust, zero FFI, 1MB overhead. Best for Phase 1. |
| SpecForge architect | **No scripting** | Tree-sitter is compile-time. Entity model is enum-based. Scripting can't extend the parser. |
| Neovim analyst | **No scripting** | Neovim needed Lua for real-time interactivity. SpecForge is a batch compiler — needs reproducibility, not scriptability. |

---

## The Fundamental Constraint: Tree-sitter

> [!NOTE]
> **Updated (RES-26):** The grammar now uses a generic `entity_block` rule that parses ANY `keyword name { fields }` pattern. Validation of which keywords are legal comes from extensions, not the grammar. The original constraint below described the old hardcoded approach.

The grammar now uses a generic `entity_block` rule:

```javascript
// grammar.js — generic entity_block (RES-26)
_block: ($) => choice(
    $.entity_block,   // generic — parses ANY keyword
    $.spec_block,     // structural (stays in core)
    $.use_statement,  // structural (stays in core)
    $.define_block,   // structural (stays in core)
),
```

**Tree-sitter grammars compile to C and are statically linked.** The grammar now parses blocks generically. Wasm extensions operate **downstream** of parsing, on the already-parsed graph via host functions (`specforge.query_graph`, `specforge.emit_diagnostic`, etc.).

## Why Neovim's Model Doesn't Apply

| Neovim (interactive editor) | SpecForge (batch compiler) |
|-|-|
| Responds to keypresses in <16ms | Runs once, produces output |
| Modifies buffer content on-the-fly | Reads files, writes artifacts |
| Infinite customization surface | Fixed transformation pipeline |
| Long-running process (hours) | Short-running process (seconds) |
| User-facing scripting | CI/CD integration |
| Mutable state is the point | Determinism is the point |

Neovim needed Lua because users must customize editor behavior in real-time. SpecForge needs **reproducible, versioned, CI-compatible transformations**. The Wasm approach serves this better than Lua — deterministic execution, sandboxed, versionable `.wasm` artifacts.

## What Wasm Plugins Can Extend

| Extension Point | Mechanism | Host Function |
|----------------|-----------|---------------|
| **Validation rules** | Post-parse graph analysis | `specforge.query_graph` + `specforge.emit_diagnostic` |
| **Custom entities** | Entity registration | `specforge.register_entity` |
| **File emission** | File output | `specforge.emit_file(path, content)` |
| **Ref providers** | External service validation | `specforge.http_get` |
| **Custom edges** | Edge registration | `specforge.register_edge` |

## Wasm Runtime Architecture

### Host Functions (SpecForge → Plugin)

```
specforge.query_graph(query) → entities/edges
specforge.emit_diagnostic(code, severity, message, span)
specforge.register_entity(kind_def)
specforge.emit_file(path, content)
specforge.http_get(url) → response
```

### Lifecycle

```
1. specforge reads specforge.json
2. Validates peer_dependencies for all declared extensions
3. Topologically sorts extensions (official → third-party)
4. For CLI: AOT-compile and cache .wasm modules
5. For LSP/MCP: keep warm engine instances in-process
6. Load extensions, call initialize() → registers entities, edges, validators
7. After graph is built, call validate() on each extension
8. Merge diagnostics with built-in validation results
```

### Performance Mitigations

| Surface | Strategy |
|---------|----------|
| **CLI** | AOT compilation cache — first run compiles, subsequent runs load native code |
| **LSP** | Warm Extism engine kept in-process, plugins stay loaded across edits |
| **MCP** | Same as LSP — long-running process, warm engines |
| **CI** | Cache `.wasm` AOT artifacts in CI cache layer |

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Cold start (CLI) | 50-150ms first load | AOT caching reduces to <10ms on subsequent runs |
| Binary size (+5MB) | Larger distribution | Acceptable for sandboxing + multi-language benefits |
| Plugin author toolchain | Friction for new authors | Extism PDKs + `specforge plugin init` scaffolding |
| Debugging across Wasm boundary | Harder stack traces | Host-side logging, `specforge plugin test` command |
| WASI maturity (networking) | Experimental in Preview 2 | Use `specforge.http_get` host function instead of WASI networking |

## Action Items

1. [ ] Add `extism` dependency to SpecForge
2. [ ] Define host function API (`specforge.query_graph`, `specforge.emit_diagnostic`, etc.)
3. [ ] Implement Wasm plugin loader with AOT caching
4. [ ] Implement peer dependency validation and topological sort
5. [ ] Create `specforge plugin init` scaffolding command (Rust, Go, JS templates)
6. [ ] Create `specforge plugin test` for local plugin development
7. [ ] Document the Wasm plugin protocol in `docs/extension-protocol.md`
8. [ ] Port `@specforge/product` and `@specforge/governance` to Wasm modules

## References

- ADR `wasm_extism_plugin_runtime` in `spec/governance/decisions.spec`
- ADR `wasm_peer_dependencies` in `spec/governance/decisions.spec`
- See `RES-21b-wasm-plugin-analysis.md` for full Wasm research (adopted)
- See `RES-21e-specforge-architecture-constraints.md` for codebase analysis
