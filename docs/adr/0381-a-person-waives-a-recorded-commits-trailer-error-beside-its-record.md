# A person waives a recorded commit's trailer error beside its record

**Status:** accepted (2026-11-20). **Amends**
[ADR 0380](0380-a-gsd-task-trailer-names-its-task-by-its-plan-or-by-its-full-path.md): its Limits item that a commit
recorded without its task keeps its `linked_by` and its `gsd/E521`, and its Consequences item that `gsd/E521`'s
suggestion says the commit judged keeps its error; and
[ADR 0351](0351-a-plan-lists-its-decisions-and-the-threads-gaps-have-codes.md) D6: `gsd/E521` is judged on every
recorded commit with the fault. Builds on
[ADR 0301](0301-a-human-attested-command-runs-only-for-a-person-at-a-terminal.md) (a human-attested command runs only
for a person at a terminal), [ADR 0287](0287-who-writes-a-field-is-declared.md) (who writes a field is declared: one
writer per harness field), [ADR 0303](0303-a-collectors-plan-is-judged-for-host-errors-only.md) (a collector's plan is
judged for host errors only),
[ADR 0379](0379-gsd-strict-gates-fresh-verdicts-traced-requirements-and-task-trailers-and-is-measured.md) (gsd-strict
turns the trailer gate on) and [ADR 0370](0370-gsd-core-holds-exactly-gsd-cores-kinds-and-fields.md) D1 (the waiver is
a harness value written by a guarded command, not an authored field, so it adds no provenance row). No protocol
change, no kernel change, no new code and no new setting.

## Context

- **Under gsd-strict a recorded commit keeps its error for ever.** gsd-git records a commit once, keyed by its 12-hex
  name, and never rewrites what it recorded: it only backfills `items_changed`, once, onto a commit it recorded
  before item facts were served. A commit recorded before it carried its `Gsd-Task:` trailer keeps its
  `linked_by title` (or its missing task) and so its `gsd/E521`, and gsd-strict refuses the project for it.
- **The two ways out today are both wrong.** Rewriting history does not help: an amended commit is a new hash,
  recorded beside the old one, whose error stays; and the rewrite changes every clone's history. Turning
  `specforge.task_trailers` off silences every commit, the later ones too, so the gate that caught the fault stops
  catching the next one.
- **What is missing is a recorded decision.** A person who has looked at one commit and accepts its fault needs to say
  so once, with a reason, in a way that does not touch what gsd-git recorded and that an agent cannot do for itself.

## D1. A commit carries a `waiver` block

The commit kind gains a last field, `waiver`: a block of `reason`, `by` and `at`, each required. It is harness-written
and immutable, set once beside the fields gsd-git recorded, which it never changes; gsd-git's own one-time
`items_changed` backfill (ADR 0352) is the one field a later collect adds beside it. The collector never writes it: no
plan of gsd-git sets, creates or removes a `waiver`. The block shares one declaration with the broken window's waiver,
so a waiver reads the same wherever a person records one.

The reason: a waiver is a fact about the record, not part of it; set beside it, the record stays what the log said.

## D2. `specforge gsd waive-trailer <commit> <reason> <by>` writes it

- **Human-attested.** The command runs only for a person at a terminal (ADR 0301); an agent's call is refused
  `human_only` and writes nothing. An agent that could waive the gate would defeat it.
- **`by` is the person's own typed name**, as for every human-attested command: the host passes a command no actor,
  so `by` is never defaulted, derived from the host or filled in by an agent. `at` is the host's clock.
- **The commit is named by the 12-hex name `gsd/E521` prints**, resolved as a graph id (exact, or a unique
  `/`-boundary suffix, which a commit id has none of), never as a path or a git revision.
- **It refuses**, writing nothing: an unknown commit; a blank reason or `by`, or one that holds a control
  character other than a tab or a line feed; a commit already waived, before the host's immutability error would
  refuse it; and a commit `gsd/E521` does not judge.
- **It writes one value** through the plan file that already holds the commit, and the host writes its one ledger line.

The reason: the person who looked at the commit is the only one who can accept its fault, and the record says who,
why and when.

## D3. gsd/E521 is not judged on a waived commit

A commit whose `waiver` is set is not judged. The pass and the command read one rule of what `gsd/E521` judges (a
commit of a plan, not a `docs` one, whose task its subject's title found, or whose type is a task type and that names
no task), so the command cannot waive a commit the pass would not judge, nor the pass judge one the command refused to
waive. A waiver does not depend on the setting: it holds when a profile turns the gate on later.

The reason: one rule in one place cannot disagree with itself.

## D4. gsd/E521's suggestion names the waiver

Beside the advice to add the trailer to the task's later commits, the suggestion names
`specforge gsd waive-trailer` with the judged commit's own name, for the commit already made.

The reason: the diagnostic that reports a fault a person cannot fix by editing says how to accept it.

## Rejected

- **Amending and rewriting history.** gsd-git records a commit once by its hash; a rewrite records a new commit beside
  the old one, whose error stays, and changes every clone's history.
- **Dropping the error by profile or setting.** It clears every commit, not the one a person looked at, and hides the
  next fault.
- **An authored `.spec` entity that names the commit.** A second place to look, written by whoever edits the file, not
  attested, and a kind gsd-core has no reason for.
- **A waiver for any code.** This decision is `gsd/E521`'s only: its fault is the one a later edit cannot fix.

## Limits

- `by` is a person's claim, attested only as far as ADR 0301's terminal proof goes.
- A waiver is final: the block is immutable, as the broken window's is. A wrong waiver stays, with who gave it, why
  and when.
- A waiver typed into the `.spec` by hand has no ledger line, and the host reports it (`E083`): only the command
  records one.
- A commit whose hash has left the log is still recorded (ADR 0380's "Would reopen it" item stands).

## What each decision becomes

"Superseded" means the decision no longer holds. "Kept" means it holds as written. "Amended" means it holds with the
change named.

| ADR | Decision | Outcome | Why |
|---|---|---|---|
| 0380 | Limits (the recorded commit's error) | amended | the commit keeps its fields, and a person may waive its `gsd/E521` beside them (D1, D2) |
| 0380 | Consequences (the suggestion's last clause) | amended | the suggestion names the waiver for the commit already made (D4) |
| 0351 | D6 | amended | a waived commit is not judged (D3) |
| 0379 | gsd-strict's trailer gate | kept | the gate stays on; a waiver clears one commit, not the gate |
| 0303 | a collector's plan judged for host errors only | kept | the collector never writes the waiver (D1) |
| 0301 | human-attested commands | kept | one more human-attested command (D2) |

ADR 0380: 2 amended. ADR 0351: 1 amended. ADRs 0379, 0303 and 0301: 1 kept each.

## Consequences

- `gsd/E521` under gsd-strict has a way out that is a recorded decision, not a history rewrite and not a profile
  switch.
- The human-attested commands are seven.
- The writer roster gains one row: the commit's `waiver`, written by `waive_trailer`.
- `specforge gsd help` and the MCP tool list name the command; the tool refuses an agent.
- The `@gsd/core` blob is re-vendored.

## Would reopen it

- A waiver another code needs.
- An adopter who must withdraw a waiver.
- A host that passes a command the person's identity, which would replace the typed `by`.
