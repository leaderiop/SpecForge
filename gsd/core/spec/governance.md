# Risks

```
 risk ─*──*─ task | truth          (mitigated_by: what controls it)
   ├─*──*─ requirement             (affects: what it threatens; the requirement reads `risks`)
   └─*──0..1─ milestone            (milestone: where it must be settled; the milestone reads `risks`)
```

Files: `spec/gsd/governance/risks.spec`.

GSD keeps no risk file: this kind is beyond parity, and no `gsd-compat` rule needs it. A risk is rated as FMEA rates a
failure mode, severity, occurrence and detection each from 1 to 10, and SpecForge computes what the ratings give: an
rpn is never written by hand.

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
| blocking | bool | C | its current rpn (`post_rpn` once mitigated, else `rpn`) is at or above `specforge.risk_block_on` |

Each computed value is absent while a required rating is missing or one is outside 1 to 10: the kernel reports that
(a missing required field, E061 out of bounds, in the post block too), and no number is guessed. Writing a computed
field is E082.

### What mitigates a risk

A risk is `mitigated` once every item of its `mitigated_by` is done:

- a task, when a recorded commit names it or its plan is `complete` (the rule that gives the task its `done`);
- a truth, when its phase's verification verifies it (an override, or a fresh passing `gsd/E501` verdict that no
  recorded test contradicts). A truth no verification assesses is not verified.

A name that does not resolve is never done (the kernel reports it, E003), so a risk naming one stays `mitigating`. The
prose `mitigation` is no evidence: a risk that names nothing in `mitigated_by` stays `open` whatever its prose says.

### Blocking

`blocking` reads the risk's current rpn: `rpn` until it is mitigated, `post_rpn` after. A mitigated risk whose residual
is still at or above the threshold keeps blocking, and an optimistic post block lowers nothing before its mitigations
are done. The threshold is `specforge.risk_block_on` ([settings.md](settings.md#specforge-only-settings)), an
integer from 1 to 1000, 100 by default.

gsd/W532 ([rules.md](rules.md#governance)) warns on a risk whose `rpn` is at or above the threshold and that names
nothing in `mitigated_by`: rate it again, or name what controls it. It is silent below the threshold and once anything
is named, done or not (`blocking` says whether it still blocks).

`mitigated_by` names no reverse: the kernel reads a reverse name through a field's one target kind, and this field
has two.

### The ratings

Each rating is an integer from 1 to 10, as the FMEA scales give it. Detection runs the other way: 10 is a failure no
control catches.

| Rating | 1 | 10 |
|---|---|---|
| severity | no effect | hazardous, without warning |
| occurrence | remote | almost certain |
| detection | almost certainly caught before harm | cannot be caught |
