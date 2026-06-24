# PRD-007: Extension-Federated Inference Prompt

## Summary

An MCP prompt (`specforge://prompts/infer`) that agents (Claude Code / Cursor) call to get structured guidance for inferring spec entities from code. SpecForge doesn't do the inference — it provides the agent with domain-specific hints about what to look for.

## Design Decisions

| Decision | Choice |
|----------|--------|
| Delivery mechanism | MCP prompt (not CLI command) |
| Who does inference | The agent (Claude Code / Cursor), not SpecForge |
| Extension contribution | `inference_guide: Option<String>` on `EntityKindDescriptor` — freeform prose |
| Project overrides | `"inference"` key in `specforge.json` with per-kind + `"global"` |
| Merge semantics | Append (extension default + "\n\n**Project-specific:**\n" + override) |
| Scope parameter | `scope`: unset (overview), `kind:{name}` (focused), `file:{path}` (dedup context) |
| Entity listing | Counts in overview, full IDs in scoped mode |
| Examples | One concrete `.spec` example per kind in scoped response |
| Validation loop | Prompt instructs agent to call `specforge_validate` after writing |
| Formatting | Not included in workflow |
| State access | Add `ProjectConfig` to `McpState`, load at init |
| Graph freshness | Always reads current state, no caching |
| Guide length | No constraint |

## Agent Workflow

1. Agent calls `specforge://prompts/infer` (no scope) → gets overview of all installed extensions, entity kind counts, inference guides
2. Agent scans the codebase, identifies candidates
3. Agent calls `specforge://prompts/infer?scope=kind:behavior` → gets focused guidance, full ID list, concrete example
4. Agent writes `.spec` files
5. Agent calls `specforge_validate` to check correctness
6. Agent fixes errors, repeats

## Scope Behavior

### No scope (overview)
- Returns all installed extensions and their kinds
- Entity counts per kind (not full IDs)
- All inference guides (extension default + project override appended)
- Global project conventions
- One generic syntax example
- Instruction to call `specforge_validate` after writing

### `scope=kind:{name}`
- Returns focused guide for one kind
- Full list of existing entity IDs of that kind
- One concrete `.spec` example for that kind
- Field descriptions with required markers

### `scope=file:{path}`
- Returns all guides (same as no scope)
- Plus: existing entities that reference the given file path
- Value is deduplication context, not code analysis

## Implementation Steps

1. Add `inference_guide: Option<String>` to `EntityKindDescriptor` in `crates/specforge-wasm/src/protocol/types.rs`
2. Add `inference_guide: Option<String>` to `ManifestEntityKind` in `crates/specforge-registry/src/manifest/types.rs`
3. Write inference guides for all 22 entity kinds (software: 5, product: 9, governance: 3, formal: 5)
4. Add `inference` parsing to `ProjectConfig` in `crates/specforge-common/src/project.rs`
5. Add `project_config: ProjectConfig` to `McpState` in `crates/specforge-mcp/src/state.rs`
6. Implement `crates/specforge-mcp/src/prompts/infer.rs` with scope handling
7. Register prompt in `crates/specforge-mcp/src/prompts/mod.rs` and `crates/specforge-mcp/src/registry.rs`
