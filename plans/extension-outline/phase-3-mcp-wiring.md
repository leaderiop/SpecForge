# Phase 3: MCP Wiring

**Status**: NOT STARTED
**Depends on**: Phase 0, Phase 1, Phase 4 (manifests exposed)
**Crate**: `specforge-mcp`

---

## Goal

Add `specforge.outline_extensions` MCP tool that renders the extension hierarchy. Follows the same pattern as `specforge.model`.

**Naming note**: `specforge.outline` is already taken (file-level entity outline in `tools/outline.rs`). The new tool is `specforge.outline_extensions`.

---

## Checklist

### 3.1: Create MCP tool handler

- [ ] Create `crates/specforge-mcp/src/tools/outline_extensions.rs`
- [ ] `pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse`
- [ ] Parse args: `format` (string, default "markdown"), `fields` (string, default "keys")
- [ ] Extract manifests from `state` (see Phase 4 for how they're exposed)
- [ ] Build `OutlineIntermediate::from_manifests(&manifests)`
- [ ] Parse format/fields → `OutlineOptions`
- [ ] Call `render(outline, options)` → wrap in success response

### 3.2: Register in tools dispatch

- [ ] Add `mod outline_extensions;` to `crates/specforge-mcp/src/tools/mod.rs`
- [ ] Add dispatch: `"specforge.outline_extensions" => outline_extensions::call(state, arguments, id)`

### 3.3: Add tool descriptor to registry

- [ ] Add `McpToolDescriptor` to `default_tools()` in `crates/specforge-mcp/src/registry.rs`
- [ ] Tool name: `"specforge.outline_extensions"`
- [ ] Description: `"Render the extension architecture — dependencies, enhancements, contributions, and field attribution"`
- [ ] Input schema:
  ```json
  {
    "type": "object",
    "properties": {
      "format": {
        "type": "string",
        "enum": ["markdown", "mermaid", "dot", "json"],
        "default": "markdown",
        "description": "Output format"
      },
      "fields": {
        "type": "string",
        "enum": ["none", "keys", "all"],
        "default": "keys",
        "description": "Detail level: none (counts only), keys (names), all (full field attribution)"
      }
    }
  }
  ```

---

## Pattern Reference

Follow `specforge.model` tool pattern in:
- `crates/specforge-mcp/src/tools/model.rs` — handler
- `crates/specforge-mcp/src/tools/mod.rs` — dispatch (line ~49)
- `crates/specforge-mcp/src/registry.rs` — descriptor (lines ~199-240)

---

## Verify

```bash
cargo test -p specforge-mcp  # tool appears in list
# MCP tool invocation returns Markdown
# format=json returns valid JSON
```
