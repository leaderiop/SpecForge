# 102 — Axel van Lamsweerde

**Cluster:** C12 — Requirements engineering, ubiquitous language & docs-as-code
**Roster role:** KAOS goal-oriented requirements engineering
**SpecForge anchors:** feature→behavior→invariant linkage — `spec/features/*.spec` behaviors lists, `maintains` blocks linking behaviors to `spec/invariants/*.spec`, `see_also` chains in `spec/glossary.spec`

## Why this engineer
KAOS made the requirements graph itself the engineering artifact: goals are refined top-down into requirements and expectations assigned to agents, with obstacles and conflicts derived systematically. SpecForge's schema does the same mechanically — features compose behaviors, behaviors maintain invariants, edges carry the refinement — so a missing link (an orphan behavior, an unmaintained invariant) is a compiler error rather than a review note. Van Lamsweerde's operationalization step (goal → agent requirement) is exactly the feature → behavior → verify descent, and his goal-obstacle patterns are a checklist for `failure-modes.spec`.

## References for SpecForge
**Key works**
- **Goal-Directed Requirements Acquisition** (A. Dardenne, A. van Lamsweerde, S. Fickas) — Science of Computer Programming 20(1-2), 1993. [DOI: 10.1016/0167-6423(93)90021-G] The founding KAOS paper: goal refinement trees, agent assignment — the abstract shape of SpecForge's graph.
- **Goal-Oriented Requirements Engineering: A Guided Tour** — Proc. IEEE International Requirements Engineering Conference (RE'01), 2001. The condensed method: obtain, refine, operationalize, resolve obstacles — stage-by-stage guidance for authors.
- **Requirements Engineering: From System Goals to UML Models to Software Specifications** — Wiley, 2009. The full treatise, including goal patterns and obstacle catalogs worth mining for the governance extension.

## Study first
1. The 1993 paper's refinement/assignment semantics vs SpecForge's feature/behavior/invariant edges
2. Operationalization: how a goal becomes agent-observable requirements = behaviors with `verify`
3. Obstacle catalogs (book ch. 16) as enrichment candidates for failure_modes entities
