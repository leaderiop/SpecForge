# RES-21d: Lessons from Neovim's Lua Plugin Ecosystem

**Expert perspective:** TJ DeVries, folke (lazy.nvim), Neovim core team analysis
**Decision outcome:** Neovim's Lua model rejected for SpecForge. Wasm/Extism adopted instead. Key patterns (plugin management, lock files, health checks) carry forward.

---

## 1. Why Neovim Adopted Lua

- **Performance:** VimL interpreter was 10-100x slower than LuaJIT
- **Language quality:** VimL has quirky semantics; Lua is a real language
- **Embeddability:** Lua designed for embedding (180KB runtime)
- **JIT compilation:** LuaJIT made hot paths competitive with C

### Adoption Timeline
- 2014-2016: Experimental Lua API
- 2017-2019: Early adopters (telescope.nvim proved the model)
- 2020-2021: Tipping point (~30% plugins Lua-native)
- 2022-2024: Mainstream (~70% new plugins Lua-first)
- 2025-2026: Near-complete (~85% actively maintained plugins have Lua versions)

## 2. Plugin Discovery & Loading — lazy.nvim Revolution

```lua
{
  "nvim-telescope/telescope.nvim",
  dependencies = { "nvim-lua/plenary.nvim" },
  cmd = "Telescope",       -- Lazy-load on command
  keys = { "<leader>ff" }, -- Lazy-load on keymap
  config = function()
    require("telescope").setup({ ... })
  end
}
```

**Key mechanisms:**
- Lazy loading by command, keymap, filetype, event
- Topological sort for dependency resolution
- `lazy-lock.json` pins exact Git commits
- Parallel Git fetches

**Distribution:** 99% GitHub repos (not LuaRocks). No central registry.

## 3. API Surface — Three Layers

```lua
-- Layer 1: Low-level C API
vim.api.nvim_buf_set_lines(0, 0, -1, false, {"hello"})

-- Layer 2: VimL compatibility
vim.fn.expand("%:p")

-- Layer 3: Lua-idiomatic wrappers
vim.keymap.set("n", "<leader>x", function() ... end)
```

**Hardest APIs to get right:**
1. Buffer/window handle lifetime (invalidation between event loop iterations)
2. Autocommand race conditions (multiple plugins on same event)
3. Namespace collisions (highlight groups, keymaps stomping each other)

## 4. Problems & Anti-Patterns

| Problem | Manifestation | Severity |
|---------|--------------|----------|
| Plugin conflicts | Two plugins map `<leader>f` | High |
| Startup performance | 200+ plugins = 2s startup | High |
| Security | Arbitrary code from GitHub repos | Medium |
| Debugging | Which of 50 plugins caused this error? | High |
| Dependency hell | Plugin A needs plenary v0.1, B needs v0.2 | Medium |

### What Neovim Would Do Differently Today (Core Team, 2024-2026)

1. **Built-in package manager from day 1** (not rely on community)
2. **Stricter API versioning** (v1, v2 namespaces, not just deprecation)
3. **Standard library** (blessed utilities to prevent duplicate helpers)
4. **Sandboxing option** (opt-in capability system for security)

## 5. Editor Extension Models Compared (2025-2026)

| Editor | Model | Isolation | Language | Trend |
|--------|-------|-----------|----------|-------|
| **VS Code** | Node.js separate process | Process | JS/TS | Mature, stable |
| **Neovim** | Embedded LuaJIT | None | Lua | Mature, successful |
| **Zed** | Wasm plugins | Hardware | Any→Wasm | Growing |
| **Helix** | None (yet) | N/A | N/A | Community frustrated |
| **Lapce** | Wasm (WASI) | Hardware | C/Rust/AS | Early |
| **Sublime** | Python plugins | In-process | Python | Mature, aging |

**2025-2026 Trend:** Wasm is winning for NEW editors. Sandboxing + language agnosticism are killer features. SpecForge follows this trend.

## 6. The Critical Mismatch: Editor vs Compiler

### Why Lua Works for Neovim

Neovim is an **interactive, long-running, event-driven** application:
- Users type → Lua reacts in <16ms
- Plugins modify buffers, create UI elements, handle events
- Customization is the product's core value
- Process lives for hours/days

### Why SpecForge Chose Wasm Over Lua

SpecForge is a **batch, short-lived, deterministic** compiler:
- Read .spec files → produce artifacts
- Runs for <1 second
- CI/CD compatibility is critical
- Reproducibility is a hard requirement
- No interactive UI to customize

Wasm fits this profile better than Lua because:
- **Deterministic execution** — no GC non-determinism
- **Hardware sandboxing** — critical for CI pipelines with third-party plugins
- **Universal binaries** — same `.wasm` runs everywhere, perfect for CI reproducibility
- **Multi-language** — plugin authors aren't locked to Lua

## 7. What SpecForge Copies from Neovim (Applied to Wasm)

| Neovim Pattern | SpecForge Wasm Equivalent |
|---------------|--------------------------|
| Plugin manifests | Wasm plugin manifests with `peer_dependencies` |
| Lock files (`lazy-lock.json`) | `specforge-lock.json` pinning `.wasm` artifact hashes |
| Namespaced APIs | `@specforge/` package prefix for official plugins |
| Deprecation warnings | Validation code versioning in plugin manifests |
| `:checkhealth` | `specforge doctor` command (validates plugin loading) |
| `:Lazy` UI | `specforge plugin list/update/add/init` |
| Topological dependency sort | Topological plugin loading order (core → official → third-party) |

## 8. What SpecForge Does NOT Copy

1. **Embedded scripting language** — Wasm host functions instead of Lua API
2. **Dynamic plugin loading** — plugins declared in `specforge.json`, loaded at startup
3. **Runtime plugin installation** — extensions declared upfront, installed via `specforge add`
4. **Mutable configuration** — config is immutable during compilation
5. **Event system** — batch compiler uses lifecycle hooks (initialize → validate → generate → shutdown), not editor events
6. **Global state** — each Wasm plugin gets isolated linear memory, no shared globals

## 9. Key Takeaway

> "Neovim needed Lua because editors require embedded, real-time scriptability.
> SpecForge needs Wasm because compilers require sandboxed, reproducible, multi-language extensibility.
> The problems are orthogonal — but both ecosystems teach the same lesson: invest in plugin DX (scaffolding, testing, distribution) from day 1."
