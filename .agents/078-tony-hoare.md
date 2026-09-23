# 078 — C. A. R. Hoare

**Cluster:** C10 — Formal methods (@specforge/formal + analyze passes)
**Roster role:** CSP (process/sync entities); verifying-compiler grand challenge
**SpecForge anchors:** extensions/formal `process` entities (alphabet/states/composition), E034/E042 cycle checks, RES-25 CSP row

## Why this engineer
Hoare's CSP is the theory behind SpecForge's `process` entities — the alphabet/states/initial_state/composition model in `extensions/formal` is CSP's processes-and-events worldview in miniature, and the `sync` vocabulary on events is CSP synchronization. His cycle discipline (deadlock/freedom from circular waits) is what E034 (unmitigated cycle) and E042 (process composition cycle) enforce structurally. His verifying-compiler grand challenge legitimizes SpecForge's trajectory from structural linting toward `specforge analyze --prove`.

## References for SpecForge
**Key works**
- [Communicating Sequential Processes](https://dl.acm.org/doi/10.1145/359576.359585) — CACM 21(8), 1978. Defines processes, alphabets, synchronous communication — the direct basis for `process` entities and `sync` edges.
- **Communicating Sequential Processes (book)** — Prentice Hall, 1985 (free via Oxford's concurrency group). Refinement, failures/divergences semantics; the math a future `analyze` pass over ProcessComposition should implement.
- **The Verifying Compiler: A Grand Challenge for Computing Research** — JACM 50(1), 2003. The charter document for compiler-integrated verification — SpecForge's compiled-and-checked graph is a modest cousin.
- **An Axiomatic Basis for Computer Programming** — CACM 12(10), 1969. Preconditions/postconditions as the semantics of `requires`/`ensures` blocks.
- **The Emperor's Old Clothes** (Turing Award lecture) — CACM 24(2), 1981. The argument for simplicity and proof-friendly design that the zero-entity-core architecture embodies.

## Study first
1. 1978 CSP paper; map event/sync vocabulary onto `process` alphabet fields
2. CSP book ch. on failures/divergences — what E042-style checks cannot yet see
3. Verifying compiler paper — framing for the formal roadmap (RES-25)
