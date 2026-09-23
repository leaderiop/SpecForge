# 103 — Michael A. Jackson

**Cluster:** C12 — Requirements engineering, ubiquitous language & docs-as-code
**Roster role:** Problem Frames; JSP — describe the problem before the solution
**SpecForge anchors:** `problem`/`solution` fields on every feature in `spec/features/*.spec`; `docs/entity-model.md` entity semantics; `spec/product/personas.spec` as problem-context actors

## Why this engineer
Jackson's central demand is separation of problem description from machine specification: you must frame the problem (its domain types, shared phenomena, requirement constraints) before designing any solution, and say which domain facts are in scope. SpecForge bakes this in as syntax — every feature entity carries paired `problem` and `solution` prose fields, and behaviors reference observable phenomena on graph nodes rather than implementation internals. His Problem Frames taxonomy (required behavior, workpieces, information display…) is a ready-made classifier for future feature patterns, and his 1995 lexicon is the discipline behind the glossary.

## References for SpecForge
**Key works**
- **Problem Frames: Analyzing and Structuring Software Development Problems** — Addison-Wesley, 2001. Frame diagrams and domain/phenomenon separation — the theory behind `problem`/`solution` pairing and observable-phenomena edges.
- **Software Requirements & Specifications: A Lexicon of Practice, Principles and Prejudices** — Addison-Wesley, 1995. Short essays demanding precise shared terms — the paper ancestor of `glossary.spec` term discipline.
- **Principles of Program Design** (JSP) — Academic Press, 1975. Structure the program to mirror the structure of its problem data — echoed in SpecForge's grammar-shaped AST and graph-first design.
- **Problem Frames and Software Engineering** — Information and Software Technology 47(14), 2005. Jackson's own retrospective tying frames to SE practice.
- [Jackson workbench (community archive)](http://www.jacksonworkbench.co.uk/stevefergspages/pfa/index.html) — maintained collection of JSP/JSD/Problem Frames materials and papers.

## Study first
1. Problem Frames ch. 1-3: machine vs problem domains — audit SpecForge features for domain leakage
2. The 1995 lexicon — rules for what belongs in a glossary term vs a doc
3. Frame variants as candidate feature archetypes for extensions
