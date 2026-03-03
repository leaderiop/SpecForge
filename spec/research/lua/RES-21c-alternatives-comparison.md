# RES-21c: Scripting Alternatives Comparison — Rhai, Starlark, Deno, Nickel

**Expert perspective:** Embedded scripting specialist for Rust applications
**Decision outcome:** ALL REJECTED — Wasm/Extism selected as unified plugin runtime (see ADR `wasm_extism_plugin_runtime`)

---

## 1. Rhai — Rust-Native Scripting

### Profile
- **Type:** Pure Rust, no FFI, no unsafe
- **Syntax:** Rust-like / JS-like hybrid
- **Stars:** ~3k
- **Binary impact:** +1MB
- **Memory:** ~500KB per engine
- **Startup:** ~1ms

### Strengths
- Zero FFI — no C linkage, pure Rust compilation
- Built-in sandboxing: no I/O, resource limits by default
- Minimal dependencies (12 transitive vs Lua's 30+)
- Registers Rust types directly via traits
- Small footprint

### Plugin Example
```rhai
fn register() {
    let entity = new_entity();
    entity.name = "epic";
    entity.fields = [
        field("title", "string", true),
        field("stories", "reference_list", "feature"),
    ];
    entity.testable = false;
    entity
}
```

### Limitations
- No classes/OOP (functions and maps only)
- No async/await (synchronous)
- Limited standard library (intentional)
- 10-50x slower than native Rust
- Smaller community than Lua

### Why Rejected
- **Single language** — plugin authors must learn Rhai, no choice
- **Interpreted, 10-50x slower** than Wasm's near-native execution
- **Software sandboxing** — not hardware-enforced like Wasm
- **No universal binary** — scripts are source-distributed, not compiled
- **Small ecosystem** — harder to find help, fewer examples

### Real-World Users
- Bevy (game scripting), nushell (shell config), SurrealDB (database functions), Veloren (NPC behavior)

---

## 2. Starlark — Deterministic Python Dialect

### Profile
- **Type:** Python dialect, Rust implementation by Meta (`starlark-rust`)
- **Syntax:** Python subset
- **Design:** Hermetic, deterministic, no side effects
- **Binary impact:** +3MB
- **Memory:** ~1MB per evaluator

### Strengths
- **Deterministic by design** — same input always produces same output
- Python-like syntax — low learning curve
- Industrial strength — powers Buck2 (Meta's entire build infrastructure)
- No I/O, no randomness, no threading by default
- Immutable data structures

### Plugin Example
```python
# epic.star
def register():
    return entity(
        name = "epic",
        fields = [
            field("title", type = "string", required = True),
            field("stories", type = "reference_list", targets = ["feature"]),
        ],
        testable = False,
    )
```

### Limitations
- No imports (single-file evaluation)
- No while loops (only for loops)
- Limited introspection
- Starlark is not Python — missing many Python features
- Smaller ecosystem than Lua

### Why Rejected
- **Single language** — Python-like but not Python, niche
- **No universal binary** — source-distributed
- **Software sandboxing** — deterministic by design but not hardware-enforced
- **Build system niche** — designed for Bazel/Buck2, not general plugin systems

### Real-World Users
- Bazel (Google), Buck2 (Meta), Pants (Twitter)

---

## 3. Deno Core / V8 — Embedded JavaScript/TypeScript

### Profile
- **Type:** V8 JavaScript engine in Rust via `deno_core`
- **Syntax:** Full JavaScript/TypeScript
- **Binary impact:** +50MB (!)
- **Memory:** 10-20MB base
- **Startup:** 50-100ms

### Strengths
- Full TypeScript support — type-safe plugins
- Massive npm ecosystem for distribution
- Excellent tooling (VS Code, debuggers)
- Near-native performance after JIT warmup
- Familiar to most developers

### Plugin Example
```typescript
export function register(): EntityManifest {
    return {
        name: "epic",
        fields: [
            { name: "title", type: "string", required: true },
            { name: "stories", type: "reference_list", targets: ["feature"] },
        ],
        testable: false,
    };
}
```

### Limitations and Why Rejected
- **+50MB binary size** — unacceptable for a CLI tool
- **50-100ms startup** — noticeable on every invocation
- **10-20MB memory** — wasteful for short-lived CLI
- Complex V8 API
- Security surface area of V8
- JS/TS authors can compile to Wasm via Javy/QuickJS instead — getting Wasm benefits without V8 overhead

---

## 4. Nickel — Typed Configuration Language

### Profile
- **Type:** Configuration language with contracts
- **Syntax:** Functional, ML-inspired
- **Rust-native**

### Strengths
- Gradual typing with contracts
- Good for the "config that becomes code" spectrum
- Merge semantics for configuration composition

### Why Rejected
- **Not a scripting language** — cannot express plugin logic
- No function registration
- No host interop API
- Tiny community
- SpecForge already has `specforge.json` for configuration

---

## 5. Full Comparison Matrix

| Dimension | Lua 5.4 | **Wasm/Extism** | Rhai | Starlark | Deno/V8 |
|-----------|---------|-----------------|------|----------|---------|
| **Status** | Rejected | **ADOPTED** | Rejected | Rejected | Rejected |
| **Learning curve** | Low | Medium | Medium | Low | Low |
| **Sandboxing** | Software | **Hardware** | Software | By design | Software |
| **Startup (ms)** | 0.5 | 10-50 (AOT: <10) | 1 | 2 | 50-100 |
| **Memory (MB)** | 0.1-1 | 1-10 | 0.5 | 1 | 10-20 |
| **Binary size** | +2MB | +5MB | +1MB | +3MB | +50MB |
| **Rust integration** | FFI | Host functions | Native | Native | API |
| **Multi-language** | No | **Yes** | No | No | JS/TS only |
| **Type safety** | Dynamic | Via WIT | Dynamic | Dynamic | TypeScript |
| **Async** | Coroutines | No | No | No | Yes |
| **Ecosystem** | Moderate | **Growing** | Small | None | Massive |
| **Deterministic** | No | **Yes** | No | Yes | No |
| **Debugging** | Print | Host logging | Print | Print | Excellent |
| **CI compatible** | Needs engine | **Universal .wasm** | Needs engine | Needs engine | Needs V8 |
| **Universal binary** | No | **Yes** | No | No | No |

---

## 6. Why Wasm Wins Over All Alternatives

The decision factors that make Wasm/Extism categorically better for SpecForge:

1. **Hardware sandboxing** — Lua, Rhai, Starlark all rely on software sandboxing. Wasm uses linear memory with hardware enforcement. For a tool consumed by AI agents in CI, this matters.
2. **Multi-language** — All scripting alternatives lock plugin authors into one language. Wasm lets them choose Rust, Go, JS, Python, AssemblyScript.
3. **Universal binary** — One `.wasm` artifact runs on Linux, macOS, Windows without recompilation. Scripts require the engine to be present.
4. **Near-native speed** — Wasm executes at near-native speed. Lua/Rhai/Starlark are 10-50x slower (interpreted).
5. **Deterministic execution** — Wasm execution is deterministic. Lua's GC introduces non-determinism. Critical for CI reproducibility.
6. **Uniform protocol** — One runtime for plugins, providers, and generators. Alternatives would require a different protocol for each.
