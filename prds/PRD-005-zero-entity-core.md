# PRD-005: Zero-Entity Core

**Status:** Draft
**Author:** Mohammad AL Mechkor
**Date:** 2026-04-12
**Depends on:** PRD-001 (Extension Protocol)

---

## Problem Statement

SpecForge's core compiler (parser, resolver, validator, emitter) still contains residual domain knowledge. While the architecture is designed around Principle 2 (zero domain knowledge in core), the implementation has not fully achieved it. Approximately 60% of the core is domain-agnostic; the remaining 40% contains:

1. **Hardcoded entity references.** Several compiler passes, validators, and emitters check for specific entity kind names like "behavior", "feature", or "invariant". These checks should be driven by extension-declared metadata, not hardcoded strings.

2. **Structural keyword leakage.** The grammar correctly handles `spec`, `ref`, `use`, `define`, and `verify` as structural keywords (not entity kinds). But some validator and emitter code makes assumptions about which keywords exist beyond the structural set, rather than consulting the `KindRegistry`.

3. **Testability remnants.** The parser has a `verify_statement` grammar rule, and several types carry `testable`/`supports_verify` fields that influence compilation behavior. With testability extracted to an extension (PRD-003), these should be removed from the core.

4. **Fixed validation logic.** Some validation rules that should be extension-declared are implemented as hardcoded checks in the core validator. The validation engine supports declarative rules and custom Wasm validators, but not all existing checks have been migrated.

The consequence: if a user creates a new domain (e.g., `@specforge/hardware` with entity kinds `component`, `signal`, `bus`), they hit edge cases where the core compiler behaves differently than expected because it was written with software/product assumptions.

## Solution

Complete the zero-entity core architecture: make the compiler a pure typed-graph engine where ALL domain vocabulary enters through extensions and the `KindRegistry`/`FieldRegistry`/`EdgeRegistry`. After this work, the core compiler compiles any `keyword name { fields }` block without knowing what the keyword means. Validation of what keywords are legal, what fields they accept, and what edges they create comes entirely from extensions.

### Target State

- **Parser**: Parses `keyword name { ... }` generically. Structural keywords (`spec`, `ref`, `use`, `define`) are grammar rules. Everything else is a `Custom(String)` entity kind resolved via `KindRegistry`.
- **Resolver**: Resolves references by consulting `FieldRegistry` for target kinds and `EdgeRegistry` for edge labels. No hardcoded entity kind names.
- **Validator**: All validation rules are either declarative (pattern-matching, declared by extensions) or custom (Wasm-backed, declared by extensions). No hardcoded validation logic in the core beyond structural checks.
- **Emitter**: Schema generation, DOT rendering, JSON export, and all other output formats derive metadata from registries. No hardcoded entity kind metadata.
- **Grammar**: The `verify_statement` rule is removed. The grammar handles `keyword name { ... }` generically.

## User Stories

1. As an extension author creating a new domain (e.g., `@specforge/hardware`), I want the core compiler to accept my custom entity kinds (`component`, `signal`, `bus`) without any compiler modifications, so that I can extend SpecForge to new domains purely through extensions.

2. As a SpecForge contributor, I want the core compiler to have zero references to "behavior", "feature", "invariant", or any other domain-specific entity kind, so that I can verify Principle 2 compliance by grepping the codebase.

3. As a spec author, I want the compiler to accept any `keyword name { ... }` block where `keyword` is registered in the `KindRegistry`, so that new entity kinds from extensions are immediately usable in spec files.

4. As an extension author, I want the compiler to reject unknown keywords (not registered in any extension) with a clear diagnostic, so that typos are caught rather than silently accepted.

5. As an extension author, I want the compiler to validate fields against `FieldRegistry` entries, so that unknown fields on an entity kind produce W020 warnings rather than being silently accepted.

6. As an extension author, I want all validation rules to be declared by extensions (not hardcoded in the core), so that my extension controls what is valid and what is not.

7. As a formal methods user, I want the `verify` grammar rule removed, so that the parser doesn't recognize verify blocks and the grammar is cleaner.

8. As a SpecForge contributor, I want the `verify_statement` production removed from the Tree-sitter grammar, so that the grammar only handles the generic `keyword name { ... }` pattern plus structural keywords.

9. As a SpecForge contributor, I want the `VerifyStatement` type removed from the parser, so that the parser's type system doesn't carry testability concepts.

10. As an extension author, I want the resolver to look up `target_kind` from `FieldRegistryEntry` when resolving references, so that cross-extension references work correctly without the resolver knowing entity kind names.

11. As an extension author, I want the DOT renderer to read `dot_shape`, `dot_color`, and `dot_fillcolor` from `KindRegistryEntry` for every entity kind, so that my custom entity kinds have correct visualization without compiler changes.

12. As an extension author, I want the LSP to read `semantic_token` and `lsp_icon` from `KindRegistryEntry` for every entity kind, so that my custom entity kinds get syntax highlighting and icons without LSP changes.

13. As a SpecForge contributor, I want the emitter to read entity descriptions from `KindRegistryEntry` rather than hardcoding them, so that the schema output is fully extension-driven.

14. As a SpecForge contributor, I want a CI check that greps for hardcoded entity kind names in the core crates, so that regressions to Principle 2 are caught automatically.

15. As an extension author, I want the `WasmEntityKind::Custom(String)` variant to be the default path for all entity kinds (not a fallback), so that the type system doesn't privilege certain kinds over others.

16. As a spec author, I want the compiler to provide fuzzy-match suggestions when I misspell an entity kind, so that the diagnostic includes "did you mean 'behavior'?" based on `KindRegistry` contents.

17. As a SpecForge contributor, I want the core crates (`specforge-parser`, `specforge-resolver`, `specforge-validator`, `specforge-emitter`, `specforge-graph`) to depend only on registry interfaces, not on extension-specific types, so that the dependency graph enforces Principle 2.

## Implementation Decisions

### Audit-Driven Approach

The implementation starts with a comprehensive audit of all core crates to catalog every hardcoded entity kind reference. Research RES-26 found the codebase is ~60% ready. The remaining ~40% is addressed through systematic elimination:

1. **Audit**: Grep core crates for hardcoded entity kind names ("behavior", "feature", "invariant", "event", "type", "port", "decision", "constraint", etc.). Catalog each reference by file, line, and category (validator, emitter, resolver, test).

2. **Migrate**: For each reference, determine whether it should:
   - Be replaced with a `KindRegistry` lookup
   - Be replaced with a `FieldRegistry` lookup
   - Be moved to an extension's validation rules
   - Be removed entirely (dead code from pre-extension era)

3. **Verify**: After each batch of changes, run the full test suite. The existing 2,600+ tests serve as regression gates.

### Grammar Changes

Remove the `verify_statement` rule from the Tree-sitter grammar (`crates/tree-sitter-specforge/grammar.js`). This requires:
- Removing the grammar rule and its test cases
- Regenerating the Tree-sitter parser C code
- Removing `VerifyStatement` from the Rust parser types
- Removing `FieldValue::VerifyList` from the field value enum
- Updating any code that handles verify blocks

### Existing Registry Infrastructure

The codebase already has the right abstractions:
- `KindRegistry` -- maps keyword -> `KindRegistryEntry` with all metadata
- `FieldRegistry` -- maps (kind, field) -> `FieldRegistryEntry` with type, edge, target
- `EdgeRegistry` -- maps label -> `EdgeRegistryEntry` with source/target kinds
- `WasmEntityKind::Custom(String)` variant exists for extension-defined kinds
- `populate_registries()` builds all three registries from manifests

The task is to make all core code USE these registries instead of hardcoding assumptions.

### CI Compliance Check

Add a CI step that greps core crates for entity kind name strings and fails if any are found outside of test fixtures. The grep pattern:

```
"behavior"|"feature"|"invariant"|"event"|"port"|"decision"|"constraint"|"failure_mode"|"property"|"axiom"|"protocol"|"refinement"|"process"|"journey"|"deliverable"|"milestone"|"module"|"term"|"persona"|"channel"|"release"
```

Exclusions: test files, snapshot files, string comparisons against registry-sourced values.

### Modules Affected

**specforge-parser**: Remove `verify_statement` grammar rule, `VerifyStatement` type, `FieldValue::VerifyList`. Update node visitors that pattern-match on entity kinds to use registry lookups instead.

**specforge-resolver**: Replace hardcoded target_kind checks with `FieldRegistry` lookups. Replace entity kind name comparisons with `KindRegistry.contains()` checks.

**specforge-validator**: Move remaining hardcoded validation checks to extension-declared rules. The validation engine already supports declarative patterns and custom Wasm validators -- the task is to ensure ALL checks use this system.

**specforge-emitter**: Replace hardcoded entity kind metadata (descriptions, DOT properties, schema defaults) with registry lookups. The schema generator and DOT renderer are the main targets.

**specforge-graph**: Ensure graph node types use `WasmEntityKind::Custom(String)` as the default, not as a fallback.

**specforge-lsp**: Replace hardcoded semantic token and icon mappings with `KindRegistryEntry` lookups.

**specforge-mcp**: Replace hardcoded entity kind references in tool implementations with registry lookups.

## Testing Decisions

### What Makes a Good Test

Tests verify that the core compiler handles arbitrary entity kinds correctly by using test-only extension manifests with custom entity kinds ("test_kind_a", "test_kind_b"). Tests should never reference real entity kinds like "behavior" or "feature" in core crate tests -- only in extension-specific test files.

### Modules to Test

**Grammar** -- Verify that removing `verify_statement` doesn't break parsing of any other construct. Verify that `keyword name { ... }` blocks parse for any keyword. Prior art: `tree-sitter-specforge/tests/queries.rs`.

**Parser** -- Verify that custom entity kinds produce the same AST structure as built-in kinds. Verify that `VerifyStatement` removal doesn't affect error recovery. Prior art: `specforge-parser/tests/parse_test.rs`.

**Resolver** -- Verify that reference resolution uses `FieldRegistry` for target kind lookup. Verify that unknown kinds produce correct diagnostics. Prior art: `specforge-resolver/tests/`.

**Validator** -- Verify that all validation checks are extension-declared (no hardcoded checks remain). Verify that a project with only custom extensions compiles correctly. Prior art: `specforge-registry/tests/zero_entity_validation.rs` (this file exists and tests the zero-entity scenario).

**Emitter** -- Verify that schema output for custom entity kinds includes correct metadata from registries. Prior art: `specforge-emitter/tests/schema.rs`.

**CI check** -- Verify the grep-based compliance check catches a deliberate violation and passes on a clean codebase.

**Integration** -- Create a test project with a single custom extension that declares two entity kinds with fields and edges. Compile it end-to-end. Verify schema output, DOT output, JSON export, and validation all work correctly without any built-in extension installed.

## Out of Scope

- **Removing the KindRegistry/FieldRegistry/EdgeRegistry types from the core.** These are core abstractions that must stay. The goal is to make the core depend on these interfaces, not on specific values within them.

- **Removing ManifestV2 from specforge-registry.** Manifest parsing stays as a legacy path (PRD-001 dual-mode). The zero-entity core effort targets runtime behavior, not manifest loading.

- **Changing the extension API.** The extension contribution model (entity kinds, fields, edges, validation rules, etc.) stays the same. The zero-entity core effort is internal refactoring only.

- **Performance optimization.** Registry lookups may be slightly slower than hardcoded checks. This is acceptable -- correctness over performance at this stage.

## Further Notes

### Readiness Assessment

RES-26 assessed the codebase at ~60% ready for zero-entity core:

- **Already registry-driven**: KindRegistry, FieldRegistry, EdgeRegistry exist and are populated from manifests. The validation engine supports declarative and custom rules. The schema generator reads entity metadata from registries.

- **Needs migration**: ~10 files in core crates with hardcoded entity kind references. Parser grammar with `verify_statement`. Some emitter code with hardcoded DOT properties. LSP code with hardcoded semantic tokens.

### Verification Strategy

After the zero-entity core migration, create a "bare metal" test: a project with a single extension declaring two entity kinds (`alpha` and `beta`) with fields and edges. No built-in extensions. If the full pipeline (parse, resolve, validate, emit) works correctly for this project, the zero-entity core is complete.

### Relationship to Other PRDs

- **PRD-001 (Extension Protocol)**: The protocol assumes a zero-entity core. The host discovers all vocabulary through `__handshake` and `__describe`. If the core has hardcoded knowledge, the protocol's promise of full extension-driven behavior is hollow.

- **PRD-002 (Entity Audit)**: The audit removed redundant entities, reducing the surface area of hardcoded references to clean up.

- **PRD-003 (Testability Extraction)**: Extracting testing to an extension is itself a zero-entity core improvement -- it removes `verify` from the grammar and `testable` from the core types.
