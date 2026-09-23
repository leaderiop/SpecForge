# 002 — John Backus

**Cluster:** C1 — Foundations of intent capture & specification
**Roster role:** FORTRAN team lead; BNF; 1977 Turing lecture on liberation from von Neumann style
**SpecForge anchors:** crates/tree-sitter-specforge/grammar.js, docs/model/formats.md

## Why this engineer
Backus led FORTRAN — the first compiler whose output was efficient enough to prove the whole idea — then gave his notation to the ALGOL reports, creating BNF, the meta-language every small grammar (including `grammar.js`) descends from. His 1977 Turing lecture argues programs should be stated as declarative intent over ideas, not as machine-shaped step sequences: SpecForge's move from prose-plus-code to a typed graph is that argument applied to what agents read. His "model" rendering lineage (docs/model/formats.md) is BNF's descendants describing structure instead of computation.

## References for SpecForge
**Key works**
- [Can Programming Be Liberated from the von Neumann Style? A Functional Style and Its Algebra of Programs](https://dl.acm.org/doi/10.1145/359576.359579) — Turing Award Lecture, *Communications of the ACM* 21(8), 1978. The manifesto for stating intent declaratively — the intellectual basis for compiling specs rather than scripting agents.
- **The FORTRAN Automatic Coding System** (with R. Beeber et al.) — Proceedings of the Western Joint Computer Conference, 1957. The feasibility proof a spec compiler must repeat: human-legible source, machine-checked output, worth the translation cost.
- **Preliminary Report: Specifications for the International Algebraic Language** (ALGOL 58) — ACM committee (Backus et al.), presented at ICIP Paris, 1959. First publication of the notation later called BNF.
- **Report on the Algorithmic Language ALGOL 60** (ed. P. Naur, notation by Backus) — *Communications of the ACM* 3(5), 1960. The discipline of defining a tiny grammar exactly so independent tools agree — SpecForge's zero-domain-knowledge core parsed by many consumers.

## Study first
1. The 1978 Turing lecture — declarative intent over procedural noise
2. FORTRAN 1957 report — the economics that justify any compiler
3. ALGOL 60 report — BNF discipline for a small, stable grammar
