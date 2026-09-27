# Case Study — Neovim's Lua Plugin Ecosystem

**Researcher:** `research-neovim-lua` · **Date:** 2026-09-27
**Question:** What does Neovim — the largest real-world deployment of "embed Lua for plugins" — actually look like, and would SpecForge's spec-validation/analysis workload face the same problems?

---

## 1. How Neovim embeds Lua

Neovim embeds a Lua interpreter directly into the editor process. Its help doc states: *"The Lua 5.1 script engine is builtin and always available"*, and *"Lua 5.1 is the permanent interface for Nvim Lua"* — the interpreter is normally **LuaJIT**, chosen *"for performance reasons"*, which additionally exposes the **LuaJIT FFI** for calling arbitrary C from Lua ([runtime/doc/lua.txt:14-49](https://raw.githubusercontent.com/neovim/neovim/master/runtime/doc/lua.txt)).

Key characteristics of the embedding:

- **Single shared interpreter in the host process.** Plugins, config (`init.lua`), and internal modules all run in one Lua state with the `vim` API as the bridge — there is no per-plugin isolation.
- **Full host I/O via `vim.uv`.** Lua gets direct libuv bindings: filesystem events, timers, TCP servers/clients, processes ([lua.txt:471-558](https://raw.githubusercontent.com/neovim/neovim/master/runtime/doc/lua.txt)). Combined with the FFI, Lua plugins have ambient access to the entire machine.
- **Headless/batch mode.** The same embedding runs scripts without a UI (`nvim --headless`), which is why Neovim also uses Lua as its *own* implementation language for builtin plugins.
- Lua became the primary plugin/config language with the **0.5 release (2 July 2021)**, alongside built-in LSP and Tree-sitter ([Wikipedia: Neovim](https://en.wikipedia.org/wiki/Neovim)).

## 2. The sandbox model: effectively none

Neovim has no plugin sandbox. Three facts establish this precisely:

1. **The only "sandbox" is a per-command guard for option expressions.** Vim/Neovim's `:sandbox` exists to protect against expressions set from *modelines* or tags files; while inside it, shell execution, file read/write, buffer changes, and mappings are blocked — and the documentation itself says: *"This is not guaranteed 100% secure, but it should block most attacks"* ([vimhelp.org/eval.txt.html §eval-sandbox](https://vimhelp.org/eval.txt.html)). It is exposed programmatically only as an opt on Ex-command execution (`sandbox: (boolean)` in `nvim_cmd`/`nvim_exec2` options, [runtime/doc/api.txt:3117](https://raw.githubusercontent.com/neovim/neovim/master/runtime/doc/api.txt)). It was never designed as a plugin isolation boundary.
2. **Config files are fully trusted code.** Neovim *removed* Vim's `secure` option: *"Everything is allowed in 'exrc' files, because they must be [trusted]"* ([runtime/doc/vim_diff.txt:902](https://raw.githubusercontent.com/neovim/neovim/master/runtime/doc/vim_diff.txt)). The mitigation is a **trust database with a user prompt** — `vim.secure.read()` / `vim.secure.trust()` persisting allow/deny choices in `$XDG_STATE_HOME/nvim/trust` ([lua.txt:5141-5177](https://raw.githubusercontent.com/neovim/neovim/master/runtime/doc/lua.txt)). This is consent, not containment.
3. **Installation runs arbitrary code.** Plugin managers execute post-update hooks as shell commands or Lua functions — e.g. vim-plug's `{ 'do': './install --all' }` and `'do': 'make'` ([vim-plug README:171-176, 310-312](https://raw.githubusercontent.com/junegunn/vim-plug/master/README.md)). Installing a plugin is arbitrary code execution by design.

Consequences are visible in the advisory record: GHSA-6f9m-hj8h-xjgj, *"Arbitrary code execution when using treesitter with injections"* (published Feb 2023, Moderate, by core maintainer bfredl) is the flagship entry in Neovim's own security advisories ([github.com/neovim/neovim/security/advisories](https://github.com/neovim/neovim/security/advisories)). Neovim core has very few CVEs; the exposure lives in the trust model, not in memory-safety bugs.

## 3. Ecosystem size

- **awesome-neovim** (the canonical curated list) links **~1,480 plugins** — my count of top-level entries in [README.md](https://raw.githubusercontent.com/rockerBOO/awesome-neovim/main/README.md), fetched 2026-09-27.
- GitHub's `topic:neovim-plugin` tag matches **5,126 repositories** ([GitHub search API](https://api.github.com/search/repositories?q=topic:neovim-plugin), 2026-09-27).
- **neovim/neovim: 102,595 stars**, 7,142 forks ([GitHub API](https://api.github.com/repos/neovim/neovim), 2026-09-27); Neovim was voted Stack Overflow's *most admired development environment for the fifth consecutive year* in 2025 ([Wikipedia: Neovim](https://en.wikipedia.org/wiki/Neovim)).

Order of magnitude: **thousands of actively distributed plugins**, most written in Lua, all running unsandboxed with full machine access.

## 4. Known problems

### 4.1 Performance
The ecosystem's #1 plugin manager, lazy.nvim, exists chiefly to fight Lua startup cost: its headline features are *"Fast startup times thanks to automatic caching and bytecode compilation of Lua modules"* and automatic lazy-loading on events/filetypes/commands ([lazy.nvim README](https://github.com/folke/lazy.nvim)). When the standard practice for a plugin manager is byte-compiling and deferring plugins to mask interpreter startup, that is an ecosystem-scale admission. Inside the runtime, performance is constrained by the single-threaded host: `vim.uv` callbacks may not call most `vim.api` functions (error E5560) because the majority of API functions cannot run in `api-fast` context ([lua.txt:476-497, 892-895](https://raw.githubusercontent.com/neovim/neovim/master/runtime/doc/lua.txt)) — plugin authors fight this with `vim.schedule` boilerplate. Pure Lua compute itself is fast (LuaJIT), which is why editor interaction survives — but that speed buys nothing for *host-mediated* operations, which serialize through the editor's event loop.

### 4.2 Security
Covered in §2: no capability system, ambient I/O via `vim.uv`/FFI, install hooks as code execution, trust-by-prompt. The model is "plugins are as trusted as the user's own config" — defensible for a personal editor, catastrophic as a template for software that *distributes* plugins.

### 4.3 API stability
Neovim is pre-1.0 and the breaking-change record is a permanent treadmill:
- `news-0.11` has a BREAKING CHANGES section covering API (`vim.rpcnotify(0)` semantics changed), diagnostics handler behavior, sign placement, UI event shapes ([news-0.11.txt:14-80](https://raw.githubusercontent.com/neovim/neovim/master/runtime/doc/news-0.11.txt)).
- `news-0.12` removes `vim.diagnostic.disable()` and the legacy `vim.diagnostic.enable()` signature — items *deprecated in 0.10 and removed in 0.12* — plus `sign_define()` configuration paths and a Windows PATH security change ([news-0.12.txt](https://raw.githubusercontent.com/neovim/neovim/master/runtime/doc/news-0.12.txt)).
- `deprecated.txt` already contains a **"DEPRECATED IN 0.13"** section (`nvim_win_set_height`, `nvim_win_set_width`, `vim.F.*` renames) ([deprecated.txt](https://raw.githubusercontent.com/neovim/neovim/master/runtime/doc/deprecated.txt)).
- Meanwhile the *language* is frozen: "Lua 5.1 is the permanent interface" ([lua.txt:31](https://raw.githubusercontent.com/neovim/neovim/master/runtime/doc/lua.txt)) — the embedding pinned an interpreter version from 2006 and cannot move without breaking thousands of plugins. Embedding created a de-facto API freeze in both directions.

## 5. Does SpecForge's use case face the same issues?

SpecForge's workload: declarative manifests + `validate__*` rules + graph-snapshot compiler passes + collectors, against host functions `specforge.query_graph` / `emit_diagnostic` / `emit_file` / `http_get`, with requirements R-1 (no first-party tier), R-2 (sandboxable untrusted plugins), R-3 (single binary), R-4 (signed registry), R-5 (hot reload), R-6 (deterministic snapshot-testable analyze) ([.plugin/decision-brief.md](../../decision-brief.md); [RES-21e §5-7](../../spec/research/lua/RES-21e-specforge-architecture-constraints.md)).

| Neovim problem | Transfers to SpecForge? |
|---|---|
| **Runtime performance** | **No — this is the one axis where Lua wins.** SpecForge is a batch compiler; cold interpreter init is milliseconds (the RES-21 panel estimated ~5-10 ms for mlua, [RES-21:47](../../spec/research/lua/RES-21-plugin-runtime-decision.md)), far cheaper than a fresh Wasmtime instance per CLI run, and comparable to Extism's warm-pool LSP path. LuaJIT compute speed would make the `@specforge/formal` passes fast. |
| **Sandboxing** | **Yes — fully, and this is disqualifying.** Neovim's own history proves that "embed Lua" converges on *trusted-code* plugins: 13 years in, its only protections are a modeline guard that is "not guaranteed 100% secure" and a consent prompt. Lua has no capability system; with mlua you can strip `os`/`io`/`require`, but the precedent (vim.uv + FFI + install hooks) shows the gravity. R-2 + R-4 require the exact guarantee Neovim never offered: untrusted third-party plugins with no ambient FS/network/process access. R-1 then forces the four builtins through that same sandbox — doable in Lua (host-mediated everything), but you have re-invented Wasm's boundary with weaker enforcement. |
| **API stability** | **Yes — and it compounds.** Embedding froze Neovim to Lua 5.1 forever and produced a 0.10→0.13 churn of deprecations. SpecForge would pin mlua + a Lua host-API surface; every future `specforge.*` host function becomes a public contract that AI-authored community plugins depend on, while R-6 demands deterministic, snapshot-testable behavior across interpreter upgrades. Neovim absorbs this with a full-time core team; SpecForge's plugin API is a side effect of the compiler. |
| **Ecosystem gravity** | **Yes.** Thousands of Lua plugins formed *because* Neovim made Lua ambient and unrestricted. The same gravity would push SpecForge's AI-agent authors toward raw Lua idioms (`pcall`, globals, metatables, subtle error handling) that hurt deterministic diagnostics — and toward demanding the ambient I/O capabilities R-2 forbids. |

**Net:** Neovim validates Lua as a *fast* embedding and as an *ergonomic* authoring surface for a trusted-user tool. It is a counter-example for exactly the properties SpecForge's requirements encode: uniform sandboxed plugins, signed registry distribution, and a stable deterministic host contract. Adopting Lua would not replicate Neovim's problems by accident — it would replicate them by the same mechanism.

## Bottom line

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** Neovim — the biggest "embed Lua for plugins" deployment in existence — demonstrates that a scripting-language plugin surface inevitably converges on trusted, unsandboxed plugins and a permanent API-freeze/deprecation treadmill, which directly violates SpecForge's R-2 sandboxability, R-4 signed-distribution, and R-6 determinism requirements; Lua's genuine speed advantage does not outweigh that.
