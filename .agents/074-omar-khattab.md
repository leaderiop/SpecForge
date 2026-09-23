# 074 — Omar Khattab

**Cluster:** C9 — MCP & AI-agent context engineering
**Roster role:** DSPy creator; declarative optimization of LM pipelines
**SpecForge anchors:** declarative optimization over LM pipelines (crates/specforge-registry validation_engine, declarative Kind/Field/EdgeRegistry)

## Why this engineer
Khattab — ColBERT author, DSPy creator, now MIT faculty — replaced hand-tuned prompts with declarative modules plus a compiler that optimizes them against metrics. That is the same "declare intent, let the system optimize" move SpecForge makes when .spec sources compile into a validated graph whose vocabulary is declared, not hardcoded. DSPy is the academic backbone for treating agent-facing artifacts (prompts, context) as compiled outputs of a program — the trajectory SpecForge's extension-declared surfaces and PRD-007 guidance prompts sit on.

## References for SpecForge
**Key works**
- [stanfordnlp/dspy](https://github.com/stanfordnlp/dspy) — GitHub, 2022. The declarative-pipeline compiler; architectural sibling of spec-to-graph compilation.
- DSPy: Compiling Declarative Language Model Calls into State-of-the-Art Pipelines — ICLR 2024. The paper: signatures/modules → optimized programs via metric-driven compilation; template for "compile, don't prompt".
- [ColBERT: Efficient and Effective Passage Search via Contextualized Late Interaction over BERT](https://arxiv.org/abs/2004.12832) — SIGIR 2020. Token-aware late interaction — mechanics for ranking what enters a fixed context budget.
- Demonstrate–Search–Predict: Composing retrieval and language models into pipelines — arXiv:2212.14024, 2022. DSPy's predecessor; LM steps composed as explicit programs.

## Study first
1. DSPy signatures/modules vs SpecForge entity/field declarations
2. Metric-driven optimization (teleprompters) vs validation_engine as the objective
3. ColBERT budget mechanics applied to ranking graph slices for agents
