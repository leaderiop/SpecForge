# Invariants

```
 invariant ─*──0..1─ phase             (holds_from: the phase it holds from)
   ├─*──*─ test                        (proof: the `@code/core` tests that prove it)
   └─*──*─ item                        (items: the `@code/core` code items it holds over)

 phase ─*──*─ invariant                (invariants_due, computed: the invariants it owes a judgement)
```

Files: `spec/gsd/governance/invariants.spec`.

## invariant

What must always hold in the code, from a phase on: the statement, the tests that prove it and the code items it
holds over. Name: a word, unique in the project (`entity_id_uniqueness`). Unscoped, like `broken_window`: an
invariant outlives the phase it starts at.

| Field | Type | Flags | Meaning |
|---|---|---|---|
| statement | str | R | what must always hold |
| description | str | | a summary beside the statement |
| scope | enum{project, milestone} | | how far it reaches: the project (absent), or the milestone of its `holds_from` phase ([the window](#when-an-invariant-is-due)) |
| holds_from | → phase | | in force once this phase has been complete (executed or after) or verified, and for good; absent: from the start |
| proof | →* test | | the tests that prove it: `@code/core` test facts, quoted (`"js:pkg/a.test.js#adds"`) |
| items | →* item | | the code items it holds over: `@code/core` item facts, quoted (`"js:pkg/src/a.js::add"`) |
| criticality | enum{critical, high, medium, low} | | what a violation costs, highest first |
| tags | strs | | words to group it |
| refs | strs | | external references: issue or document URIs |
| status | enum{pending, unverified, stale, holds, violated, uncertain} | C | `pending` until its `holds_from` phase has been complete or verified; then `unverified` with no usable verdict, `stale` once an input moved since, or the fresh verdict's outcome |
| last_verified | ts | C | the time of the fresh verdict that it holds; absent otherwise |

`proof` and `items` read `@code/core`'s facts as a truth's `proof` and a task's `items` do: a name no served store
holds is `code/E101`, and with no `@code/core` loaded a written one is I004.

An invariant reads back the phases that owe it a judgement (`phase.invariants_due`, computed) as `due_in`, a name it
never writes: writing it is E079 ([ADR 0359](../../../docs/adr/0359-requirement-and-decision-fields-absorb-feature-constraint-and-decision.md)).

### In force, and its status

An invariant is in force from the start when it names no `holds_from`, else once that phase has been complete in
the phase's own sense: `executed`, `needs_review` or `complete`, the values of the phase's `complete_when`. Executed is
enough; its verification is not awaited. A `holds_from` that names no phase (the kernel reports it) keeps the
invariant pending.

In force is reached once and stays. GSD's gap closure (`plan-phase N --gaps`) adds a plan to a phase whose
verification found gaps, and the phase is `in_progress` again until that plan has a summary: the invariant stays in
force, so its verdict still shows and its `proof` is still checked, while the gaps are open. The status is computed and keeps no history, so the latch is read from what outlives the slide: a phase
with a recorded verification has run, whatever the verification found. A phase that gains a plan before any
verification is still being executed, and its invariant waits for it.

While it is not in force its `status` is `pending`, whatever a verdict says, and the invariant is pending in the
kernel's sense (its `complete_when` holds on every other status): its `proof` and `items` are not checked, so a test
or an item the phase has yet to write is no finding. In force, its `status` reads its verdict:

| Verdict | status |
|---|---|
| none, missing, unrecorded or unreadable | unverified |
| stale (an input moved since) | stale |
| fresh | its outcome: holds, violated or uncertain |

`last_verified` is the `at` of the verdict while `status` is `holds`, and absent otherwise. `scope` changes no
status: it bounds the window of the commits that can make the invariant due ([When an invariant is
due](#when-an-invariant-is-due)).

### Its verdict

The verdict is the judged rule `gsd/I536` ([rules.md](rules.md#governance)): outcomes `holds` (passing), `violated`
and `uncertain`, judged by the prompt `verify_invariant`, over two inputs, the invariant itself and its tests (its
proof's results and the tests whose name tags it, as a truth's). An invariant in force without a fresh `holds`
reports it at Info under every profile. The prompt lists the invariants in force that do not hold and those a phase
owes a judgement ([When an invariant is due](#when-an-invariant-is-due)), reads each one's code items, and records
with `specforge gsd record-verdict <invariant> I536 <outcome>`, citing the source files it read, so an edit to one
stales the verdict.

A pending invariant owes no verdict: the rule waits for it to be in force (`judged_while complete`: the kind's
`complete_when`, read after the compute passes). It is not reported, no gate reads a verdict of it (gsd/E516), and the
prompt does not list it; with only pending invariants the prompt says how many wait. A verdict recorded early, with
`verify_invariant --invariant <id>`, is kept and read once the invariant is in force. If a commit of its window
changed what it guards after that verdict, it is due the moment it is in force: the write that brings it into force
(its `holds_from` phase's verification, say) is not refused for that `gsd/E531`, which is reported once the write
lands, and judging it again clears it.

### When an invariant is due

A phase owes an invariant a judgement once one of its commits changed code the invariant guards after the invariant
was last judged ([ADR 0358](../../../docs/adr/0358-a-risk-is-rated-and-an-invariant-is-judged.md#e531-is-a-pass-over-the-verdict)).
The phase lists it in its computed `invariants_due` ([project.md](project.md#phase)).

- **What it guards.** Its `items`, as each reference resolves (a rooted `/…` spelling is the item) and as written; the
  items its tests run as (`@code/core`'s `runs_as`, from an item to the test it is the function of), its tests being
  those its `proof` names and those whose name tags it, run or not; and the items whose name tags it.
- **The window.** With no `holds_from`, every phase's commits. Otherwise the `holds_from` phase, a later-numbered
  phase of its milestone and, where numbers do not order two phases (another milestone, a custom-named or a backlog
  `999.x` phase), a commit no older than the first commit recorded for the `holds_from` phase (until it has one, any
  commit: the verdict alone bounds it). `scope milestone` keeps only the `holds_from` phase's milestone; with no
  `holds_from` it reads as `project`. A commit is placed by its author time and its phase by its plan's: a commit
  whose author time is in the future (a clock ahead, a date set by hand) stays due, and judging again does not clear
  it, until that time has passed; a quick plan's commit is in no phase and is not judged.
- **Covered.** Any `gsd/I536` verdict recorded at or after the commit, fresh or stale, whatever its outcome: recording
  a verdict only clears a phase; editing the invariant changes which commits can make it due (what it guards, its
  window), never the verdict that covers one. A verdict records the second it was given, so a commit authored later
  the same day is due; a verdict recorded with a day for its `at` (by a host before verdicts carried their second)
  covers that whole day. An `at` edited by hand leaves the verdict unrecorded (gsd/I536 reads it as such, E083) and it
  covers nothing.
- **Owed in force, once judged.** A pending invariant owes nothing, as it owes no verdict ([Its verdict](#its-verdict)),
  and an invariant in force never judged owes nothing either: `gsd/I536` reports its missing verdict. `gsd/E531`
  waits for the invariant to be in force (its code's `reported_while`, [ADR 0363](../../../docs/adr/0363-a-codes-findings-may-wait-for-its-owner-state-and-a-write-that-reveals-one-adds-no-error.md)),
  so the write that brings a due invariant into force reveals it rather than adding it, and is accepted; a write that
  widens what an invariant in force guards over a changed phase adds it, and is refused (E080).
- **The fallback.** A commit whose `items_changed` is unwritten (no item facts were served then, or a file's lines
  were unknown) is matched by the files it touched, against the files of the guarded items and tests. A written empty
  `items_changed` changed nothing. A guarded test no item runs as (a JavaScript project's: its source pack writes no
  `test_id`) is matched by its file whatever the commit wrote, so a commit that changes only a proof test is due.

One function, `passes::invariants::due`, joins the recorded commits with the guarded code; `invariants_due` and
`gsd/E531` ([rules.md](rules.md#governance)) read its rows. E531 is an Error under every profile, one per invariant and
phase, on the invariant: its message names the phase, its suggestion the prompt to run (`verify_invariant --invariant
<id>`), each commit and what it changed or touched. A due invariant usually still `holds` (its verdict reads no code it
did not cite), so the prompt run bare lists every invariant a phase owes a judgement too, whatever its status, with the
phases that owe it. Judge it again and the phase is no longer due. A reader of which items a test exercises (coverage)
would add one source to what an invariant guards, and every reader of those rows would follow unchanged.

### The old invariant's fields

`@specforge/governance` declared an `invariant` too; loading it beside `@gsd/core` is E026. Each of its fields has a
home here:

| Old field | Here |
|---|---|
| guarantee | statement |
| risk (critical, high, medium, low) | criticality, the same words in the same order |
| description | description |
| refs | refs |
| verify (`@specforge/testing`) | proof |

Its presentation (icon, colours, inference guide) has no home: `@gsd/core` declares none for any kind.
