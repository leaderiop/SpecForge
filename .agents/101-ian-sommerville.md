# 101 — Ian Sommerville

**Cluster:** C12 — Requirements engineering, ubiquitous language & docs-as-code
**Roster role:** Requirements engineering canon
**SpecForge anchors:** behavior/invariant/feature chain — `spec/features/*.spec` → `spec/behaviors/*.spec` → `spec/invariants/*.spec`; `docs/spec-writing-flow.md`

## Why this engineer
Sommerville wrote the reference discipline for turning fuzzy stakeholder intent into structured, validated, traceable requirements: elicitation, specification, validation, management as a managed process with viewpoints and checklists. SpecForge's entity chain is that canon compiled into a typed graph — features state the capability, behaviors state observable contracts, invariants state the always-true properties, and validation replaces manual RE checklists. His good-practice guides are the benchmark for judging whether SpecForge's authoring flow (`spec-writing-flow.md`) omits a needed RE step.

## References for SpecForge
**Key works**
- **Requirements Engineering: A Good Practice Guide** (with Pete Sawyer) — Wiley, 1997. Viewpoints, scenarios, and process improvement checklists — the source of "validation ≠ verification" discipline that SpecForge's validator stages implement.
- **Requirements Engineering: Processes and Techniques** (with G. Kotonya) — Wiley, 1998. The systematic treatment of elicitation through traceability management; maps to SpecForge's reference links and file-ref checks.
- **Software Engineering**, 10th edition — Pearson, 2015. The canonical textbook; chapters on RE and dependability define the shared vocabulary every team member arrives knowing.

## Study first
1. Good Practice Guide ch. on validation techniques — mirror them as declarative validation rules
2. Viewpoints/scenarios as the ancestor of personas → journeys → features in `spec/product/`
3. Requirements change management — how `specforge-watch` incremental invalidation fits RE canon
