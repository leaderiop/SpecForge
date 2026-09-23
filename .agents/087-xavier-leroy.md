# 087 — Xavier Leroy

**Cluster:** C10 — Formal methods (@specforge/formal + analyze passes)
**Roster role:** CompCert; verified-compiler ceiling and TCB discipline
**SpecForge anchors:** verified-compiler ceiling for the pipeline, TCB discipline (check-zero-entity-core.sh spirit), crates/tree-sitter-specforge → specforge-graph → specforge-emitter chain

## Why this engineer
Leroy's CompCert is the proof that a full compiler can be verified: every transformation step carries a machine-checked correctness theorem, and the trusted computing base is kept deliberately tiny. SpecForge's pipeline (grammar → AST → resolver → graph → emitters) is a compiler with the same shape and, for now, the same unverified steps — his work defines both the ceiling (what a fully verified compile could mean: compiled output preserves validated-graph meaning) and the discipline (shrink and audit the TCB), which scripts/check-zero-entity-core.sh enforces in miniature by proving the core stays free of domain vocabulary.

## References for SpecForge
**Key works**
- **Formal verification of a realistic compiler** — Communications of the ACM 52(7), 2009. The CompCert summary paper: semantics preservation, pass-by-pass proofs, measured TCB — the target shape for any SpecForge correctness claim.
- [AbsInt/CompCert](https://github.com/AbsInt/CompCert) — GitHub. The verified C compiler source; note the proof architecture around the extraction to OCaml.
- [compcert.org](https://compcert.org) — canonical site; current releases (v3.x) show a verified toolchain maintained in production.
- **Formal Certification of a Compiler Back-end** — POPL 2006. The first CompCert paper; per-pass refinement statements as a template for graph→emitter claims.
- [CompCert at Inria/Collège de France](https://xavierleroy.org) — his publication page; see also the Collège de France "Mechanized semantics" lectures.

## Study first
1. CACM 2009 paper — decomposition of correctness into per-pass theorems
2. CompCert's TCB accounting — analogue of the zero-entity-core guarantee
3. Semantics-preservation statements — what "compile .spec → emitted artifacts faithfully" would formalize
