# 099 — David L. Parnas

**Cluster:** C12 — Requirements engineering, ubiquitous language & docs-as-code
**Roster role:** Information hiding; requirements as mathematical tables
**SpecForge anchors:** `crates/` workspace modularity (17 crates, zero-domain core), traceability chain `specforge-emitter/src/trace.rs` + `specforge-report.json`, `specforge-validator` file-ref checks

## Why this engineer
Parnas defined the only criterion that justifies SpecForge's crate layout: modules/crates hide design decisions likely to change (grammar, graph shape, emission formats), while exposing stable interfaces — exactly the zero-domain-knowledge core with vocabulary injected by Wasm extensions. His A-7E work proved requirements can be documented as precise, page-referenced mathematical tables, the ancestor of SpecForge's behavior contracts and machine-checkable invariants, and of the trace.rs chain linking every emitted artifact back to a source span.

## References for SpecForge
**Key works**
- **On the Criteria To Be Used in Decomposing Systems into Modules** — CACM 15(12), 1972. The information-hiding criterion behind separating `specforge-parser`/`-graph`/`-emitter` from extension-supplied vocabulary.
- **The Rational Design Process: How and Why to Fake It** (with P. C. Clements) — IEEE Transactions on Software Engineering SE-12(2), 1986. SpecForge's `.spec` corpus is a maintained "faked" rational process: idealized documentation kept true against a real, evolving codebase.
- **Software Requirements for the A-7E Aircraft** (K. L. Heninger, J. Kallander, D. L. Parnas, J. E. Shore) — NRL Memorandum Report 3876, 1978; second release (Parnas, Asmis & Madey) NRL Report 9314, 1992. The proof that tabular, checkable function documents scale to real avionics — the model for SpecForge's declarative validation tables.
- **Functional Documents for Computer Systems** (with J. Madey) — Science of Computer Programming 25, 1995. The distillation of the A-7E style: relations and functions over observable quantities, directly analogous to SpecForge behaviors over graph state.

## Study first
1. The 1972 decomposition paper — then re-read SpecForge's crate boundaries as decisions to hide
2. "The Rational Design Process" — docs drift (wasi vs unknown-unknown) is the documented-aspiration gap
3. A-7E device-function tables (NRL 9314) as the format ancestor of SpecForge behavior tables
