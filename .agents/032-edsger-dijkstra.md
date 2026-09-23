# 032 — Edsger W. Dijkstra

**Cluster:** C5 — Graph engine & algorithms
**Roster role:** shortest paths, weakest preconditions, 'On the role of scientific thought'
**SpecForge anchors:** graph traversal & ordering (crates/specforge-graph/src/graph.rs DFS, crates/specforge-resolver/src/resolve.rs `topological_sort`); formal-extension reasoning style (crates/specforge-emitter/src/builtins/formal.rs, extensions/formal); core-vs-extension separation of concerns (zero-domain-knowledge core, all vocabulary from Wasm extensions)

## Why this engineer
Dijkstra supplies both halves of SpecForge's intellectual spine. His 1959 paper is the archetype of the economical graph algorithms the resolver/graph crates implement; his guarded-command/weak-precondition work is the reasoning style the formal extension's requires/ensures contracts imitate. EWD447's "separation of concerns" is precisely how SpecForge stays sane: a core that knows no domain vocabulary, studied in isolation, with all vocabulary injected by extensions — correctness of each layer reasoned about separately before composition.

## References for SpecForge
**Key works**
- **A Note on Two Problems in Connexion with Graphs** — Numerische Mathematik 1:269–271, 1959. Shortest path + minimum spanning tree in three pages; the model of algorithm exposition the graph crate should live up to.
- [On the role of scientific thought (EWD447)](https://www.cs.utexas.edu/~EWD/transcriptions/EWD04xx/EWD447.html) — EWD archive, University of Texas at Austin, 1974. The separation-of-concerns memo; the design charter for SpecForge's zero-domain-knowledge core.
- **Guarded Commands, Nondeterminacy and Formal Derivation of Programs** — Communications of the ACM 18(8):453–457, 1975. Weakest preconditions: the semantics under the declarative validation_engine's requires/ensures rules.
- **A Discipline of Programming** — Prentice-Hall, 1976. The wp-calculus worked end to end; reading for formalizing extension validation rules as predicates.

**Archive**
- [E.W. Dijkstra Archive](https://www.cs.utexas.edu/~EWD/) — University of Texas at Austin. The canonical collection of all EWD notes.

## Study first
1. EWD447 — separation of concerns as the core/extensions design discipline
2. A Discipline of Programming: weakest precondition as the meaning of a contract
3. The 1959 paper — rigor by economy of proof
