# Phase 5: Tests

**Status**: NOT STARTED
**Depends on**: Phase 0-4 (all implementation complete)
**Crates**: `specforge-emitter`, `specforge-cli`, `specforge-mcp`

---

## Goal

Comprehensive test coverage for the outline feature: builder logic, renderer snapshots, CLI E2E, MCP tool contracts.

---

## Checklist

### 5.1: Emitter unit tests (`crates/specforge-emitter/tests/outline.rs`)

- [ ] Create `crates/specforge-emitter/tests/outline.rs`
- [ ] Register in `crates/specforge-emitter/tests/main.rs` (`mod outline;`)
- [ ] **Builder tests**:
  - [ ] Empty manifests → empty OutlineIntermediate
  - [ ] Single extension → correct entity/edge/rule counts
  - [ ] Multiple extensions with peer_dependencies → OutlineDependency populated
  - [ ] Optional peer_dependency → `optional: true`
  - [ ] Entity enhancements → OutlineEnhancement with field names + count
  - [ ] Cross-extension edge detection → OutlineCrossEdge populated
  - [ ] Surface counts → cli_commands, mcp_tools, mcp_resources counted
  - [ ] Contributes flags mapped correctly
- [ ] **Snapshot tests** (insta):
  - [ ] Markdown at Detail=None
  - [ ] Markdown at Detail=Keys
  - [ ] Markdown at Detail=All
  - [ ] Mermaid at Detail=Keys
  - [ ] DOT at Detail=Keys
  - [ ] JSON at Detail=Keys
  - [ ] JSON at Detail=All

### 5.2: CLI E2E tests (`crates/specforge-cli/tests/outline.rs`)

- [ ] Create `crates/specforge-cli/tests/outline.rs`
- [ ] Default format (markdown) produces output containing "Extension Architecture"
- [ ] `--format=mermaid` produces output containing "graph TD"
- [ ] `--format=dot` produces output containing "digraph"
- [ ] `--format=json` produces valid JSON
- [ ] `--fields=none` produces overview only (no entity detail)
- [ ] `--fields=all` produces field attribution

### 5.3: MCP tool tests (`crates/specforge-mcp/tests/tools_outline.rs`)

- [ ] Tool `specforge.outline_extensions` appears in tool list
- [ ] Default invocation returns Markdown
- [ ] `format: "json"` returns valid JSON
- [ ] Invalid format returns error

---

## Verify

```bash
cargo test --workspace  # all pass, zero regressions
cargo clippy --workspace  # zero new warnings
```
