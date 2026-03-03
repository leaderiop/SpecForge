# RES-21a: Lua Embedding in Rust — Deep Analysis

**Expert perspective:** Roberto Ierusalimschy's design philosophy + Rust ecosystem analysis
**Decision outcome:** REJECTED — Wasm/Extism selected as unified plugin runtime (see ADR `wasm_extism_plugin_runtime`)

---

## 1. Rust Crate Landscape

| Crate | Stars | Status | Lua Versions | Async | Safety |
|-------|-------|--------|-------------|-------|--------|
| **mlua** | 2.6k | Active (v0.11.6, Jan 2026) | 5.1-5.5, LuaJIT, Luau | Yes (coroutines) | Safe API, Send+Sync |
| **rlua** | 1.8k | Archived (Sept 2025) | 5.3-5.4 | No | Wrapper around mlua now |
| **hlua** | — | Abandoned (~2019) | 5.2 | No | Incomplete |
| **piccolo** | ~400 | Experimental | Custom (Lua-ish) | Yes | Pure Rust, no C |

**Winner: `mlua`** — no contest. It's the only actively maintained, feature-complete option.

### mlua Key Features

- Supports Lua 5.1, 5.2, 5.3, 5.4, 5.5, LuaJIT, and Luau
- Full async/await support via Lua coroutines mapped to Rust futures
- WebAssembly target support
- `UserData` trait for exposing Rust types to Lua
- Built-in sandbox mode (removes `io`, `os`, `debug`, `require`)
- Module mode for creating native Lua extensions
- Memory limit and hook-based CPU limits

## 2. LuaJIT vs Lua 5.4 for SpecForge

| Aspect | LuaJIT 2.1 | Lua 5.4 | Recommendation |
|--------|-----------|---------|----------------|
| Performance | 2-100x faster (JIT) | Interpreted | LuaJIT wins but irrelevant for CLI |
| Integers | No (doubles only) | Yes (64-bit) | **Lua 5.4** — IDs, line numbers |
| Startup | Slower (JIT warmup) | Fast (~0.1ms) | **Lua 5.4** — CLI runs are short |
| Memory | Higher (JIT overhead) | Lower (~100KB) | **Lua 5.4** |
| UTF-8 | External library | Built-in | **Lua 5.4** |
| Generational GC | No | Yes | **Lua 5.4** |
| FFI | Yes (call C directly) | No | LuaJIT wins but unnecessary in Rust |
| Maintenance | Mike Pall semi-retired | PUC-Rio active | **Lua 5.4** |

**Verdict: Lua 5.4.** LuaJIT's performance advantage is wasted on a CLI tool with <1s runtime. Integer support and modern features tip the balance.

## 3. Lua as Configuration Language

### The Config→Code Spectrum

```
JSON/TOML ←──── Pure data ────── Computed config ────── Full logic ───→ General purpose
   ↑                                    ↑                    ↑               ↑
specforge.json            Lua return {}         Lua functions      Rust plugin SDK
```

### Real-World Precedents

| Tool | Config Language | Pattern | Success? |
|------|----------------|---------|----------|
| **Neovim** | Lua (init.lua) | Full scripting | Yes — but editor needs it |
| **Awesome WM** | Lua (rc.lua) | Programmatic UI | Yes — but WM needs dynamic layout |
| **Hammerspoon** | Lua (init.lua) | Automation scripts | Yes — but automation IS scripting |
| **OpenResty** | Lua (nginx.conf) | Request-time logic | Yes — but HTTP requires dynamic routing |
| **Kong** | Lua plugins | Request/response hooks | Yes — but API gateway needs filters |
| **Redis** | Lua scripts | Atomic operations | Yes — transactional scripting |

**Pattern:** Every successful Lua embedding is in a **long-running, interactive, or event-driven** system. None are batch compilers.

## 4. Performance Analysis for SpecForge

### Overhead Budget

| Operation | Time | With Lua overhead |
|-----------|------|-------------------|
| Parse 50 .spec files | ~50ms | +0.65ms (engine init) |
| Build graph | ~20ms | +0ms (Lua not involved) |
| Validate | ~30ms | +5-10ms (Lua validators) |
| Generate output | ~20ms | +2-5ms (Lua generators) |
| **Total** | **~120ms** | **~128-136ms** |

**Conclusion:** ~7-13% overhead. Acceptable but not free. For a CLI that runs in CI on every commit, this adds up across thousands of runs.

## 5. Security Model

### mlua Sandbox

```rust
let lua = Lua::new();
lua.sandbox(true)?; // Removes: io, os, debug, require, load, dofile

// Additional restrictions
lua.set_hook(HookTriggers::every_nth_instruction(100_000), |_, _| {
    Err("CPU limit exceeded".into())
})?;
lua.set_memory_limit(10 * 1024 * 1024)?; // 10MB
```

**Removed in sandbox mode:** `io.*`, `os.execute`, `os.remove`, `os.rename`, `debug.*`, `require`, `package.*`, `load`, `loadfile`, `dofile`

**Still available:** `string`, `table`, `math`, `coroutine`, `utf8`, `pcall`, `type`, `tostring`

### Trust Levels

| Level | Access | Example |
|-------|--------|---------|
| **Official** | Full graph + filesystem + network | `@specforge/product` |
| **Verified** | Full graph + read-only filesystem | Community validators |
| **Untrusted** | Read-only graph, no I/O | User-submitted snippets |

## 6. Package Distribution

### Neovim's lazy.nvim Model

```lua
-- specforge.lua (hypothetical)
return {
  {"specforge/product", tag = "v1.0.0"},
  {"community/epic-validator", branch = "main"},
  {"myorg/custom-generator", path = "~/plugins/gen"},
}
```

**Distribution:** Git repos (not LuaRocks). Lock file pins commits.

### Problems with This Model for SpecForge

1. **CI reproducibility** — Git repos can be force-pushed. Need content-addressed hashing.
2. **Transitive dependencies** — No dependency resolution between plugins.
3. **Version conflicts** — Two plugins requiring different versions of a shared library.
4. **Security audit** — Can't audit all Git repos. Need signed releases.
5. **Offline builds** — Corporate environments may block Git clones.

## 7. Why Lua Was Rejected

Lua via mlua is technically viable with acceptable performance (~7-13% overhead). However:

1. **Single-language lock-in** — plugin authors must write Lua, no choice
2. **Software sandboxing only** — mlua sandbox is enforced in software, not hardware (Wasm linear memory is hardware-enforced)
3. **No universal binary** — Lua scripts are source-distributed, not compiled artifacts
4. **Tree-sitter prevents parser extension** — the primary motivation for scripting is impossible regardless
5. **Batch compiler semantics** favor reproducibility over scriptability
6. **The distribution problem** is harder than the embedding problem — Wasm's `.wasm` binaries via npm/OCI solve this cleanly

### What Wasm Does Better

| Concern | Lua | Wasm/Extism |
|---------|-----|-------------|
| Sandboxing | Software (mlua sandbox) | Hardware (linear memory) |
| Language | Lua only | Rust, Go, JS, Python, C |
| Distribution | Git repos / LuaRocks | npm, OCI, GitHub Releases |
| Binary artifact | Source scripts | Compiled `.wasm` |
| Determinism | GC non-determinism | Deterministic execution |
| Versioning | Pin Git commits | semver on package registries |
