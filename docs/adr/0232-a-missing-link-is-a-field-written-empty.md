# A missing link is a field written empty

**Status:** accepted (2026-10-23, architecture round 26, plan 03). Confirmed by [ADR 0270](0270-an-edge-from-an-entity-to-itself-counts-in-no-edge-count.md) (a written reference to itself has its link). Follows [ADR 0117](0117-a-reference-is-decided-in-the-linker.md)
D2 and D5. Amends [ADR 0015](0015-read-views-are-operations-over-the-project-view.md) D5 (what a dangling edge was)
and [ADR 0060](0060-edge-types-resolved-once.md) D4 (what an outgoing edge rule counts).

## Context

A trace flagged an expected field as missing when no edge from the entity carried its label. The linker makes no
edge for a reference that does not resolve (ADR 0117 D2), so a behavior writing `features [nope_feature]` was traced
as missing its `features` link on `trace` (JSON and human), `specforge.trace` and the trace prompt, beside `check`'s
E003 for the same token. The review document (ADR 0152 D7) and the A010 pass read the written reference and listed
it. The trace's doc and its only test of the case assumed an edge to an undeclared target, which the linker has
never made.

The same token fired its field's edge rule too: W001 "does not implement any feature" beside the E003, and across
the builtins W043, W046, I067, I055, I082, I083 (`no_outgoing_edges`) and I010 (`no_edges`), because those rules
counted edges (ADR 0060 D4). One mistyped id was two reports, and a compiler does not report the cascade of an
error it already reported. The product spec and the catalog had written the cascade down as the rule ("or only
references that don't resolve").

## Decisions

**D1. An expected field has its link when the entity writes a reference in it or an edge from the entity carries
its label.** The written reference is read by `Reading::references` (ADR 0093), resolved or not; an inherited
`defaults` value is written, an empty list is not. The edge half keeps a derived field's link (a port's `types`
from its method signatures). A missing link is an expected field with neither.

**D2. A reference that does not resolve is reported once, by the linker** (E003; I004 when its target kind is not
loaded). It is never a missing link, never a trace prompt gap, and never fires an edge rule. A wrong-kind
reference (E022) has its edge (ADR 0117 D5) and its link.

**D3. One home: `Graph::linked_fields(forms, node)`.** The fields an entity gives their link, by D1. The trace
reads it per expected field; the entity snapshot types it by edge type (`edge_type_of`, the one place an edge is
typed) into `EntityRecord::linked`, host-side only (the wire's edge counts do not change).

**D4. An outgoing edge rule reports an entity that points at nothing.** `no_outgoing_edges` fires when the entity
gives no field (of its edge type, when set) its link; `no_edges` when no edge points at it and it gives no field
its link. A field written empty, or not written, still fires them; a reference that does not resolve does not:
its E003 is the one report. Amends ADR 0060 D4 for the outgoing side. The check kinds keep their names (the
protocol's terms); their docs say this.

**D5. The incoming and cycle rules are unchanged.** A reference that does not resolve names no entity, so no
`no_incoming_edges` report can be attributed to it (a `did_you_mean` is a guess), and it closes no cycle. A
diagnostic is a cascade when it is about the entity that wrote the error and clears when the error is fixed to
any valid target; only the outgoing rules' are.

## Rejected

- Reading only the written field: a derived field writes nothing and has its link.
- An edge for an unresolved reference: reopens ADR 0117 D2, and the degree, W061 and rename read edges as resolved.
- A graph row list shared by the review document and the trace: the trace and the edge rules need one answer per
  field, the review document rows with their annotations.
- The composition (written reference ∨ edge) inline in the trace and again in the record or the check: two copies
  of one rule in two crates.
- Recording only the unresolved edge types in the record: the same rule written a second time.
- The links on the guest wire (`PassEntity`): no reader asks, and it re-versions the protocol.
- Suppressing the incoming rules too: the target of an unresolved name is unknown.

## Consequences

`trace`, `specforge.trace` and the trace prompt list fewer missing links for an entity whose expected field writes
only unresolved references; their shapes do not change. The trace test of the case compiles a project. `check`,
`validate`, the diagnostics resource and LSP publish report one mistyped id as E003 alone: none of the eight
builtin edge rules beside it. `EntityRecord` gains `linked`; the protocol's `CheckKind` docs change (a
re-vendor), its terms do not.

## What would reopen this

An outgoing edge rule whose report a project needs beside an E003 (a rule about edges as built, not about what
the entity wrote), or a third reader that wants per-field link rows rather than the set.
