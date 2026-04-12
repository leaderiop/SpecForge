# Phase 2: CLI Wiring

**Status**: NOT STARTED
**Depends on**: Phase 0, Phase 1, Phase 4 (manifests exposed)
**Crate**: `specforge-cli`

---

## Goal

Wire `specforge outline` as a CLI command with `--format` and `--fields` flags, following the same dispatch pattern as `specforge model`.

---

## Checklist

### 2.1: Create CLI handler (`crates/specforge-cli/src/outline.rs`)

- [ ] Create `crates/specforge-cli/src/outline.rs`
- [ ] `pub fn run(path: &Path, format: &str, fields: &str) -> i32`
- [ ] Call `pipeline::compile(path)` to get `CompilationContext`
- [ ] Extract manifests from context (see Phase 4 for how they're exposed)
- [ ] Build `OutlineIntermediate::from_manifests(&manifests)`
- [ ] Parse format string → `OutlineFormat` (markdown/mermaid/dot/json, default markdown)
- [ ] Parse fields string → `OutlineDetail` (none/keys/all, default keys)
- [ ] Call `render(outline, options)` → print to stdout
- [ ] Return 0 on success, 1 on error

### 2.2: Add command to `main.rs`

- [ ] Add `mod outline;` to `crates/specforge-cli/src/main.rs`
- [ ] Add `Outline` variant to `Commands` enum:
  ```rust
  Outline {
      #[arg(long, default_value = ".")]
      path: PathBuf,
      #[arg(long, default_value = "markdown")]
      format: String,
      #[arg(long, default_value = "keys")]
      fields: String,
  }
  ```
- [ ] Add match arm dispatching to `outline::run(&path, &format, &fields)`

---

## Pattern Reference

Follow the exact pattern from `specforge model` CLI wiring:

```
main.rs:  Commands::Model { path, format, ... } => model::run(&path, &format, ...)
model.rs: pipeline::compile(path) → generate_schema() → from_schema() → render()
```

For outline:
```
main.rs:  Commands::Outline { path, format, fields } => outline::run(&path, &format, &fields)
outline.rs: pipeline::compile(path) → get manifests → from_manifests() → render()
```

---

## Verify

```bash
cargo build -p specforge-cli  # compiles
specforge outline  # produces Markdown
specforge outline --format=mermaid  # produces graph TD
specforge outline --format=dot  # produces DOT
specforge outline --format=json  # produces JSON
specforge outline --fields=none  # overview only
specforge outline --fields=all  # full field attribution
```
