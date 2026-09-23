# 084 — Gerard J. Holzmann

**Cluster:** C10 — Formal methods (@specforge/formal + analyze passes)
**Roster role:** Spin/Promela; event-graph model checking practice
**SpecForge anchors:** event_graph_analyze pass, `sync` blocks on events (sync.timeout), protocol entities (ordering conflicts, W068)

## Why this engineer
Holzmann built Spin/Promela and spent decades checking distributed designs at Bell Labs and JPL — the practice behind SpecForge's event_graph_analyze pass, where events, producers/consumers, and `sync` blocks (with `sync.timeout` unblocking E034 cycles) form exactly a Promela-style communication model. His protocol entities with ordering constraints mirror Promela channel ordering, and W066–W068 validation is a lint-tier version of Spin's feasibility checks. His "design, then verify the design, cheaply, before code" discipline is RES-25's argument in engineering clothes.

## References for SpecForge
**Key works**
- **The Model Checker SPIN** — IEEE Transactions on Software Engineering 23(5), 1997. The canonical paper: automata-theoretic verification of communication protocols — the theory event graphs could grow into.
- **The Spin Model Checker: Primer and Reference Manual** — Addison-Wesley, 2003. Promela constructs (channels, rendezvous, timeouts) — direct analogues of SpecForge's sync vocabulary.
- [Spin root](https://spinroot.com/spin/whatispin.html) — canonical Spin site with tutorials and papers.
- [nimble-code/Spin](https://github.com/nimble-code/Spin) — GitHub, source repository.
- **Design and Validation of Computer Protocols** — Prentice Hall, 1991. Pre-Spin textbook on protocol failure modes; a catalog of what event-graph linting should catch.

## Study first
1. 1997 TSE paper — what structural checks miss until you explore state spaces
2. Promela `timeout` semantics — grounding for sync.timeout and E034 mitigation rules
3. Never-claim/LTL workflow — roadmap beyond structural linting
