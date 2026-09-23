# 005 — Niklaus Wirth

**Cluster:** C2 — DSL & language design
**Roster role:** Pascal/Oberon designer; lean-software advocate
**SpecForge anchors:** 5-minutes-to-learn DSL goal (principle "Learn in 5 minutes" — minimal syntax, block-based structure); specforge-cli command surface (`crates/specforge-cli/src/main.rs`, 34 clap subcommands); lean zero-domain-knowledge compiler core

## Why this engineer
Wirth designed Pascal and Oberon as languages a student masters in hours, and his stepwise-refinement method is how SpecForge's tiny block grammar should keep growing. "A Plea for Lean Software" is the standing argument for the zero-domain-knowledge core: keep the compiler a pure typed-graph engine and push all vocabulary outward — exactly SpecForge's extension model. His discipline also names the project's sharpest known risk: the 103k-LOC workspace and 34-command CLI must not accrete the accidental complexity he spent his career cutting.

## References for SpecForge
**Key works**
- **A Plea for Lean Software** — IEEE Computer 28(2), 1995. Software should run in a fraction of the resources; lean design is discipline, not poverty — the governing constraint on CLI surface and core size.
- [Program Development by Stepwise Refinement](https://dl.acm.org/doi/10.1145/362575.362577) — CACM 14(4), 1971. Grow a system in small verifiable steps — the model for evolving the .spec grammar via the 14-step compile pipeline.
- [Project Oberon: The Design of an Operating System and Compiler](https://www.projectoberon.net) — with J. Gutknecht, Addison-Wesley, 1992. A complete minimal language+compiler kept small by design; the archetype of lean core + external everything else.
- **Algorithms + Data Structures = Programs** — Prentice Hall, 1976. Program shape follows data shape; SpecForge's petgraph node/edge model is the data that dictates the code.
- **Good Ideas, Through the Looking Glass** — IEEE Computer, 2006. A designer's retrospective on which language ideas actually paid off — a filter for proposed .spec additions.

## Study first
1. A Plea for Lean Software (1995)
2. Project Oberon compiler chapters — lean core discipline
3. Program Development by Stepwise Refinement (1971)
