# @specforge/typescript Scanner — Implementation Plan

## Overview

Ship `@specforge/typescript` as the second language intelligence extension, following the exact pattern established by `@specforge/rust`. This covers `.ts`, `.tsx`, `.js`, and `.jsx` files. As part of this work, refactor the hardcoded scanner dispatch in `infer_gaps.rs` and `infer_status.rs` into a generic multi-scanner loop driven by `analyzer_contributions` from loaded manifests.

## Phase 1: Generic Scanner Dispatch (prerequisite)

**Problem:** `infer_gaps.rs` and `infer_status.rs` currently hardcode `RustExtension` and `file_path.ends_with(".rs")`. Adding a second scanner without fixing this would double the hardcoding.

**Solution:** Replace hardcoded scanner dispatch with a generic loop that:
1. Collects all `analyzer_contributions` from loaded manifests
2. For each source file, finds the matching analyzer by file extension
3. Invokes the correct scanner via `BuiltinRuntime::call_export(extension_name, scan_export, input)`

### Tasks

#### 1a. Create `ScannerDispatch` helper in `specforge-emitter/src/scanner_dispatch.rs`

New public module with a single function:

```rust
pub fn scan_source_files(
    runtime: &BuiltinRuntime,
    manifests: &[ManifestV2],
    project_root: &Path,
    source_files: &[String],
) -> (Vec<SourceItem>, Vec<String>)
```

This function:
- Builds a lookup table: `file_extension → (extension_name, scan_export)`
  from all `analyzer_contributions` across all manifests
- For each `source_file`, finds the matching analyzer by extension
- Reads the file content, builds a `ScanRequest`, calls `runtime.call_export(ext_name, scan_export, &input)`
- Deserializes `ScanResponse`, converts each `ScannedItem` → `SourceItem` with `scanner: Some(extension_name)`
- Returns `(all_items, scanners_used)`

#### 1b. Update `infer_gaps.rs` (MCP tool)

Replace the hardcoded `RustExtension` block with:
- Instantiate `default_runtime()` from `specforge_emitter::builtins`
- Call `scan_source_files(&runtime, &state.manifests, &project_root, &source_files)`
- Pass results to `compute_gap_report`

#### 1c. Update `infer_status.rs` (CLI)

Same refactor — use `default_runtime()` + `scan_source_files()` instead of hardcoded `RustExtension`.

#### 1d. Test: multi-scanner dispatch

Integration test in `specforge-emitter/tests/scanner_dispatch.rs`:
- Create a `BuiltinRuntime` with `@specforge/rust` registered
- Create temp directory with `.rs` and `.txt` files
- Verify `scan_source_files` only scans `.rs` files
- Verify returned `scanners_used` contains `"@specforge/rust"`
- Verify returned items have correct `scanner` field

#### 1e. Verify existing tests still pass

`cargo test --workspace` — 0 failures.

---

## Phase 2: Implement `@specforge/typescript` Extension

### Scope

TypeScript/JavaScript exported symbols. Analogous to Rust's `pub` items, these are:

| TS/JS Pattern | `item_kind` | Notes |
|---|---|---|
| `export function name(...)` | `"function"` | Named function exports |
| `export async function name(...)` | `"function"` | Async function exports |
| `export class Name` | `"class"` | Class exports |
| `export interface Name` | `"interface"` | Interface declarations |
| `export type Name` | `"type_alias"` | Type alias exports |
| `export enum Name` | `"enum"` | Enum exports |
| `export const name` | `"constant"` | Const exports |
| `export let name` / `export var name` | `"variable"` | Variable exports |
| `export default function name(...)` | `"function"` | Default export with name |
| `export default class Name` | `"class"` | Default export with name |

**Not scanned** (too noisy or unexportable):
- Re-exports (`export { X } from './other'`) — these are structural, not definitions
- `export default <expression>` without a name — no symbol to anchor
- `module.exports = ...` (CommonJS) — out of scope for v1
- Namespace merging, declaration merging — too complex for regex

### Tasks

#### 2a. Create `specforge-emitter/src/builtins/typescript.rs`

Implement `TypeScriptExtension` struct implementing `BuiltinExtension`:

**`handshake()`:**
```rust
HandshakeResponse {
    protocol_version: "1.0.0",
    name: "@specforge/typescript",
    version: "1.0.0",
    contribution_flags: ContributionFlags { analyzers: true, ..Default::default() },
    peer_dependencies: vec![],
    sandbox_policy: Some(SandboxPolicy {
        network_access: false,
        file_system_access: true,
        max_memory_mb: 512,
        max_execution_ms: 30000,
    }),
}
```

**`describe("analyzers")`:**
```rust
vec![AnalyzerDescriptor {
    language: "typescript",
    file_extensions: vec![".ts", ".tsx", ".js", ".jsx"],
    excluded_dirs: vec!["node_modules", "dist", "build", ".next", ".nuxt"],
    scan_export: "scan__typescript",
    classify_export: "classify__typescript",
    map_export: "map__typescript",
    description: Some("Scans TypeScript/JavaScript source files for exported symbols"),
}]
```

**`call_analyzer()`:**
Routes `scan__typescript`, `classify__typescript`, `map__typescript` to the three internal functions.

#### 2b. Implement `scan_typescript(req: &ScanRequest) -> ScanResponse`

Regex-based scanner for exported symbols. Pattern list (order matters — longest prefix first):

```
"export default async function " → "function"
"export default function "       → "function"
"export default class "          → "class"
"export async function "         → "function"
"export function "               → "function"
"export class "                  → "class"
"export abstract class "         → "class"
"export interface "              → "interface"
"export type "                   → "type_alias"
"export enum "                   → "enum"
"export const enum "             → "enum"
"export const "                  → "constant"
"export let "                    → "variable"
"export var "                    → "variable"
```

Name extraction: after the prefix, consume `[a-zA-Z_$][a-zA-Z0-9_$]*`.

Skip lines:
- Starting with `//` or `/*` or `*` (comments)
- Starting with `export {` (re-exports)
- Starting with `export *` (barrel exports)
- Starting with `export default` NOT followed by `function`/`class` (anonymous defaults)

Signature extraction: same logic as Rust — everything up to `{` or end of line.

TSX/JSX: no special handling needed — the exported symbol patterns are identical to TS/JS. The JSX inside function bodies doesn't affect the top-level scan.

#### 2c. Implement `classify_typescript(req: &ClassifyRequest) -> ClassifyResponse`

Heuristic classification mapping TS/JS items to SpecForge entity kinds:

| `item_kind` | Name pattern | `suggested_entity_kind` | `confidence` |
|---|---|---|---|
| `"function"` | `handle*`, `process*`, `create*`, `update*`, `delete*`, `get*`, `*Handler` | `"behavior"` | 0.8 |
| `"function"` | (other) | `"behavior"` | 0.5 |
| `"class"` | `*Error`, `*Event`, `*Message` | `"event"` | 0.7 |
| `"class"` | `*Service`, `*Client`, `*Port`, `*Repository`, `*Gateway` | `"port"` | 0.7 |
| `"class"` | (other) | `"type"` | 0.6 |
| `"interface"` | `I*Service`, `I*Port`, `I*Repository`, `*Port`, `*Service`, `*Gateway` | `"port"` | 0.8 |
| `"interface"` | (other) | `"type"` | 0.6 |
| `"type_alias"` | (any) | `"type"` | 0.7 |
| `"enum"` | (any) | `"type"` | 0.7 |
| `"constant"` | (any) | `None` | 0.3 |
| `"variable"` | (any) | `None` | 0.3 |

Skip test files: paths containing `/tests/`, `/test/`, `/__tests__/`, `/__mocks__/`, or ending with `.test.ts`, `.test.tsx`, `.test.js`, `.test.jsx`, `.spec.ts`, `.spec.tsx`, `.spec.js`, `.spec.jsx`.

#### 2d. Implement `map_typescript(req: &MapSymbolRequest) -> MapSymbolResponse`

TypeScript naming conventions differ from Rust:
- Functions/variables are already camelCase → convert to snake_case for entity IDs
- Classes/interfaces are PascalCase → convert to snake_case for entity IDs
- Same matching logic as Rust: check `exact_snake_case` first, then `exact_original`, then `generated_snake_case`

The `to_snake_case` function can be shared or duplicated (it's the same logic — insert `_` before uppercase, lowercase everything). Since the Rust extension already has a private copy, duplicate it in the TypeScript module (6 lines, not worth a shared dependency).

#### 2e. Register in `specforge-emitter/src/builtins/mod.rs`

```rust
mod typescript;
pub use typescript::TypeScriptExtension;

// In default_runtime():
.with_extension("@specforge/typescript", Box::new(TypeScriptExtension))
```

---

## Phase 3: Tests

#### 3a. Unit tests in `typescript.rs` (mirror `rust.rs` test structure)

1. **`scan_finds_exported_functions`** — `export function hello()` → function at line 1
2. **`scan_finds_exported_classes`** — `export class MyService {}` → class
3. **`scan_finds_exported_interfaces`** — `export interface UserPort {}` → interface
4. **`scan_finds_exported_types`** — `export type Result = ...` → type_alias
5. **`scan_finds_exported_enums`** — `export enum Status {}` → enum
6. **`scan_finds_async_functions`** — `export async function fetchData()` → function
7. **`scan_finds_default_exports`** — `export default function main()` → function
8. **`scan_finds_abstract_classes`** — `export abstract class Base {}` → class
9. **`scan_finds_const_enum`** — `export const enum Direction {}` → enum
10. **`scan_skips_comments`** — `// export function fake()` → 0 items
11. **`scan_skips_reexports`** — `export { X } from './other'` → 0 items
12. **`scan_skips_barrel_exports`** — `export * from './module'` → 0 items
13. **`scan_skips_anonymous_default`** — `export default () => {}` → 0 items
14. **`scan_captures_signature`** — verify signature up to `{`
15. **`classify_handler_as_behavior`** — `handleLogin` → behavior (0.8)
16. **`classify_interface_as_port`** — `UserRepository` interface → port (0.8)
17. **`classify_class_as_type`** — `Config` class → type (0.6)
18. **`classify_skips_test_files`** — `*.test.ts` → None
19. **`classify_skips_spec_files`** — `*.spec.ts` → None
20. **`map_matches_existing_snake_case`** — `MyService` + existing `my_service` → exact_snake_case
21. **`map_generates_snake_case`** — `ConfigLoader` + no match → generated_snake_case
22. **`handshake_declares_analyzer`** — verify flags
23. **`describe_returns_analyzer_descriptor`** — verify language, extensions, exports

#### 3b. Protocol round-trip test in `specforge-emitter/tests/builtins.rs`

```rust
#[test]
fn typescript_extension_loads_via_protocol() {
    let manifest = load_via_protocol("@specforge/typescript", Box::new(TypeScriptExtension));
    assert_eq!(manifest.name, "@specforge/typescript");
    assert_eq!(manifest.version, "1.0.0");
    assert!(manifest.contributes.analyzers);
    assert!(!manifest.contributes.entities);
    assert_eq!(manifest.analyzer_contributions.len(), 1);
    let ac = &manifest.analyzer_contributions[0];
    assert_eq!(ac.language, "typescript");
    assert_eq!(ac.file_extensions, vec![".ts", ".tsx", ".js", ".jsx"]);
    assert_eq!(ac.scan_export, "scan__typescript");

    let diags = validate_manifest(&manifest);
    assert!(diags.is_empty(), "schema validation errors: {:?}", diags);
}
```

#### 3c. Scanner integration test in `specforge-emitter/tests/builtins.rs`

```rust
#[test]
fn typescript_scanner_finds_exported_symbols() {
    // Multi-pattern source covering all item_kinds
    // Verify count, names, kinds, line numbers
}
```

#### 3d. Multi-scanner dispatch integration test

Test that `scan_source_files` with both `@specforge/rust` and `@specforge/typescript` registered correctly scans a mixed project with both `.rs` and `.ts` files, each file going to the right scanner.

#### 3e. `cargo test --workspace` — 0 failures

---

## Phase 4: Update MCP Test Infrastructure

#### 4a. Update `tools_inference.rs` test helper

Add `@specforge/typescript` manifest to `rust_manifest()` (rename to `analyzer_manifests()` or add a second manifest):

```rust
fn analyzer_manifests() -> Vec<ManifestV2> {
    vec![rust_manifest(), typescript_manifest()]
}
```

#### 4b. Add inference test with TypeScript source files

Create test that sets up a project with both `.rs` and `.ts` source files, verifies that `infer_progress` discovers both, and `infer_gaps` scans both with appropriate scanners.

---

## Implementation Order (TDD vertical slices)

```
1a → 1b → 1c → 1d → 1e        (generic dispatch)
2a → 2b → 3a[1-14] → 2c → 3a[15-19] → 2d → 3a[20-23] → 2e  (extension)
3b → 3c → 3d → 3e              (integration tests)
4a → 4b                         (MCP test infra)
```

Each `→` is one RED→GREEN cycle.

## Files Changed (expected)

| File | Change |
|---|---|
| `specforge-emitter/src/scanner_dispatch.rs` | **NEW** — generic multi-scanner dispatch |
| `specforge-emitter/src/lib.rs` | Add `pub mod scanner_dispatch` |
| `specforge-emitter/src/builtins/typescript.rs` | **NEW** — full extension impl |
| `specforge-emitter/src/builtins/mod.rs` | Add `mod typescript`, `pub use`, register in `default_runtime()` |
| `specforge-mcp/src/tools/infer_gaps.rs` | Use `scan_source_files()` instead of hardcoded `RustExtension` |
| `specforge-cli/src/infer_status.rs` | Use `scan_source_files()` instead of hardcoded `RustExtension` |
| `specforge-emitter/tests/builtins.rs` | Add protocol round-trip + scanner tests |
| `specforge-emitter/tests/scanner_dispatch.rs` | **NEW** — multi-scanner integration test |
| `specforge-mcp/tests/tools_inference.rs` | Update manifest helper, add TS test |
| `plans/language-intelligence/PROGRESS.md` | Update tracker |

## Non-Goals

- No AST parsing (tree-sitter for TS) — regex scanning only, same as `@specforge/rust`
- No CommonJS support (`module.exports`) — ESM `export` only
- No `declare` scanning (ambient declarations) — those are type-only, not implementations
- No JSDoc comment parsing
- No `.d.ts` scanning — declaration files are type-only
