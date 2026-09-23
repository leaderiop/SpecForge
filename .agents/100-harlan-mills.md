# 100 — Harlan Mills

**Cluster:** C12 — Requirements engineering, ubiquitous language & docs-as-code
**Roster role:** Cleanroom software engineering; correctness by specification
**SpecForge anchors:** correctness-by-specification philosophy — `specforge-validator` declarative `validation_engine`, behavior contract statements + `verify` blocks, ~2,900-test suite as usage-model analogue

## Why this engineer
Mills inverted debugging: programs are mathematical functions whose intended function is specified first and verified incrementally, with execution reserved for statistical usage testing, not defect search. SpecForge encodes the same inversion for specifications: a behavior's contract statement and verify blocks define intended function before any consumer exists, and the validator (not a runtime) carries the correctness argument. Cleanroom's separation of specification, development and certification maps onto SpecForge's pipeline split — author, compiler, validator.

## References for SpecForge
**Key works**
- **Cleanroom Software Engineering** (H. D. Mills, M. Dyer, R. C. Linger) — IEEE Software 4(5), 1987. [DOI: 10.1109/MS.1987.231413] The founding statement: formal specification, function-theoretic correctness, statistical certification — no unit debugging.
- **The New Math of Computer Programming** — CACM 18(1), 1975. Every program = intended function + correctness theorem; the mindset behind SpecForge's "every behavior carries its own verifiable contract."
- **Zero Defect Software: Cleanroom Engineering** — Advances in Computers 36, 1993. Mills' mature synthesis, including box-structure specification (black/white/state boxes) — a template for tiered behavior detail.
- **Cleanroom Software Engineering: Technology and Process** (Prowell, Trammell, Linger & Whittaker) — Addison-Wesley, 1999. The codified reference process; useful for organizing SpecForge's validate-then-emit gates.

## Study first
1. The 1987 IEEE Software paper — the shortest route to the philosophy
2. Box structures (black/state/white boxes) vs SpecForge's feature → behavior → verify layering
3. Usage models (Markov chains) as the analogue of SpecForge's declarative validation rules
