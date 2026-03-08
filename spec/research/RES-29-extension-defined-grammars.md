# RES-29: Extension-Defined Grammars — Body Parser Architecture

**Status:** Active
**Priority:** Critical
**Date:** 2026-03-07
**Depends on:** RES-11a (Compiler Architecture), RES-23 (Contribution-Based Extension Model), RES-26 (Zero-Entity Core)

## Problem Statement

The current tree-sitter grammar is fixed to `keyword name [title] { fields }`. Extensions needing structured syntax beyond key-value pairs must push data into opaque strings the compiler cannot validate. This contradicts:

- **Principle 2** (Zero domain knowledge in core): Core grammar embeds assumptions about field structure
- **Principle 4** (Validation is the value): Opaque strings bypass compile-time validation

## Expert Panel Analysis

### Path 1: Monolithic Grammar Extension
Extend the core tree-sitter grammar with new syntax rules for each extension need.

**Pros:** Single grammar, simpler tooling
**Cons:** Core grows with each extension (violates zero-domain-knowledge), grammar conflicts between extensions, core team becomes bottleneck

**Panel verdict:** 2/10 — fundamentally incompatible with zero-entity-core architecture.

### Path 2: Per-Extension Body Parsers (Selected)
Extensions provide Wasm exports that parse raw body text into structured JSON fields. Core captures `keyword name { <raw_body> }` and delegates parsing to extensions.

**Pros:** Core stays minimal, extensions own their syntax, composable, backward compatible
**Cons:** Two-phase parsing complexity, grammar caching needed, ABI version management

**Panel verdict:** 9/10 — natural extension of contribution-based model (RES-23), aligns with Wasm runtime (RES-26).

## Architecture Design

### Compilation Flow

```
Phase 1   — Core Parse:       keyword name { <raw body text> }
Phase 1.5 — Extension Parse:  body_parser.parse(raw_text) -> structured JSON fields
Phase 2   — Semantic:         validate structured fields against FieldRegistry
```

### New Contribution Types

1. **`grammars`**: Tree-sitter `.wasm` binaries for editor syntax highlighting
   - Declared per entity kind in extension manifest
   - Loaded by LSP for syntax highlighting
   - ABI version validated against host runtime
   - Content-hash cached for performance

2. **`body_parsers`**: Wasm exports for compile-time body parsing
   - Export signature: `body_parse(raw_text: string) -> JSON`
   - Output validated against declared field schema
   - Timeout-enforced (configurable, default 5000ms)
   - Fallback to raw string on parser error

### Conflict Resolution

`GrammarConflictPolicy` (configurable in `specforge.json`):
- **`error`** (default): Fail if two extensions target the same entity kind
- **`priority`**: Higher-priority extension wins
- **`namespace`**: Both grammars load, scoped by extension namespace

## LSP Integration

- Grammar `.wasm` files loaded at LSP startup and on extension change
- Semantic tokens delegated to extension grammars for their entity kinds
- Body parser results cached per-file for incremental updates
- Grammar cache invalidated on extension update or ABI version change

## Backward Compatibility

- Extensions without body parsers use the existing field parser (no change)
- Extensions without grammar contributions get default highlighting (no change)
- `ManifestV2` fields are @optional — existing manifests remain valid
- Phase 1.5 is a no-op when no body parsers are registered

## Performance Budget

- Grammar loading: < 50ms per grammar (AOT cached)
- Body parser dispatch: < 100ms per entity (Wasm warm start)
- Grammar cache hit: < 1ms (content-hash lookup)
- Total Phase 1.5 overhead: < 200ms for typical project

## Security Considerations

- Grammar `.wasm` validated for ABI version and size limits before loading
- Body parser runs in Wasm sandbox (no filesystem, no network)
- Timeout enforcement prevents infinite loops
- Output JSON validated against schema before acceptance into compilation pipeline
- Checksum verification for grammar `.wasm` integrity

## Cross-References

- **RES-11a**: Compiler pipeline — Phase 1.5 inserts between structural parse and semantic validation
- **RES-23**: Contribution model — `grammars` and `body_parsers` are new contribution types alongside entities, validators, renderers, providers
- **RES-26**: Zero-entity core — body parsers ensure core grammar needs zero domain knowledge
