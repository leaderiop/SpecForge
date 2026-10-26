# Spec writing flow

A writing order for a software project that uses the builtin extensions (`@specforge/software`, `product`,
`governance`, with `testing` for proof). It is one method among many: SpecForge itself has no writing order,
because it has no domain. Another extension set implies its own order. A GSD-style method starts from milestones
and phases; a quality team starts from procedures.

The order is the one the [authoring tutorial](guides/authoring-spec-files.md) follows:
**foundation → data → guarantees → boundaries → behavior → product → governance**. Each step only references
what earlier steps declared, so `specforge check` stays clean as you go. Run it after every step.

```
 foundation ─► data ─► guarantees ─► boundaries ─► behavior ─► product ─► governance
 spec, term    type    invariant     port          behavior    feature    decision
                                     event                     journey    constraint
                                                               persona    failure_mode
                                                               channel    milestone
                                                               deliverable, module, release
```

Cross-cutting: `ref` attaches an external reference (an issue, a design, a document) at any step.

---

## 1. Foundation: what is this project, what do we call things?

- `spec`: the project's identity and version. Extensions are listed in `specforge.json`, not in the spec.
- `term` (product): the words the team uses with one meaning.

**Checkpoint:** `specforge check` passes on the empty project.

## 2. Data: what shapes exist?

- `type` (software): the data shapes the domain needs, with their fields.

**Checkpoint:** every type is used by something later, or W002 (unreferenced type) says so.

## 3. Guarantees: what must always be true?

- `invariant` (software): a rule that must never be broken, with its `risk` and what proves it (`verify`).

**Checkpoint:** each invariant will be listed in a behavior's `invariants` in step 5; until then W003
(unenforced invariant) reminds you.

## 4. Boundaries: how do parts talk to each other and to the outside?

- `port` (software): an interface to another component or an external system.
- `event` (software): something the system announces, with its `payload` type.

**Checkpoint:** W005 (unreferenced port) and W007 (event never produced) clear once behaviors use them.

## 5. Behavior: what does the system do?

- `behavior` (software): one operation's contract, linked to the types, ports, invariants and events it uses
  (`types`, `ports`, `invariants`, `produces`), with a `verify` line per obligation.

**Checkpoint:** no E003 (unresolved reference); W003, W005 and W007 from earlier steps are gone; with
`@specforge/testing`, W004 lists entities with nothing to prove them yet.

## 6. Product: who is it for, and what ships?

- `persona`, `channel`: who uses it, and through what.
- `feature`: a unit of value, implemented by behaviors (a behavior lists it in `features`).
- `journey`: a persona's flow on channels, through features.
- `module`, `deliverable`, `milestone`, `release`: how it is built, shipped and scheduled.

**Checkpoint:** W001 (behavior implements no feature), W008 (unimplemented feature), W041 (unreferenced feature),
W042 (unreferenced journey), W043 (deliverable without journeys), W044 (unreferenced module) and E007 (module
dependency cycle) are clear.

## 7. Governance: why, within which limits, and what can go wrong?

- `decision`: a significant choice, its context and its consequences.
- `constraint`: a limit (performance, security, compliance) on what the system does.
- `failure_mode`: what can go wrong (its `cause` and `effect`), how severe it is, and the invariant it
  threatens.

**Checkpoint:** `specforge check --strict` passes: no errors, no warnings.

---

## Iterate

The order is a first pass, not a rule. Writing behaviors reveals new terms and types; writing journeys reveals
missing features. Go back, add them, and check again. A relation is always written on the field that owns it
(E079 says when it isn't), and `specforge trace <id>` shows the other side.

## What the check catches along the way

| Code | What it catches | From |
|---|---|---|
| E002 | Two entities with the same id | core |
| E003 | A reference to something that doesn't exist | core |
| E006 | A required field left out | the extension declaring the field |
| E079 | A relation written on its other side | core |
| W113 | Files that import each other in a cycle | core |
| W001, W002, W003, W005, W007, W008 | Behaviors, types, invariants, ports, events and features nothing connects | `@specforge/software` |
| W041–W044, E007 | Features, journeys, deliverables and modules nothing connects; module cycles | `@specforge/product` |
| W004 | An entity with obligations but no proof | `@specforge/testing` |

`specforge explain <code>` prints the full text of any code; [diagnostics.md](diagnostics.md) lists them all.
