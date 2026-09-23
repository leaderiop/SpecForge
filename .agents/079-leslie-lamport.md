# 079 — Leslie Lamport

**Cluster:** C10 — Formal methods (@specforge/formal + analyze passes)
**Roster role:** TLA+; safety/liveness/fairness property taxonomy
**SpecForge anchors:** property entity + kind (safety/liveness/fairness, W063), extensions/formal property specs, future `analyze` temporal passes

## Why this engineer
Lamport gave SpecForge its property taxonomy for free: the `kind` field on property entities (safety/liveness/fairness, enforced by W063) is exactly TLA+'s classification, and SpecForge already documents it that way. TLA+ shows how to write state-machine specs with invariants first and temporal properties second — the same progressive formality RES-25 encodes (prose contracts → structured conditions → formal properties). His TLC model checker is the template for a future analyze pass that takes event graphs and checks properties beyond structural reach.

## References for SpecForge
**Key works**
- [Specifying Systems](https://lamport.azurewebsites.net/tla/book-21-07-04.pdf) — Addison-Wesley, 2002 (canonical free PDF). The full TLA+ language and TLC workflow; safety/liveness chapters map 1:1 onto property kinds.
- **Proving the Correctness of Multiprocess Programs** — IEEE Transactions on Software Engineering SE-3(2), 1977. The safety/liveness decomposition every `kind: safety|liveness|fairness` declaration inherits.
- [TLA⁺ tools (TLC model checker)](https://github.com/tlaplus/tlaplus) — GitHub, tlaplus/tlaplus. Reference implementation of trace generation and counterexample reporting — UX benchmark for analyze diagnostics.
- [Leslie Lamport's publications](https://lamport.azurewebsites.net/pubs/pubs.html) — canonical index, incl. "Computation and State Machines". His state-machine framing suits petgraph-based specforge-graph.

## Study first
1. Specifying Systems ch. 8 (liveness and fairness) — justify W063's kind requirement
2. TLC's error traces — how analyze passes should report violated properties
3. "Who Builds a House without Drawings?" (CACM 2015) — specs-before-code as SpecForge's pitch
