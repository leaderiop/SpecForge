# RES-21b: WebAssembly/WASI Plugin System Analysis

**Expert perspective:** Wasm ecosystem specialist, Extism/Wasmtime deep-dive
**Decision outcome:** ADOPTED — Wasm/Extism is the unified plugin runtime (see ADR `wasm_extism_plugin_runtime`)

---

## 1. Runtime Comparison

| Runtime | Stars | Focus | WASI Support | Component Model | Binary Size Impact |
|---------|-------|-------|-------------|-----------------|-------------------|
| **Extism** | 5.5k | Plugin systems | Partial | No | +3-5MB |
| **Wasmtime** | 17.7k | General Wasm | Full (Preview 2) | Yes | +5-8MB |
| **Wasmer** | 19.2k | General Wasm | WASIX | Partial | +5-8MB |
| **wasm3** | 7.5k | Embedded/IoT | Preview 1 | No | +500KB |

### Extism — Purpose-Built for Plugins (Selected)

Extism adds a plugin-oriented layer over Wasmtime:
- **Simplified memory management** — host allocates, plugin writes to offsets
- **Built-in HTTP** without full WASI
- **Persistent module-scope variables** — state across calls
- **Runtime limiters and timers** — sandboxing built-in
- **15+ host SDKs** — Rust, JS, Go, Python, Ruby, etc.
- **10+ plugin PDKs** — Rust, JS, Go, Python, AssemblyScript, etc.

**Why Extism over raw Wasmtime:** Extism's plugin-oriented abstractions (host functions, memory management, built-in HTTP) match SpecForge's needs exactly. Raw Wasmtime would require reimplementing this layer.

## 2. Multi-Language Plugin Authoring

### Plugin Size by Language

| Language | Typical .wasm Size | Runtime Included? | Dev Experience |
|----------|-------------------|-------------------|----------------|
| **Rust** | 100KB - 2MB | No | Excellent |
| **AssemblyScript** | 50KB - 500KB | Minimal | Good (TS-like) |
| **C/C++** | 50KB - 1MB | No | Manual memory |
| **Go** | 2 - 10MB | Yes (TinyGo better) | Good but large |
| **JavaScript** | 500KB - 5MB | QuickJS embedded | Familiar |
| **Python** | 5 - 20MB | CPython embedded | Poor DX |

### Friction Points

1. **Go produces huge binaries** due to runtime inclusion (TinyGo helps but limits stdlib)
2. **Python requires embedding CPython** — 20MB+ binary, slow startup
3. **Debugging across Wasm boundary** is painful — no step-through debugger
4. **Async/await** not supported in most PDKs

### Recommended Plugin Languages for SpecForge

**Tier 1 (official templates):** Rust, AssemblyScript (TypeScript-like)
**Tier 2 (community supported):** Go (via TinyGo), JavaScript (via Javy/QuickJS)
**Tier 3 (possible but not recommended):** Python, C/C++

## 3. Real-World Deployments

### Zellij (Terminal Multiplexer)
- Custom Wasm runtime, any language
- GitHub releases for distribution
- **Lesson:** Plugin startup time matters for interactive tools

### Lapce (Code Editor)
- WASI-based, C/Rust/AssemblyScript
- Plugins in separate processes
- **Lesson:** Async plugin calls prevent UI blocking

### Fermyon Spin (Microservices)
- Wasmtime + Component Model
- `spin new`, `spin build`, `spin up` DX
- 6.3k stars, production deployments
- **Lesson:** CLI-driven DX works well — model for `specforge plugin init/build/test`

### Envoy Proxy
- V8 Wasm filters for HTTP processing
- <1ms overhead per request
- **Lesson:** Wasm overhead acceptable for I/O-bound tasks

## 4. Performance Characteristics

| Metric | Extism/Wasm | Lua 5.4 | Rhai | Native Rust |
|--------|-------------|---------|------|-------------|
| Engine startup | 10-50ms | 0.5ms | 1ms | 0ms |
| Function call | 10-50us | 1-5us | 5-20us | 0.01us |
| Memory per plugin | 1-10MB | 100KB-1MB | 500KB | 0 |
| Execution speed | Near-native | 10-50x slower | 10-50x slower | Baseline |
| Binary size impact | +5MB | +2MB | +1MB | 0 |

### Cold Start Mitigation Strategy

The original analysis flagged cold start as "PROBLEMATIC for CLI":

```
Cold start budget: <200ms total
Wasm engine init:   10-50ms  (5-25% of budget!)
Plugin loading:     10-20ms per plugin
Total for 5 plugins: 60-150ms — PROBLEMATIC for CLI
```

**Adopted mitigations:**

| Surface | Strategy | Expected Latency |
|---------|----------|-----------------|
| **CLI (first run)** | AOT-compile `.wasm` → native code, cache in `.specforge/cache/` | 60-150ms (one-time) |
| **CLI (subsequent)** | Load AOT-cached native code | <10ms total |
| **LSP** | Warm Extism engine kept in-process across edits | 0ms after initial load |
| **MCP** | Same as LSP — long-running process | 0ms after initial load |
| **CI** | Cache `.specforge/cache/` in CI cache layer | <10ms with warm cache |

The cold start concern is real but **only affects the first CLI invocation without cache**. All subsequent runs and long-running surfaces are unaffected.

## 5. WASI Status (2026)

### Preview 2 (Released Sept 2024)

| Capability | Status | Notes |
|-----------|--------|-------|
| Filesystem | Stable | Restricted by capabilities |
| Clocks/timers | Stable | |
| Random | Stable | |
| Env vars | Stable | |
| Networking | Experimental | Not reliable |
| Threading | Not standardized | Major gap |

### Component Model

**Status:** Standardized but early adoption. Tooling immature. Larger binaries.

**Impact for SpecForge:** Component Model would enable type-safe plugin interfaces with WIT (WebAssembly Interface Types), but the toolchain complexity is high. Extism's host function model is sufficient for now; WIT can be adopted later as tooling matures.

### SpecForge's Approach to WASI Gaps

SpecForge does **not** rely on WASI capabilities directly. Instead, all external access is mediated by host functions:

| Need | WASI Status | SpecForge Host Function |
|------|-------------|------------------------|
| Graph access | N/A | `specforge.query_graph` |
| Diagnostics | N/A | `specforge.emit_diagnostic` |
| File output | Stable but sandboxed | `specforge.emit_file` (host validates paths) |
| HTTP access | Experimental | `specforge.http_get` (host mediates) |
| Entity registration | N/A | `specforge.register_entity` |

This means SpecForge plugins work regardless of WASI maturity — the host controls all I/O.

## 6. Sandboxing — Wasm's Killer Feature

```
Default: ZERO access to anything

Granted via capabilities:
  +fs:read("/project/specforge.json")
  +fs:write("/project/generated/**")
  -network (blocked)
  -process (blocked)
  -env (blocked)
```

**Hardware-enforced isolation** — linear memory, no pointer escape, no arbitrary syscalls. This is categorically stronger than any Lua/Rhai sandboxing.

This was a primary factor in the decision override. For a tool consumed by AI agents in CI pipelines, hardware sandboxing is not a nice-to-have — it's a security requirement.

## 7. Distribution Models

| Method | Tooling | Versioning | Discovery |
|--------|---------|-----------|-----------|
| **npm packages** | Mature | semver | npmjs.com |
| **GitHub Releases** | Simple | Git tags | GitHub search |
| **OCI registries** | Docker tooling | OCI tags | Docker Hub/GHCR |
| **WAPM** | Wasmer-specific | semver | wapm.io |
| **Crates.io (source)** | cargo | semver | crates.io |

**Adopted for SpecForge:** npm as primary (widest reach), GitHub Releases as fallback, OCI for enterprise. Universal `.wasm` binaries — same artifact runs on all platforms without recompilation.

## 8. Verdict for SpecForge

### Why Wasm/Extism Was Selected

- **Hardware-enforced sandboxing** — critical for AI agent CI pipelines
- **Multi-language plugins** — authors choose Rust, Go, JS, not locked to one language
- **Universal binary distribution** — one `.wasm` runs everywhere, no per-platform builds
- **Near-native execution speed** — once loaded, plugins run at near-native performance
- **Uniform runtime** — same model for plugins, providers, and generators (one protocol to learn)
- **Growing ecosystem momentum** — Zed, Zellij, Lapce, Fermyon all adopting Wasm plugins

### Accepted Tradeoffs

- **+5MB binary size** — acceptable for the sandboxing and multi-language benefits
- **Cold start overhead** — mitigated by AOT caching (CLI) and warm engines (LSP/MCP)
- **Toolchain complexity for authors** — mitigated by `specforge plugin init` scaffolding and Extism PDKs
- **Debugging difficulty** — mitigated by host-side logging and `specforge plugin test` command
