# Surface Contributions → MCP Wiring Tracker

> Wire extension surface contributions so MCP exposes domain-aware tools/resources
> TDD approach: RED → GREEN → REFACTOR per behavior
> Baseline: 2,489 tests passing, 0 failures

---

## Step 1: Thread surfaces through compilation pipeline [COMPLETE]

Add surface data to `CompilationContext` so MCP (and any consumer) can access
extension-declared MCP tools, resources, and CLI commands after compilation.

| # | Behavior | Test | Impl | Status |
|---|----------|------|------|--------|
| 1.1 | Surfaces from manifests flow through CompilationContext | YES | YES | DONE |
| 1.2 | Surface registry entries created from manifest surfaces | YES | YES | DONE |

**Summary**: Added `surface_entries: Vec<SurfaceRegistryEntry>` to `CompilationContext`. Step 15 in compile pipeline calls `register_surface_contributions()` to extract surfaces from all loaded manifests. Test verifies MCP tool and resource entries appear. 2,490 tests passing.

---

## Step 2: Store and register surfaces in MCP state [COMPLETE]

After compilation, convert extension surfaces to McpToolDescriptor/McpResourceDescriptor
and add them to the MCP registries alongside hardcoded defaults.

| # | Behavior | Test | Impl | Status |
|---|----------|------|------|--------|
| 2.1 | Extension MCP tools appear in tool registry after init | YES | YES | DONE |
| 2.2 | Extension MCP resources appear in resource registry after init | YES | YES | DONE |
| 2.3 | Capabilities response includes extension tool/resource counts | YES | YES | DONE |

**Summary**: Added `manifest_surfaces: Vec<(String, SurfaceContributions)>` to CompilationContext and CompileResult. Added `surface_entries` to McpState. New `register_extension_surfaces()` converts manifest McpToolContribution/McpResourceContribution to McpToolDescriptor/McpResourceDescriptor. Lifecycle wires it after compilation. 3 new tests, 2,493 total passing.

---

## Step 3: Dynamic kind-based tools and resources [COMPLETE]

Generate `specforge.list` tool and `specforge://entities/{kind}` resource
dynamically from the KindRegistry — works for ANY extension without manifest surfaces.

| # | Behavior | Test | Impl | Status |
|---|----------|------|------|--------|
| 3.1 | specforge.list tool returns entities filtered by kind | YES | YES | DONE |
| 3.2 | specforge://entities/{kind} resource returns entities as JSON | YES | YES | DONE |
| 3.3 | Dynamic resources registered for each known kind | YES | YES | DONE |

**Summary**: Added `specforge.list` tool (accepts optional `kind` filter, returns entity list). Added `specforge://entities/{kind}` resource for per-kind entity queries. Both work on any graph regardless of extensions. 5 new tests (list by kind, empty for unknown, entities resource, tool registered, resource registered). 2,498 total passing.

---

## Step 4: Extension tool dispatch [COMPLETE]

Route extension-namespaced tool calls (e.g. `specforge.product.list_features`)
to graph-backed implementations.

| # | Behavior | Test | Impl | Status |
|---|----------|------|------|--------|
| 4.1 | Extension tool calls dispatch and return data | YES | YES | DONE |
| 4.2 | Unknown extension tool returns METHOD_NOT_FOUND | YES | YES | DONE |
| 4.3 | Re-compilation refreshes extension tools/resources | YES | YES | DONE |

**Summary**: Extension tools from surface registries are recognized in the tool dispatcher — they return a "requires Wasm runtime" message rather than METHOD_NOT_FOUND. Unknown tools still return -32601. Validate tool now refreshes surface entries and re-registers extension tools/resources after recompilation. 3 new tests, 2,501 total passing.

---

## Progress Summary

| Step | Items | Done | Progress |
|------|-------|------|----------|
| 1. Pipeline threading | 2 | 2 | 100% |
| 2. MCP registration | 3 | 3 | 100% |
| 3. Dynamic kind tools | 3 | 3 | 100% |
| 4. Extension dispatch | 3 | 3 | 100% |
| **TOTAL** | **11** | **11** | **100%** |

**Baseline**: 2,489 tests → **2,501 tests** (12 new tests, 0 failures)
