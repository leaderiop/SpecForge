# An edge from an entity to itself counts in no edge count

**Status:** accepted (2026-10-26, architecture round 28, plan 07). Extends [ADR 0174](0174-the-graph-owns-directed-reach-and-degree.md) D2 (the degree's rule reaches the snapshot). Amends [ADR 0019](0019-one-entity-snapshot-per-compile.md) (an entity's edge counts). Confirms [ADR 0232](0232-a-missing-link-is-a-field-written-empty.md) D1 (a written reference to itself has its link).

## Context

- `Graph::degree` leaves out an edge from an entity to itself (ADR 0174 D2), so `stats`, `explore`, `review` and the export budget call an entity whose only reference names itself unconnected.
- The entity snapshot counted the same edge (raw `edges_to` / `edges_from`), so every `no_incoming_edges` rule, W012 and the pass input's `incoming_edge_count` / `outgoing_edge_count` called it referenced. W002, W041, W044, W131 and W134 stayed silent on an entity only it references, against their own spec text ("a type naming itself does not count", "or other feature", "from another refinement").
- The product extension's `health` (a guest, over the command graph) counted it too, while `term-density` in the same guest did not.
- Reproduced: `term loner { see_also [loner] }` is unconnected to `stats`, isolated to `term-density`, referenced to `health`. `no_edges` (I010) stays quiet on it by design: the field it writes has its link (ADR 0232 D1).
- Five commits across five modules wrote the rule; the snapshot, which every rule reads, was never one of them.

```text
BEFORE  degree: x -> x = 0     snapshot: x -> x = 1     health: x -> x = 1
AFTER   degree, snapshot and health: x -> x = 0         (the graph keeps the edge)
```

## Decisions

1. `Graph::edges_with_others(id, Direction)` is the edges linking an entity to another one: the one self-edge rule. `Graph::degree` counts them.
2. The entity snapshot's edge counts, in total and by edge type, both directions, are those edges. Every `no_incoming_edges` rule, W012 and the pass input's counts agree with the degree.
3. The graph and the snapshot's edge list still hold the self-edge: the cycle rules (E007, W045) and W061 read it. A derived field makes none (unchanged).
4. A reference an entity writes to itself keeps giving its field its link (ADR 0232 D1 confirmed). `no_edges` and `no_outgoing_edges` are unchanged; W061 is the self-reference's one report, as E003 is an unresolved reference's.
5. The product guest's `health` (`unreferenced_counts`, completeness' milestones with references) applies the same rule over the command graph, as `term-density` does.

Rejected: a self-edge counting everywhere (reopens ADR 0174 D2); a reference to itself giving no link (I010 and W061 would both report one written token, the cascade ADR 0232 rejected); an SDK helper for three call sites; a protocol bump (the field and its type are the same, the host has always decided the value).

## Consequences

- W002, W041, W044, W131, W134 and any extension's `no_incoming_edges` rule, and W012, report an entity only it references. No tracked project holds a self-edge, so no dogfood report moves.
- The pass input's counts leave out a self-edge: shape and version unchanged, the protocol docs say so. A third-party guest reading them sees one less for a self-referencing entity.
- `product health` scores and `unreferenced_counts` change for such an entity.
- Re-vendor: the protocol-types docs and the product guest are blob inputs.
- Out of scope: `chains_to r_self` has no W061; the VS Code webview's own degree count (plan 05).

## What would reopen it

- A rule about edges as built, one that wants `x -> x` to count.
- A guest that needs the raw count beside the edge list it already has.
