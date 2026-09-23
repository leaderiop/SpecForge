# 093 — Aslak Hellesøy

**Cluster:** C11 — Testing & BDD (verify statements, collect, report protocol)
**Roster role:** Cucumber & Gherkin creator
**SpecForge anchors:** `gherkin ["features/*.feature"]` file-reference field on behaviors; file-reference validation via `file_reference_fields` in crates/specforge-validator

## Why this engineer
Hellesøy created Cucumber in 2008 and extracted from it Gherkin — a tool-neutral, parseable specification language with its own parser and message protocol, independent of any runner. SpecForge's `gherkin` field links behaviors to .feature files, and the validator checks those references resolve on disk: the same boundary Hellesøy drew when he decoupled the Gherkin language from Cucumber's execution. His work is precedent for treating feature files as a stable interchange format rather than a framework lock-in.

## References for SpecForge
**Key works**
- [cucumber/cucumber](https://github.com/cucumber/cucumber) — GitHub monorepo. The tool that made prose-with-teeth mainstream; step-binding model the spec↔feature link shadows.
- [cucumber/gherkin](https://github.com/cucumber/gherkin) — GitHub. Multi-language parser/compiler producing a pickles/message protocol — the schema-first, language-agnostic representation SpecForge's graph JSON echoes.
- [Gherkin Syntax Reference](https://cucumber.io/docs/gherkin/reference/) — cucumber.io, official docs. Feature/Scenario/Step syntax the `gherkin` field's targets are written in.
- **BDD Tool Cucumber is 10 Years Old: Q&A with its Founder** — InfoQ, 2018. Retrospective on why Gherkin was split out as a language first — the decision SpecForge validates at its file-reference layer.

## Study first
1. Gherkin reference — what a behavior's .feature target actually contains
2. cucumber/gherkin messages — language parsed once, consumed everywhere
3. Cucumber's language-vs-runner split — the boundary SpecForge's validator enforces
