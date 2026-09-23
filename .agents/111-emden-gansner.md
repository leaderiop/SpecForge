# 111 — Emden Gansner

**Cluster:** C13 — Diagrams & rendering
**Roster role:** Graphviz layout algorithms (dot ranking, neato/sfdp stress models)
**SpecForge anchors:** `emit --format=dot` (crates/specforge-emitter/src/dot.rs), specforge-graph → dot rendering (crates/specforge-emitter/src/outline/dot.rs), model/outline `rankdir` hierarchies

## Why this engineer
Gansner designed the algorithms that turn SpecForge's DOT strings into readable diagrams: dot's layered ranking for the `rankdir=LR` digraphs the emitters produce, and the stress-majorization model behind neato for the dense cross-kind graphs that ranking alone cannot untangle. Reading his papers tells the team *why* `rankdir`, ranking, and spline routing work — and what output changes are algorithm facts (never to be "fixed" in the emitter) versus formatting facts (emitter-owned).

## References for SpecForge
**Key works**
- **A technique for drawing directed graphs** (with Koutsofios, North, Vo) — IEEE Transactions on Software Engineering 19(3), 1993. The dot ranking/orientation algorithm; grounds every layered assumption in SpecForge's DOT output.
- [Graph Drawing by Stress Majorization](https://link.springer.com/chapter/10.1007/978-3-540-31843-9_25) (with Koren, North) — Graph Drawing 2004 (LNCS 3383). neato's model; the path to scaling layout if spec corpora outgrow layered drawing.
- [An open graph visualization system and its applications to software engineering](https://graphviz.org/documentation/GN99.pdf) (with North) — Software: Practice and Experience, 1999. Whole-system view including the attribute language (`shape`, `style`, `color`) SpecForge's dot_shape/dot_color vocabulary mirrors.
- [Drawing graphs with dot](https://graphviz.org/pdf/dotguide.pdf) — AT&T user guide, 1988–2015. Canonical semantics of the attributes `emit_dot` writes.
- [Graphviz](https://graphviz.org) — project site and [source](https://gitlab.com/graphviz/graphviz).

## Study first
1. TSE 1993 ranking algorithm (rank assignment → ordering → coordinates)
2. Stress majorization for large graphs
3. dotguide attribute semantics vs what the emitters emit
