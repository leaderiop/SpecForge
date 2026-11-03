# The gsd requirement thread: a truth's requirement and proof, the `Gsd-Task` trailer, one reading of a test

**Status:** accepted (2026-11-03, build-plan R1, plan 04). Amends [ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md)
D3 (a `test(..)` input), [ADR 0289](0289-an-extension-writes-through-an-edit-plan-the-host-applies.md) D4 (gsd-git
reads a trailer), [ADR 0312](0312-collect-re-keys-a-test-id-as-the-test-report-does.md) (a truth's tests are its tags
and its proof) and [ADR 0125](0125-collects-machine-is-one-port-the-caller-passes.md) D5 (a detected collector with no
evidence is skipped). Builds on [ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md) (facts)
and [ADR 0329](0329-code-core-holds-test-facts-and-their-ids.md) (`@code/core`, a kind's `result`, owner state). No
protocol change of its own; the route uses the 1.15.0 `routes` category.

## Context

R1's exit line follows one requirement to the test that proves it:

```
TASK-01 → plan 01-01 → T1 → <sha> (Gsd-Task) → js:taskboard/test/tasks.test.js#… (passed) → TR1 verified, fresh
```

At the base, `@gsd/core` could walk the first three hops and no further:

- `truth` named no requirement and no test: its tests were the results a `[gsd:<id>]` tag (ADR 0312) filed under it.
- gsd-git found a commit's task by its subject's title alone. A commit whose subject names no plan was skipped, and the
  executor's `Gsd-Task:` trailer was never read.
- `collect --yes` refused a gsd project with E058: gsd-git declared no detection.
- Five passes read "the tests of X" each its own way, with three meanings of "no test".
- A judged `test(..)` input digested a report with no result for the entity as `[]` and no report as `none`. So any
  report that linked nothing to a truth staled its verifier verdict (R-12).

## Decisions

- **D1. `truth.requirement` is one reference**, inverse `requirement.truths`. The route and E515 join on it. It is not
  required: a strict "a truth names no requirement" code is a later round's.
- **D2. `truth.proof` lists the tests that prove a truth**: a `reference_list` with `target_kind test`, written as
  quoted scheme references (`"js:…#name"`). `@gsd/core` names `@code/core` an **optional peer**:
  - without it, `proof` is still declared (schema, completion), there is no W021, and a written proof is I004 whose
    remedy names `@code/core`;
  - with it, a proof item the store holds links to its `test` fact, and one it does not is unheld: `@code/core`
    reports it (E101, W102) unless the citing truth is pending.
  - `@gsd/core` declares owner state (ADR 0329 D4): a `plan` is complete on `status ∈ [complete]`, a `task` on
    `status ∈ [done]`. A truth is scoped by its plan, so a proof written before the plan is complete is unchecked.

  A side effect of an optional peer: while any named peer is not loaded, W021 lets every target kind of `@gsd/core`
  through. A test loads `@gsd/core` beside `@code/core` and asserts no W021, so a typo still shows there.
- **D3. gsd-git reads the `Gsd-Task:` trailer first.** Its own `git log` argv carries it, in the NUL-started header and
  before the subject: `%x00%H%x1f%P%x1f%at%x1f%(trailers:key=Gsd-Task,valueonly,unfold,separator=%x1e)%x1f%s`.
  - `unfold` joins a folded value, so a continuation line cannot become a forged numstat line. The subject stays the
    last field, so a unit separator inside it stays in it.
  - A value is `<plan>/<task>`: one `/`, both sides non-empty ASCII alphanumerics, `.`, `_` or `-`. Values are trimmed,
    empty ones dropped and duplicates merged. One distinct valid value names the task; several distinct values, or any
    invalid one, is unreadable, as upstream counts a doubled `gate-status:` as missing.
  - A trailer that names a task (exactly, by `/`-suffix, or with the configured `project_code` prefix stripped from its
    plan part) fixes both the plan and the task, whatever the subject says. A revert still names no task.
  - An unreadable trailer, or one that names no task, falls back to the subject, and the plan's title notes it.
  - The commit's new harness field **`linked_by`** (`enum{Gsd-Task, title}`) says which one found its task. It is
    written only when `task` is. `Gsd-Task` is the one gsd vocabulary value that is not lower-case: it is the
    trailer's own spelling, and what the trace prints.
- **D4. gsd-git is detected by a branch ref**: `.git/refs/heads/*` or `.git/packed-refs`. A repository with a commit
  has one; an unborn repository and a linked worktree (`.git` is a file) have neither, and name `--runner gsd-git`.
  gsd-junit stays undetected: it runs no command, so a plain collect would always find its report missing.
  - **A collector chosen by detection that yields no evidence is skipped, not fatal** (amends ADR 0125 D5). Its
    `RunnerResult` carries `skipped` with the E045 message, and the other collectors' results, facts and plans are
    kept. A collector named by `--runner` still fails with E045; refusals decided before any run (E058, consent) stay
    fatal. So detecting gsd-git never turns `--no-run`, a missing `git` or an orphan branch into a failed collect.
- **D5. One module reads the tests of an entity** (`passes/thread.rs`), with one meaning of "no test":
  - **not run**: no report, or a report with no result and no `test` fact;
  - **no test**: a run that links nothing to the entity.

  The tests of an entity are its tagged results, the results its fresh verdict cites (ADR 0304) and, for a truth, the
  results of its proof's `test` facts. A failure in any source wins. Each caller maps the evidence once:

  | Caller | Not run | No test | Passed | Failed |
  |---|---|---|---|---|
  | E515 passing | false | false | true | false |
  | E502 | no finding | "no passing test is recorded for it" | no finding | "a test recorded for it failed" |
  | scan `ran` | false | false | true | true |
  | derive check status | no value | `unknown` | `pass` | `fail` |
  | audit row | `pending` | `pending` | `green` | `red` |

  The one behaviour change: E502 after a git-only collect (an empty report) is no finding, as the derive pass already
  read it. E515 also counts a passing proof of a truth whose `requirement` names the requirement.
- **D6. The kernel reads a fact's result through its kind's declared `result` field**, never a field name it knows.
- **D7. A `test(..)` input** (amends ADR 0290 D3):
  - takes several chains, `test(self, self.proof)`, and selects their union. `test(self)` prints as before;
  - reads a selected fact's own result, `[id, status]` (`unknown` when unwritten), and every other entity's report
    results, `[name, status]`;
  - is the value **`none`** when nothing is linked: judged, and compared like any digest. A recorded `none` stays fresh
    while nothing is linked, and a linked test that vanishes stales the verdict;
  - is **unjudged** when a selected entity that is no fact has no usable report, or a proof item is unheld while no
    store holds facts of its scheme (a fresh clone). An unheld item while a store holds its scheme is `[id, absent]`;
  - `Inputs.unjudged` names the unjudged inputs, and freshness skips exactly those. I205 names both causes.
  - A chain through a reference whose declared target kinds are all unloaded is **inert**: it selects nothing and
    the rule is registered. A field that does not exist, or is no reference, is still W112. This holds for every
    selector, so `test(self, self.proof)` keeps E501 in a project without `@code/core`.

  `@gsd/core`'s truth input is `test(self, self.proof)`: tags still stale a verdict, and so does the proof.
- **D8. The requirement route.** `@gsd/core` declares the route `requirement-thread` (`gsd/requirement-thread`): from
  a `requirement`, its plans (`PlanRequirements`, inverse), their tasks (`TaskPlan`, inverse) and the tasks' commits
  (`CommitTask`, inverse, annotated with `linked_by`), and its truths (`TruthRequirement`, inverse, annotated with the
  E501 judgement) and their proofs (`TruthProof`, annotated with the test's result). `specforge trace` follows it
  (ADR 0330).

## Rejected

- **Linking a test by a `[TR1]` tag in its name.** Three truths are named TR1 in the taskboard twin (W115); `proof` is a
  typed reference the graph resolves and checks.
- **`test(self.proof)` alone** (owner decision 04-U3): tags would stop staling a verdict.
- **Detecting gsd-git by `.git`** (owner decision 04-U2): an unborn repository has one, `git log` fails there, and the
  whole collect, node-test's run included, would fail.
- **`linked_by` = `trailer`** (owner decision 04-U1): lower-case like every gsd value, but the trace would print
  `(trailer)` where the trailer is called `Gsd-Task` everywhere else.
- **Reading the trailer with a second `git` call, or through gix.** gsd-git keeps one declared argv; a later
  `@git/core` plan reads a typed `trailers` answer.
- **Skipping a `none` digest in freshness** (the base): it could not tell "nothing linked" from "nothing to read".

## Consequences

- A report that links no test to a truth stales none of its verdicts; the twin's verdicts stay fresh after an
  unrelated collect.
- `specforge collect --yes` in a gsd repository records its commits beside node-test's facts.
- The executor prompt does not write the trailer yet: `gsd/core/assets/**` is S1's, which adds it to the commit step.
- A reftable repository (`git init --ref-format=reftable`) is not detected; it names the runner.

## What would reopen it

- A commit linked to several tasks: `linked_by` and `task` are single values.
- A test input that must tell a skipped run from a missing one beyond `unjudged`.
