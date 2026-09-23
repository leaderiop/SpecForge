# 081 — Bertrand Meyer

**Cluster:** C10 — Formal methods (@specforge/formal + analyze passes)
**Roster role:** Design by Contract; client/supplier obligations
**SpecForge anchors:** `requires`/`ensures` blocks on behaviors, W036–W040 condition checks, RES-25 Part I (DbC)

## Why this engineer
RES-25 crowns Design by Contract the highest-ROI formal feature in SpecForge — and `requires`/`ensures` on behaviors, plus `maintains` on invariants, are Meyer's clauses verbatim. The condition_check pass implements his contract theory as diagnostics: W036 (port precondition stricter than behavior = violated client/supplier symmetry), W037 (unverifiable condition), W038 (unreachable postcondition), W039 (redundant precondition), W040 (invariant without a formal property). Meyer's rule that preconditions belong to callers, not callees, is precisely the port↔behavior compatibility logic.

## References for SpecForge
**Key works**
- [Object-Oriented Software Construction, 2nd edition](https://bertrandmeyer.com/OOSC2) — Prentice Hall, 1997 (full text free). Ch. 11 is the DbC bible: obligations/benefits, invariant maintenance, exception discipline — the semantics SpecForge's condition blocks borrow.
- **Applying "Design by Contract"** — IEEE Computer 25(10), 1992. The compact practitioner formulation behind RES-25's DbC row.
- [OOSC, 3rd edition (in progress)](https://bertrandmeyer.com/oosc3) — living revision; tracks where contracts meet modern tooling.
- [Eiffel.org](https://www.eiffel.org) — the language that shipped DbC first; pre/post/invariant syntax survivors still cite.

## Study first
1. OOSC2 ch. 11 — obligation asymmetry behind W036
2. "Contract archeology" / inheritance contract rules — how port-vs-behavior composition should weaken/strengthen
3. RES-25 §1 — the documented mapping from DbC terms to SpecForge entities
