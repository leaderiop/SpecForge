# Plan: Configurable Naming Conventions

## Summary

Make entity naming conventions user-configurable via the `spec` block instead of hardcoded per entity kind. The DSL syntax uses `naming "default"` as a global setting with optional `naming:<entity>` per-kind overrides. Current hardcoded defaults (snake_case for most, PascalCase for type/port) remain when no `naming` is specified.

## DSL Syntax

```spec
spec "my-project" {
  version "1.0"
  naming "snake_case"              # global default
  naming:type "PascalCase"         # per-entity override
  naming:port "PascalCase"         # per-entity override
}
```

Available convention values: `"snake_case"`, `"PascalCase"`, `"camelCase"`, `"SCREAMING_SNAKE"`, `"any"`.

`"any"` disables W014 for the affected entity kinds — any valid identifier (letters, digits, underscores) is accepted.

## Changes

### Part 1: Expand `NamingConvention` enum (`entity_id.rs`)

Add three new variants to the existing enum:

```rust
pub enum NamingConvention {
    SnakeCase,
    PascalCase,
    CamelCase,       // NEW
    ScreamingSnake,  // NEW
    Any,             // NEW — disables convention check
}
```

Add a `from_str_opt(s: &str) -> Option<Self>` parser method.

Add `is_valid_camel_case()` and `is_valid_screaming_snake()` static methods on `EntityId`.

Update `validate_convention()` to handle all 5 variants (returns `(convention, true)` for `Any`).

### Part 2: Add `NamingConfig` to `CompilerConfig` (`config.rs`)

```rust
pub struct NamingConfig {
    pub default: NamingConvention,
    pub overrides: HashMap<EntityKind, NamingConvention>,
}
```

Add a `naming: NamingConfig` field to `CompilerConfig`. Default value: `NamingConfig { default: SnakeCase, overrides: { TypeDef: PascalCase, Port: PascalCase } }` — preserves current behavior.

Add `pub fn convention_for(&self, kind: EntityKind) -> NamingConvention` method on `NamingConfig` that checks overrides first, then falls back to default.

### Part 3: Update `EntityKind::naming_convention()` → deprecate

`EntityKind::naming_convention()` becomes a hardcoded default that `NamingConfig::default()` mirrors. The validator will read from config instead of calling `kind.naming_convention()` directly. The method stays (it's the default) but the validator no longer calls it.

### Part 4: Parse `naming` / `naming:<entity>` in the resolver (`linker.rs`)

In `config_from_spec_entity()`, handle two new field patterns:
- `"naming"` → `FieldValue::String(s)` or `FieldValue::Enum(s)` → `NamingConvention::from_str_opt(s)` → sets `naming_config.default`
- `"naming:<entity>"` → same parsing → `naming_config.overrides.insert(EntityKind::from_keyword(entity), convention)`

No grammar changes needed — both are regular `key_value` fields already supported by the parser.

### Part 5: Update validator (`passes.rs`)

- Change `check_naming_conventions(files, bag)` → `check_naming_conventions(files, config, bag)`
- Replace `EntityId::validate_convention(name, entity.kind)` with lookup from `config.naming.convention_for(entity.kind)`
- When convention is `Any`: skip E014/W014 entirely (only check E013 reserved words and W013 vague names)
- For `CamelCase` and `ScreamingSnake`, use the new validation methods
- Update the call site in `validate()` to pass `config`

### Part 6: Update W014 tests (`passes.rs`, integration test, fixtures)

- Update existing W014 unit tests to pass config with convention overrides
- Add new tests:
  - `w014_disabled_by_any_convention` — config with `naming: "any"`, no W014
  - `w014_camel_case_convention` — config requiring camelCase, snake_case name triggers W014
  - `w014_screaming_snake_convention` — same pattern
  - `w014_per_entity_override` — global snake_case but type overridden to PascalCase
- Update integration test fixture `tests/fixtures/invalid/w014/` to include a `specforge.spec` with default naming so W014 still fires
- Add `tests/fixtures/valid/naming_any/` — spec with `naming "any"`, PascalCase behavior → no warning

### Part 7: Documentation updates

- `docs/entity-model.md`: Update Naming Conventions section to document the config syntax and all 5 options
- `docs/entities/*.md`: Note that convention is configurable (type.md, port.md mention PascalCase as default)
- `.claude/skills/specforge-domain/SKILL.md`: Update naming rules to mention config
- `.claude/skills/specforge-authoring/SKILL.md`: Update naming convention table
- `.claude/skills/specforge-spec-block/SKILL.md`: Document `naming` and `naming:<entity>` fields
- `MEMORY.md`: Update Entity Naming section

## Files Modified

| File | Change |
|------|--------|
| `crates/specforge-common/src/entity_id.rs` | Add CamelCase/ScreamingSnake/Any variants, validation methods, `from_str_opt` |
| `crates/specforge-common/src/config.rs` | Add `NamingConfig` struct + field on `CompilerConfig` |
| `crates/specforge-common/src/entity_kind.rs` | No changes (keep `naming_convention()` as hardcoded default) |
| `crates/specforge-common/src/lib.rs` | Re-export `NamingConfig` if needed |
| `crates/specforge-resolver/src/linker.rs` | Parse `naming` / `naming:<entity>` fields |
| `crates/specforge-validator/src/passes.rs` | Use config for convention check, new tests |
| `crates/specforge-cli/tests/integration_test.rs` | Update W014 test, add naming_any test |
| `tests/fixtures/invalid/w014/` | Update fixture to include spec with naming config |
| `tests/fixtures/valid/naming_any/` | New fixture: `naming "any"` |
| docs + skills + MEMORY.md | Documentation updates |

## NOT Changing

- Grammar (`grammar.js`) — `naming "snake_case"` and `naming:type "PascalCase"` are already valid `key_value` syntax
- `NamingStyle` in `config.rs` — that's for codegen output, separate concern
- `diagnostic.rs` — W014 stays, just becomes config-aware
- Total validation code count stays at 39

## Verification

```bash
cargo test
cargo test -p specforge-cli -- self_validation
grep -rn "naming_convention\b" crates/specforge-validator/  # should not call kind.naming_convention() directly
```
