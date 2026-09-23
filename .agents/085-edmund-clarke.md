# 085 — Edmund M. Clarke

**Cluster:** C10 — Formal methods (@specforge/formal + analyze passes)
**Roster role:** Model checking (Turing Award 2007); symbolic & bounded techniques
**SpecForge anchors:** model-checking roadmap for analyze passes, specforge-graph (petgraph reachability/cycles), property kinds beyond static checks

## Why this engineer
Clarke co-invented model checking — the automatic, counterexample-producing analysis of state machines against temporal properties — and made it scale via symbolic methods and counterexample-guided abstraction. SpecForge's analyze passes are the prehistory of that story: today petgraph does reachability and cycle detection (E041/E042); Clarke's program is the roadmap to checking safety/liveness/fairness property entities (W063 kinds) over event-graph state machines automatically, with counterexamples wired back into the 219-file spec corpus. Deceased 2020; his papers are the canon.

## References for SpecForge
**Key works**
- **Model Checking** — MIT Press, 1999; 2nd ed. 2018 (with Grumberg, Kroening, Peled, Veith). The standard text: fixpoint characterizations, symbolic (BDD) methods, abstraction — the syllabus for future analyze passes.
- **Design and Synthesis of Synchronization Skeletons Using Branching-Time Temporal Logic** — Logic of Programs Workshop (LNCS 131), 1981. The founding CTL paper — properties as temporal logic over transition systems, i.e., property entities over event graphs.
- **Counterexample-Guided Abstraction Refinement** — CAV 2000 (LNCS 1855). CEGAR: iterate abstraction until spurious counterexamples vanish — the technique for keeping analyze tractable on large graphs.
- **Model Checking** (Turing lecture) — Communications of the ACM 52(11), 2009. His own retrospective on making verification an industry practice.

## Study first
1. 1981 skeletons paper — temporal properties over state machines
2. CEGAR — abstraction strategy for analyzing whole extension graphs
3. 2nd-ed. Model Checking ch. on SAT-based bounded checking
