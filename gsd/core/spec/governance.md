# Risks and invariants

```
 risk ─*──*─ task | truth          (mitigated_by: what controls it)
   ├─*──*─ requirement             (affects: what it threatens; the requirement reads `risks`)
   └─*──0..1─ milestone            (milestone: where it must be settled; the milestone reads `risks`)

 invariant ─*──0..1─ phase         (holds_from: the phase it holds from)
   ├─*──*─ test                    (proof: the `@code/core` tests that prove it)
   └─*──*─ item                    (items: the `@code/core` code items it holds over)
```

Files: `spec/gsd/governance/risks.spec`, `spec/gsd/governance/invariants.spec`.

GSD keeps no risk file: this kind is beyond parity
([ADR 0358](../../../docs/adr/0358-a-risk-is-rated-and-an-invariant-is-judged.md)), and no `gsd-compat` rule needs it.
A risk is rated as FMEA rates a failure mode, severity, occurrence and detection each from 1 to 10, and SpecForge
computes what the ratings give: an rpn is never written by hand.

## risk

What can go wrong. Name: by convention `RK1`; any identifier is valid. Unscoped, like `broken_window`: a risk outlives
a phase, and moving it to another milestone does not rename it.

| Field | Type | Flags | Meaning |
|---|---|---|---|
| cause | str | R | what makes it happen |
| effect | str | R | what breaks when it does |
| severity | int{1..10} | R | 1 no effect … 10 hazardous, without warning |
| occurrence | int{1..10} | R | 1 remote … 10 almost certain |
| detection | int{1..10} | R | 1 almost certainly caught before harm … 10 cannot be caught |
| post | {severity int{1..10}, occurrence int{1..10}, detection int{1..10}} | | the ratings once every mitigation is done; a member left out keeps its pre rating |
| mitigated_by | →* task \| truth | | what controls it: a task done, a truth verified |
| mitigation | str | | the planned control as prose; no evidence |
| affects | →* requirement | | the requirements it threatens; each reads it back as `risks` |
| milestone | → milestone | | where it must be settled when no phase's plan mitigates it; the milestone reads it back as `risks` |
| tags | strs | | words to group it |
| description | str | | free prose |
| refs | strs | | external references |
| rpn | int | C | severity × occurrence × detection, 1 to 1000 |
| post_rpn | int | C | the same product over the post ratings; `rpn` without a post block |
| status | enum{open, mitigating, mitigated} | C | `open` while it names no mitigation; `mitigated` once every one it names is done; else `mitigating` |
| blocking | bool | C | its current rpn (`post_rpn` once mitigated, else `rpn`) is at or above `specforge.risk_block_on`; without a `post` block `post_rpn` is the `rpn`, so a mitigated risk stops blocking once post ratings bring it under |

Each computed value is absent while a required rating is missing or one is outside 1 to 10: the kernel reports that
(a missing required field, E061 out of bounds, in the post block too), and no number is guessed. Writing a computed
field is E082.

`mitigated_by` names no reverse: the kernel reads a reverse name through a field's one target kind, and this field
has two.

### What mitigates a risk

A risk is `mitigated` once every item of its `mitigated_by` is done:

- a task, when a recorded commit names it or its plan is `complete` (the rule that gives the task its `done`);
- a truth, when its phase's verification verifies it (an override, or a fresh passing `gsd/E501` verdict that no
  recorded test contradicts); a phase's verifications are read as one, so an override on any of them counts. A truth
  no verification assesses is not verified.

A risk is unscoped, so an item names a task or a truth by its full path, rooted or not (`v1.0/01/01-01/T1` or
`/v1.0/01/01-01/T1`). A name that does not resolve is never done (the kernel reports it, E003), even when another
item's path ends with it (`T1` beside `v1.0/01/01-01/T1`), so a risk naming one stays `mitigating`. The
prose `mitigation` is no evidence: a risk that names nothing in `mitigated_by` stays `open` whatever its prose says.

### Blocking

`blocking` reads the risk's current rpn: `rpn` until it is mitigated, `post_rpn` after. A mitigated risk whose residual
is still at or above the threshold keeps blocking, and an optimistic post block lowers nothing before its mitigations
are done. Without a `post` block `post_rpn` is the `rpn`, so a risk whose every mitigation is done still blocks at the
same number: write the `post` ratings the done mitigations earn to record the residual. Nothing guesses them, and no
diagnostic is raised: `status` `mitigated` beside `blocking` `true` says it. The threshold is `specforge.risk_block_on` ([settings.md](settings.md#specforge-only-settings)), an
integer from 1 to 1000, 100 by default.

gsd/W532 ([rules.md](rules.md#governance)) warns on a risk whose `rpn` is at or above the threshold and that names
nothing in `mitigated_by`: rate it again, or name what controls it. It is silent below the threshold and once anything
is named, done or not (`blocking` says whether it still blocks).

### The ratings

Each rating is an integer from 1 to 10, as the FMEA scales give it. Detection runs the other way: 10 is a failure no
control catches.

| Rating | 1 | 10 |
|---|---|---|
| severity | no effect | hazardous, without warning |
| occurrence | remote | almost certain |
| detection | almost certainly caught before harm | cannot be caught |

## Risk on phases and milestones

A phase and a milestone carry a computed `risk_level` ([project.md](project.md)): how risky they are now.

- A risk *reaches* a phase when a task or a truth its `mitigated_by` names belongs to a plan of that phase. A quick
  plan is in no phase, so a risk it mitigates reaches none.
- A phase's `risk_level` is the highest current rpn (`post_rpn` once mitigated, else `rpn`) of the risks that reach
  it, so it drops as their mitigations land.
- A milestone's `risk_level` is the highest of its phases' levels and of the current rpn of each risk whose
  `milestone` names it: a risk no phase's plan mitigates is settled there.
- Each is absent when no risk reaches it.

## invariant

What must always hold in the code, from a phase on: the statement, the tests that prove it and the code items it
holds over. Name: a word, unique in the project (`entity_id_uniqueness`). Unscoped, like `risk`: an invariant
outlives the phase it starts at.

| Field | Type | Flags | Meaning |
|---|---|---|---|
| statement | str | R | what must always hold |
| description | str | | a summary beside the statement |
| scope | enum{project, milestone} | | how far it reaches: the project (absent), or the milestone of its `holds_from` phase |
| holds_from | → phase | | in force once this phase is complete (executed or after); absent: from the start |
| proof | →* test | | the tests that prove it: `@code/core` test facts, quoted (`"js:pkg/a.test.js#adds"`) |
| items | →* item | | the code items it holds over: `@code/core` item facts, quoted (`"js:pkg/src/a.js::add"`) |
| criticality | enum{critical, high, medium, low} | | what a violation costs, highest first |
| tags | strs | | words to group it |
| refs | strs | | external references: issue or document URIs |
| status | enum{pending, unverified, stale, holds, violated, uncertain} | C | `pending` until its `holds_from` phase is complete; then `unverified` with no usable verdict, `stale` once an input moved since, or the fresh verdict's outcome |
| last_verified | ts | C | the time of the fresh verdict that it holds; absent otherwise |

`proof` and `items` read `@code/core`'s facts as a truth's `proof` and a task's `items` do: a name no served store
holds is `code/E101`, and with no `@code/core` loaded a written one is I004.

### In force, and its status

An invariant is in force from the start when it names no `holds_from`, else once that phase is complete in the
phase's own sense: `executed`, `needs_review` or `complete`, the values of the phase's `complete_when`. Executed is
enough; its verification is not awaited. A `holds_from` that names no phase (the kernel reports it) keeps the
invariant pending.

While it is not in force its `status` is `pending`, whatever a verdict says, and the invariant is pending in the
kernel's sense (its `complete_when` holds on every other status): its `proof` and `items` are not checked, so a test
or an item the phase has yet to write is no finding. In force, its `status` reads its verdict:

| Verdict | status |
|---|---|
| none, missing, unrecorded or unreadable | unverified |
| stale (an input moved since) | stale |
| fresh | its outcome: holds, violated or uncertain |

`last_verified` is the `at` of the verdict while `status` is `holds`, and absent otherwise. `scope` changes no
status.

### Its verdict

The verdict is the judged rule `gsd/I536` ([rules.md](rules.md#governance)): outcomes `holds` (passing), `violated`
and `uncertain`, judged by the prompt `verify_invariant`, over two inputs, the invariant itself and its tests (its
proof's results and the tests whose name tags it, as a truth's). An invariant without a fresh `holds` reports it at
Info under every profile, a pending one too: the rule reads no status. The prompt lists the invariants in force that
do not hold, reads each one's code items, and records with `specforge gsd record-verdict <invariant> I536 <outcome>`,
citing the source files it read, so an edit to one stales the verdict.

### The old invariant's fields

`@specforge/governance` declared an `invariant` too; loading it beside `@gsd/core` is E026, as for `risk`. Each of its
fields has a home here:

| Old field | Here |
|---|---|
| guarantee | statement |
| risk (critical, high, medium, low) | criticality, the same words in the same order |
| description | description |
| refs | refs |
| verify (`@specforge/testing`) | proof |

Its presentation (icon, colours, inference guide) has no home: `@gsd/core` declares none for any kind.
