# `@gsd/core`'s links come from GSD's fields and collected data

**Status:** accepted (2026-11-17). **Supersedes**
[ADR 0351](0351-a-plan-lists-its-decisions-and-the-threads-gaps-have-codes.md) D1 (`plan.decisions`), D2 (the four
rules read the edges; the matcher deleted), D3 (E312 excused by a listed decision), D4 (the plan check's three decision
inputs), D5 (`gsd/E519`) and D10 (`plan.decisions` names its reverse); supersedes
[ADR 0331](0331-the-gsd-requirement-thread.md) D1 (`truth.requirement`) and D2 (`truth.proof`), except D2's owner
state (a `plan` is complete on `complete`, a `task` on `done`), which stays because the planned paths of
[ADR 0350](0350-a-fields-checks-may-wait-for-its-owner.md) read it, and `@code/core` stays an optional peer.
**Amends** ADR 0351 D8 (two settings, not three) and D9 (the assets keep the `Gsd-Task:` trailer and stop writing
`decisions`); ADR 0331 D5, D7 and D8 and its 2026-11-10 amendment (the sources of an entity's tests, the truth's
`tests` input, the route); [ADR 0312](0312-collect-re-keys-a-test-id-as-the-test-report-does.md) as ADR 0331 amended
it (a truth's tests are the facts that name it);
[ADR 0352](0352-gsd-git-reads-the-host-git-port-and-commits-name-their-items.md) D3 (the `item` step goes on from the
commit) and D7 (W522's anchors and reach); and
[ADR 0370](0370-gsd-core-holds-exactly-gsd-cores-kinds-and-fields.md)'s rows for ADR 0359 D8 and D9 (what
`satisfied_by` and `proof` read). No protocol, SDK or kernel change comes with this decision, and no builtin is
re-vendored. `@gsd/core` loses one code (70 to 69) and one setting.

## Context

**The rule.** `@gsd/core` holds exactly gsd-core's kinds and fields (ADR 0370). For links the owner set the rule on
2026-11-16: links are collected, not authored. A link is a field gsd-core writes, a collected field or fact edge, or
something a pass derives. Four authored link fields break it, each added by a SpecForge ADR for a reader of its own:

- **`plan.decisions`** (ADR 0351 D1): the context decisions a plan implements. `gsd/E310`, the decision rows of
  `gsd/W317`, `gsd/W506` and `gsd/E312`'s exemption read it, and so do the plan check's `decisions` input
  (`gsd/core/src/judged.rs`, `PLAN_CHECK_INPUTS`) and the executor's `decisions` context section. It is read back as
  `implemented_by` (ADR 0351 D10).
- **`truth.proof`** (ADR 0331 D2): the tests that prove a truth. The requirement's computed `proof`, `gsd/E515`,
  `gsd/E502`, E501's truth input `tests` and the route `gsd/requirement-thread` read it.
- **`truth.requirement`** (ADR 0331 D1): the requirement a truth proves, read back as `requirement.truths`. The same
  readers as `truth.proof`, and `gsd/E519` (ADR 0351 D5, under the setting `specforge.truth_requirements`).
- **`task.items`** (ADR 0352): the code items a task names. `requirement.satisfied_by`, `gsd/W522` and the route's
  `item` step read it.

gsd-core writes none of them. What it writes instead, and what this ADR reads:

- a plan's `requirements` (`agents/gsd-planner.md` line 408);
- a summary's coverage item: a `deliverable` with its `requirement` and its `checks`, each check a `kind`, a `ref` and
  a `status` (`gsd-core/templates/summary.md` lines 53-66);
- the `Gsd-Task:` trailer the executor writes on a task commit (ADR 0351 D9, kept);
- decision ids cited in a plan's text, which upstream's decision-coverage gates match
  (`src/decision-coverage-support.cts`, `src/gate-decision-coverage-plan.cts`, `src/gate-decision-coverage-verify.cts`
  and `src/gap-checker.cts`).

Upstream facts are cited by their path under `gsd/parity/upstream/v1.19.0/` and their line. Those four sources are
vendored there, with their sha256 in `SOURCE.md`.

## D1. The four fields leave

`truth.requirement`, `truth.proof`, `task.items` and `plan.decisions` leave `@gsd/core`, with `gsd/E519`, its setting
`specforge.truth_requirements`, and the reverse names `implemented_by` and `requirement.truths`. (`task.mitigates`
already left with the risk under ADR 0370.)

## D2. A decision is cited by its id

A plan cites a decision when the decision's name (`D-01`) is a whole word of the plan's text: JavaScript's `\b`, where
a word character is an ASCII letter, digit or `_`, and the case as written. This is how upstream's `decisionMentioned`
(`src/decision-coverage-support.cts` lines 71-76) and its gap checker (`src/gap-checker.cts` lines 176-185) read it.

The word is read with the path written before it (`01/D-01`, `v1.0/01/D-01`, a rooted `/v1.0/01/D-01`) and resolved
from the plan by its scope chain ([ADR 0284](0284-a-name-is-unique-within-its-scope.md) D4): a bare `D-01` is the
plan's own phase's decision, `01/D-01` phase 01's.

The plan's text is its `objective`, `purpose` and `output`, and every field of its tasks, checkpoints, truths,
artifacts, key links and prohibitions: upstream's designated sections, which are `<objective>`, the tasks and the
must-haves (`src/decision-coverage-support.cts` lines 159-203). It is not the plan's context references, its
verification or its success criteria.

Three questions the planning left to this ADR, settled with their evidence:

- **A path that resolves to no decision** (`src/D-01`) cites nothing, and no code reports it: an uncited decision is
  still `gsd/E310`, `gsd/W317` or `gsd/W506` as any other, and the judged plan check (`gsd/W311`) may name it.
  Evidence: `passes::plans::written_target` and ADR 0284 D4 resolve a name by its holder's chain, never by a suffix,
  and an unresolved item names nothing; the rule `gsd/I321` already reads a written name so.
- **E312's exempting citation** is anywhere in the text of the task's plan: the same set E310 reads, so "cited" has one
  meaning. The exemption still needs the cited decision's own `text` to hold the phrase. Evidence: upstream's checker
  cross-references a hit with the context decision the plan claims to implement (its scope-reduction dimension,
  `agents/gsd-plan-checker.md` lines 430-446, the lines `gsd/core/src/passes/scope.rs`'s module doc cites), a claim
  made by the plan, not by the task.
- **The planner's fields-table row for `decisions`** is deleted, not replaced. Upstream's line there is the
  `autonomous` row (`agents/gsd-planner.md` line 407), which stays merged into SpecForge's computed `wave`,
  `autonomous` row (upstream line 404), so the port lock records a deletion.

## D3. E310, W317 and E312's exemption read the citations

`gsd/E310`, the decision rows of `gsd/W317` and `gsd/E312`'s exemption read D2's citations. A scope-cutting phrase in a
task action is excused only when a decision the plan cites holds that same phrase in its own text. A decision named in
the action that does not say the phrase excuses nothing: the defect ADR 0351 D3 fixed (an exemption the wrong way
round) stays fixed.

## D4. What W506 reads

`gsd/W506` reads, for a verified phase, the text of its complete plans (D2's surfaces), the text of their summaries
and of the coverage items and deviations those summaries scope, and the subjects of the recorded commits of those
plans; each citation is read from the plan by D2. It reads no file contents and no commit body.

## D5. A requirement's proof reads three test sources

A requirement's computed `proof` keeps its six states (`no_task`, `no_commit`, `not_run`, `no_test`, `failed`,
`proven`) and reads three test sources:

- a test fact that names a committed task of its plans;
- a test fact whose `names` holds the requirement (a `[gsd:<id>]` tag);
- the test fact that a check of a deliverable names (`checks.ref`, read as `[file#]name` or as a fact id by
  `passes::thread::names_fact`), when the deliverable's `requirement` is this requirement.

A requirement that no plan lists (a phase listing it does not count) reads its tests alone: `not_run`, `no_test`,
`failed` or `proven`. One that a plan lists keeps the full chain: task, commit, test.

`gsd/E515` reads `proof` unchanged; its remedy names the coverage item and a tag. `gsd/E502` reads the facts that name
the truth and the verdict's `cited_tests`. E501's truth input `tests` is `test(self<test.names)`.

## D6. `satisfied_by` and W522 read commits

`requirement.satisfied_by` is the `items_changed` of the recorded commits whose task is a task of a plan that lists the
requirement. `gsd/W522` anchors an item by those commits alone, a plan reaches a requirement by its `requirements`
alone, and the warning sits on the first anchoring commit.

## D7. The route

The route `gsd/requirement-thread` goes from a requirement to:

- its plans (`<plan.requirements`), their tasks (`<task.plan`), each task's commits (`<commit.task`, annotated with
  `linked_by`) and each commit's items (`.items_changed`);
- each plan's truths (`<truth.plan`, annotated with the E501 judgement);
- the deliverables whose `requirement` is it (`<deliverable.requirement` from the start, annotated with each check's
  `ref`).

The start is annotated with its computed `proof` and with the `status` of the tests that name it (`of <test.names`).

This is the owner's answer of 2026-11-17 to the question the planning could not settle by reading: the route prints the
deliverable and each check's `ref`, and the requirement's computed `proof`, which reads the coverage check, carries the
status on the start line. No kernel change is made in this milestone. Two reasons shape it:

- **The tagged tests are an annotation, not a step.** A route step is required (ADR 0352 D3,
  `crates/app/ops/src/read/trace/route.rs`'s `kept`): a `test` step through `<test.names` from the start would leave
  the thread of every requirement no test names incomplete, and no test of the taskboard twin names `TASK-01`. An
  annotation is not required, so `of "<test.names"` on the start reaches the tagged tests without making the thread
  incomplete.
- **A check's own computed status is not printed.** A check's status is a computed member (`checks[0].status`,
  written by `check_statuses` in `gsd/core/src/passes/derive/execute.rs`). A route annotation reads the written value
  first (`field_values` in `crates/app/ops/src/read/trace/route.rs`: the resolved value, and the computed one only when
  the field is unwritten). A written `checks` block holds `kind` and `ref` and no `status`, so `checks.status` prints
  nothing. Printing it needs a kernel change, which is recorded for a later kernel round.

## D8. The judged inputs and the context packs

The plan check's `decisions` input goes. `phase_decisions` and `all_decisions` stay, so the checker still sees every
decision, and the plan check reads 11 inputs. The executor's `decisions` context section goes: the phase's context
decisions are in its `context` section, which is enriching, so a small budget may cut them, and that is accepted. The
`gsd/plan-checker` context follows the judged table. A recorded plan check goes stale once (`gsd/W318`), a recorded
executor pack once (`gsd/W524`).

## D9. The assets

The assets ADR 0351 D9 rewrote carry upstream's text again (the planner, the plan checker and `/gsd-plan-phase`),
except for two things: the `Gsd-Task:` trailer, which stays, and one line of SpecForge's own in the planner: a decision
of another phase is written with its path (`01/D-01`), and a bare id is always this phase's.

## D10. The A/B corpus and the ledger fixture

This is the owner's answer of 2026-11-17 to the second question the planning could not settle by reading. The A/B
corpus (`evaluation/gsd-ab/`) seeded three of the thread's defects through `truth.proof`:

- **S5 and S6 are re-pointed** to a broken coverage check that `gsd/W408` reports, on each fixture where that signal
  is seed-only: the seeded copy reports it, and the unseeded copy and the copy with the file edit alone do not.
- **S1 is retired.** No gsd entity holds an authored reference to a test any more, and the coverage item that could
  name a test absent from the run is written by the run itself (the execute task's summary), so there is nothing to
  seed before the run.
- **H2 and arm B0 stay**, carried by S5 and S6. `gsd/W408` reads the check's status, which the derive pass computes
  from test facts (`check_statuses`); arm B0 has no test facts, so it computes no status and reports neither S5 nor S6,
  while arm B reports both. S5 and S6 still separate B from B0, which is what H2 measures. H2 is re-registered over
  the thread's defects that remain (S5 and S6, no longer S1): a change of the registered hypothesis, which the
  protocol's version bump to `gsd-ab-v3` records.

Where a fixture gives S5 or S6 no seed-only signal (the vault twin has no coverage item; the ledger's S6 if its only
deliverable is already `gsd/W408` on the red run), that task no longer seeds it, and the task records why; the defects
S5 and S6 themselves stay.

**Why not a test tag.** A requirement tag can carry a seed-only signal: `[gsd:TASK-01]` on a fixture test makes the
requirement's `proof` `failed` on the seeded copy only. But tagging renames the fixture's recorded tests, which changes
the recorded test names and the gsd-tools goldens recorded over the fixture, and re-recording those needs a gsd-core
1.19.0 build this machine does not have. GSD's own coverage item already names the tests (`checks.ref`), so a broken
check carries S5 and S6 with no fixture edit.

**The ledger fixture.** Its red run shows as the failing check of its coverage item (`gsd/W408` on
`01-02-SUMMARY/D1`). Whether the red run still stales the truth `01-02/TR2` depends on whether the kernel digests a
verdict's cited tests from the run, which was read, not run, at planning: the phase observes it and writes the observed
answer into this decision before it closes. The ledger fixture records no commit, so `LEDG-02`'s proof is `no_commit`
either way.

## What each decision of ADRs 0351 and 0331 becomes

"Superseded" means the decision no longer holds. "Kept" means it holds as written. "Amended" means it holds with the
change named.

| ADR | Decision | Outcome | Why |
|---|---|---|---|
| 0351 | D1 | superseded | `plan.decisions` leaves (D1); a plan cites a decision by its id in its text (D2) |
| 0351 | D2 | superseded | E310, W317 and W506 read D2's citations, not an edge (D3, D4); the id match is back, the soft phrase is not |
| 0351 | D3 | superseded | E312's exemption reads a decision the plan cites by id, and still needs the decision's own text to hold the phrase (D3) |
| 0351 | D4 | superseded | the plan check's `decisions` input goes; `phase_decisions` and `all_decisions` stay (D8) |
| 0351 | D5 | superseded | `gsd/E519` and `specforge.truth_requirements` leave with `truth.requirement` (D1) |
| 0351 | D6 | kept | `gsd/E521` reads the trailer, which gsd-core's executor writes |
| 0351 | D7 | kept | `gsd/W520` reads a plan's paths and its commits' files, none of the four fields |
| 0351 | D8 | amended | two settings, `specforge.task_trailers` and `specforge.plan_paths_touched`; `specforge.truth_requirements` leaves (D1) |
| 0351 | D9 | amended | the assets keep the `Gsd-Task:` trailer and stop writing `decisions` (D9) |
| 0351 | D10 | superseded | `plan.decisions` and its reverse `implemented_by` leave (D1) |
| 0331 | D1 | superseded | `truth.requirement` and its reverse `requirement.truths` leave (D1) |
| 0331 | D2 | superseded except its owner state | `truth.proof` leaves (D1); the owner state stays, read by ADR 0350's planned paths, and `@code/core` stays an optional peer |
| 0331 | D3 | kept | gsd-git reads the `Gsd-Task:` trailer first, as ADR 0352 D3 left it |
| 0331 | D4 | kept | gsd-git's detection, as ADR 0352 D4 and the 2026-11-10 amendment left it |
| 0331 | D5 | amended | an entity's tests are the facts that name it, the facts a deliverable check's `ref` names and a verdict's cited tests; no proof (D5) |
| 0331 | D6 | kept | the kernel reads a fact's result through its kind's `result` field |
| 0331 | D7 | amended | `@gsd/core`'s truth input is `test(self<test.names)` (D5) |
| 0331 | D8 | amended | the route reaches truths through the plans and tests through the coverage item and the tags (D7) |

ADR 0351: 6 superseded, 2 kept, 2 amended. ADR 0331: 1 superseded, 1 superseded except its owner state, 3 kept, 3
amended.

## The reasons ADRs 0351 and 0331 gave, answered

| Reason | Where it was given | What answers it now |
|---|---|---|
| "A name match cannot tell phases apart" | ADR 0351, Context | the citation is read with its path and resolved by scope (D2). What remains is upstream's own behaviour, accepted: a bare id written for another phase covers this phase's decision of the same name, so the planner writes another phase's decision by its path (D9) |
| "A paraphrase of six words is no citation" | ADR 0351, Context and Rejected | kept, as a named divergence (below) |
| "A name in prose is no claim" | ADR 0351, Context | accepted as upstream's reading; the judged plan check (`gsd/W311`) still reads every decision of the phase and judges whether the tasks deliver it, and E312's exemption needs the decision's own text (D3) |
| E312's exemption was "the wrong way round" | ADR 0351, Context and D3 | stays fixed (D3) |
| "The route and E515 join on it" (`truth.requirement`) | ADR 0331 D1 | the route reaches truths through the plans and tests through the coverage item and the tags (D7); E515 reads `proof` (D5) |
| A `[TR1]` tag cannot name a truth: "three truths are named TR1" (W115) | ADR 0331, Rejected | a tag names the truth by its path (`[gsd:01-01/TR1]`), resolved as any fact reference ([ADR 0346](0346-a-fact-says-where-it-is-written-and-what-it-names.md) D4) |

## Divergences from upstream, named

The parity acceptance list names each with its tag: `[decision-citation-id-only]` and `[decision-shipped-reach]`.

- **`[decision-citation-id-only]`.** gsd-core 1.19.0 also counts a decision cited when the first six normalised words
  of its text appear in the plan's normalised text (`decisionMentioned`'s soft phrase,
  `src/decision-coverage-support.cts` lines 54-76). SpecForge counts the id alone. Also: SpecForge reads the path
  before the id (upstream matches the id inside any path), a path that names no decision cites nothing, and an id
  inside a fenced block or an HTML comment of a field counts (upstream's plan gate strips them first,
  `extractPlanDesignatedSections`). No vendored verdict depends on these: every fixture decision's text is under six
  words.
- **`[decision-shipped-reach]`.** Upstream's verify gate also reads the contents of the files the summaries list and the
  bodies of the phase's commits (`src/gate-decision-coverage-verify.cts` lines 120-143). `gsd/W506` reads neither (D4),
  and its `verify-final` verdict stays unreproduced for that reason.

## Rejected

- **Keeping `truth.proof`.** It is an authored link GSD does not write.
- **The soft phrase.** A paraphrase is no citation.
- **Reading shipped files and commit bodies in W506.** `@gsd/core` reads no file content for a rule, and the commit
  body is not recorded.
- **A `test` step in the route.** A required step makes every untagged requirement's thread incomplete.
- **A harness field on the deliverable that a collector writes from `checks.ref`.** It is a field GSD does not write.
- **Tagging the twin's tests, for the route or for the A/B seeds.** It would work, but it renames recorded tests and
  changes the gsd-tools goldens recorded over the fixture, which need a gsd-core 1.19.0 build to re-record.
- **Excluding a human-judged coverage item's checks.** Its checks' status is still computed from collected facts.

## Consequences

Each holds once the phase that this ADR opens lands.

- `@gsd/core` holds no authored reference to a test, so `code/E101`, `code/W102` and `code/W401` no longer fire on a
  gsd entity.
- A failing test stales a truth's verdict only through a test that names the truth or through the verdict's cited
  tests.
- `satisfied_by` is absent until a commit is collected with item facts; W522's warning moves from a task to a commit.
- The taskboard twin's thread ends at its coverage item.
- `@gsd/core` declares 69 codes, and the settings and preset tables are one row shorter.
- Breaking: a project that writes one of the four fields or sets `specforge.truth_requirements` no longer checks clean.

## Would reopen it

- gsd-core writing a truth's requirement or test.
- A kernel round that lets a route print a computed block member: D7 would then print the check's status.
- Upstream dropping the soft phrase.
