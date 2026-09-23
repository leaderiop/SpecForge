# 082 — K. Rustan M. Leino

**Cluster:** C10 — Formal methods (@specforge/formal + analyze passes)
**Roster role:** Dafny/Spec#/Boogie; automatic verification-condition generation
**SpecForge anchors:** future `specforge analyze --prove` path, condition_check pass, W035 undischarged obligations, dafny-lang benchmark

## Why this engineer
Leino's Dafny is the existence proof that requires/ensures ergonomics plus automatic VC discharge can reach ordinary engineers — the exact destination of SpecForge's progressive formality ladder (prose → conditions → properties → proof). When SpecForge's `analyze --prove` path materializes, it will face Dafny's core problem set: translating contracts to verification conditions (Boogie), feeding them to an SMT backend (Z3), and turning opaque failures into actionable diagnostics. His verifier-feedback UX is the bar for W035-style obligation reporting.

## References for SpecForge
**Key works**
- **Dafny: An Automatic Program Verifier for Functional Correctness** — LPAR-17 (LNCS 6397), 2010. How requires/ensures become Boogie VCs — the encoding SpecForge's contract layer should mirror before any prove pass.
- **Developing Verified Programs with Dafny** — ICSE 2013. The iteration loop between annotations and counterexamples — UX model for analyze diagnostics.
- **Program Proofs** — MIT Press, 2023. Modern Hoare-logic/wp textbook aimed at working engineers; the on-ramp for formal contributors to @specforge/formal.
- [dafny-lang/dafny](https://github.com/dafny-lang/dafny) — GitHub. Reference implementation: annotation syntax, VC pipeline, LSP integration.
- [leino.science](https://leino.science) — papers and lectures; see "This is Boogie 2" (manuscript, 2008) for the contract-IR layer.

## Study first
1. Dafny LPAR-17 — contract→VC encoding
2. Boogie 2 manuscript — the intermediate verification language analyze --prove should emit
3. Dafny LSP counterexample surfacing — diagnostics for undischarged obligations
