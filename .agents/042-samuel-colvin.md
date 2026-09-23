# 042 — Samuel Colvin

**Cluster:** C6 — Schemas, validation & serialization (Graph Protocol)
**Roster role:** Pydantic creator; Rust-core validation architecture
**SpecForge anchors:** `crates/specforge-registry/src/compilation/validation_engine.rs` (declarative `ValidationRulePattern` kinds: missing-field, field-constraint, cycle, file-exists, conditional-required, custom wasm); manifest validation rules; `WasmValidationRuntime` dispatch

## Why this engineer
Colvin rewrote Pydantic's hot path in Rust (pydantic-core), proving at ecosystem scale the architecture SpecForge's validator already echoes: users declare rules in a friendly DSL, a compiled core executes them fast, and the core reports precisely localized errors. SpecForge's `validation_engine.rs` is the same shape — declarative rule patterns (`required: true`, conditional-field requirements, constraint matching) interpreted over the compiled graph, with unknown checks delegated to Wasm custom validators. Pydantic v2's remaining lesson: keep the rule vocabulary small and orthogonal, and spend engineering on error quality, not keyword sprawl.

## References for SpecForge
**Key works**
- [pydantic/pydantic-core](https://github.com/pydantic/pydantic-core) — GitHub, 2021–present. Rust validation core behind Pydantic v2 (released June 2023): the declare-DSL/compile-execute split to emulate.
- [Pydantic documentation — validation concepts](https://docs.pydantic.dev) — docs.pydantic.dev. Strict vs lax coercion and field-constraint design, comparable to `FieldConstraintPattern`.
- "SE Radio 676: Samuel Colvin on the Pydantic Ecosystem" — IEEE Software Engineering Radio, July 2025. First-person account of the v1→v2 rewrite trade-offs and the Wasm-adjacent "core in Rust, API elsewhere" strategy.
- [pydantic.dev](https://pydantic.dev) — Company site (Founder & CEO). How validation tooling sustains itself as a product.

## Study first
1. pydantic-core's SchemaValidator compilation: rules → executable plan, mirroring `ValidationRulePattern`
2. Error localization: path-annotated, severity-graded failures → SpecForge `Diagnostic`/`Severity`
3. Which checks stay declarative vs. escape-hatch to code (`Custom` / wasm_function)
