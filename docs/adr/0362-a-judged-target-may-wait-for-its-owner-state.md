# A judged target may wait for its owner state

**Status:** accepted (2026-11-15). **Amends** [ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) D1
(a target may wait for an owner state, beside `unless_set`) and builds on the owner state of
[ADR 0329](0329-code-core-holds-test-facts-and-their-ids.md) D4, which [ADR 0350](0350-a-fields-checks-may-wait-for-its-owner.md)
already lets a field's checks wait for. Protocol 2.2.0, unreleased: the addition goes into the 2.2.0 row, no bump. The
wire and the SDK change, so the builtins are re-vendored. Extended by [ADR 0363](0363-a-codes-findings-may-wait-for-its-owner-state-and-a-write-that-reveals-one-adds-no-error.md) to a code: its findings may wait for
an owner state, and the write that reveals one adds no error (E080). Extended by
[ADR 0377](0377-a-verdict-is-fresh-over-what-it-cites-and-owed-by-its-owner-state.md) with new users of `judged_while`:
E501's targets and W525 wait for a complete owner, the plan checks for an open one, W302 for a closed threat.

## Context

- A judged rule reports on every entity of each kind it targets. The only way to skip one is `unless_set`: an entity
  that writes one of the named fields owes no verdict. Nothing else can say "not yet".
- Some targets owe a verdict only once their owner is complete, and that state is computed. `@gsd/core`'s invariant
  (ADR 0358 D10) is pending until its `holds_from` phase has run; its kind's `complete_when` already makes the kernel
  treat it as pending, so its `proof` and `items` are not checked. Its judged rule, `gsd/I536`, still reported every
  pending invariant as owing a verdict while the prompt that judges it listed only the invariants in force, and under
  `gsd-plus` a stale early verdict on a pending one refused every scoped gate (gsd/E516).
- A selector's kind filter (ADR 0356 D13) keeps only the entities whose authored field holds a value, but it filters
  what an input reads, not which entities owe a verdict, and it refuses a computed field: a digest is taken before the
  compute passes.
- Judged rules are evaluated before the compute passes, which read every judgement (a computed status may follow a
  verdict), and an owner state is decided on the merged snapshot, after them.

## D1. `judged_while` is a property of the target

`JudgedTarget.judged_while` is `complete` or `pending`, an `OwnerState` word kept a string, so an unknown word costs
the rule, not the describe payload. The SDK writes it with `JudgedTargetBuilder::once_complete()` (`complete`) and
`while_open()` (`pending`), the words a field's `checks_while` takes. Absent, a target is judged in either state.

## D2. Owner state, after the compute passes

- An entity's owner state is ADR 0329 D4's: its kind's `complete_when`, else its nearest scope ancestor's, read on the
  snapshot the compute passes' values are merged into, so a computed status decides
  (`EntitySnapshot::is_pending`).
- The rule is evaluated on every entity of the kind, as before, and the compute passes read every judgement. Once the
  values are merged the evaluation is settled: an entity in the other state owes no verdict. The rule says nothing on
  it, its judgement is not kept, so a check pass, a guard, a gate and every output (inspect, query, the export, the
  graph a command receives) see none, and `query --inputs` answers its inputs with the state `exempt`. W168 stays: it
  is about the written verdict. I205, I207 and E045 count only the judgements kept. The baseline is settled the same
  way, on the commit's snapshot and its compute passes' values.
- A verdict recorded while an entity owes none is kept in its block and its ledger line: it is read as soon as the
  owner state is reached, fresh or stale as its inputs say. Judging early is allowed, not asked for.

## D3. W112

The rule is W112 and not registered when `judged_while` is neither word, when neither the target kind nor a kind
along its `scoped_by` chain declares `complete_when` (no owner state can decide it: the static walk of ADR 0350 D6),
or when the rule's `values` or `outcome_fields` bind a field of that kind: a fresh verdict's values are given before
the compute passes, and the owner state is known only after them.

## Rejected

- A field filter on the target (`kind[field=value|…]`, a computed field allowed): it would restate the owner state
  every kind that waits already declares in `complete_when`, and two declarations of "pending" could disagree.
- Settling before the compute passes: a computed owner state is not known then, and the first pass to read the
  verdict would be the one that decides whether it is owed.
- Hiding the judgement from the compute passes too: a pass that computes a status from a verdict would read none for
  an entity whose status the same pass decides.
- Refusing a verdict on an entity that owes none: judging early is harmless, and the verdict is ready when it is owed.

## Would reopen it

- A target that needs to wait for a state other than its owner's.
- A rule that must bind a verdict value on a kind that waits.
