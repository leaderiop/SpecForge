# A plan lists its decisions, and the thread's gaps have codes

**Status:** accepted (2026-11-08, R2 of the Beyond Parity build plan). **Amends**
[ADR 0331](0331-the-gsd-requirement-thread.md) D1 (the strict code for a truth with no requirement exists: gsd/E519,
off in gsd-compat), D3 (the executor writes the `Gsd-Task:` trailer, and gsd/E521 says when a task commit lacks it)
and its consequence "the executor prompt does not write the trailer yet", and
[ADR 0150](0150-a-relation-is-written-on-one-side.md) D4 (a field that names its reverse is its own relation, D10).
Builds on
[ADR 0284](0284-a-name-is-unique-within-its-scope.md) (a reference resolves in its scope),
[ADR 0343](0343-a-gsd-block-calls-only-specforge.md) (a gsd block calls only `specforge`) and
[ADR 0350](0350-a-fields-checks-may-wait-for-its-owner.md) (E016 on a planned path once its owner is complete). No
protocol change, no new kernel code; `@gsd/core` gains three codes.

## Context

- A plan claims its requirements with a typed list (`plan.requirements`), but the context decisions it implements
  only by its text. Four rules read that text:
  - **E310** (a context decision no plan cites) took a decision as cited when its name (`D-01`) was a word of the
    plan's objective, purpose, output, tasks or must-haves, or when its first six normalised words were.
  - **W506** (a verified phase's decision nothing shipped) ran the same matcher over summaries and commit subjects.
  - **E312** (a task action that cuts scope) excused a hit when the action named any decision of the plan's phase.
  - **W317** (the post-planning gap table) counted an item covered when its name was a word of any plan text.
- A name match cannot tell phases apart: a phase-2 plan that cites phase 1's `D-01` covered phase 2's `D-01`, and E310
  stayed silent. A paraphrase of six words is no citation, and a name in prose is no claim.
- E312's exemption was the wrong way round. Upstream's checker cross-references each hit with its decision and
  judges whether the task delivers it in full; its own red-flag example, "D-26 cost references (v1, static labels)",
  cites the decision it cuts and is "ALWAYS BLOCKER" (`agents/gsd-plan-checker.md:437-443`, MIT, `LICENSE-GSD`).
- ADR 0331 left three gaps of the requirement thread without a code: a truth that names no requirement of its plan,
  a task commit found by its title instead of its trailer (no asset wrote the trailer, so every executor commit was),
  and a complete plan whose commits never touched a path it planned.

## Decisions

- **D1. `plan.decisions`** is a list of references to `decision`, after `requirements`: the context decisions the
  plan implements, written `decisions [D-01, D-03]` and resolved in the plan's phase (ADR 0284). It is not required:
  E310 is the one diagnostic of a missing one. A plan may list another phase's decision (`02/D-01`); that covers
  nothing in its own phase. A quick plan may list its quick context's decisions; no rule reads them.
- **D2. The four rules read the edges, and the matcher is deleted.**
  - E310: a context decision (`origin context`) of a planned phase is covered when a plan of the phase has a
    `decisions` edge to it.
  - W317: a requirement row is covered by a plan's `requirements` edge, a decision row by its `decisions` edge. The
    table, its summary and its order stay GSD's byte for byte.
  - W506: a decision of a phase whose verification passed is shipped when a complete plan of the phase lists it.
  - The text matcher (`mentions`, the six-word soft phrase, the scanned plan text, the action's citation and W317's
    word search) goes, with no shim.
- **D3. E312 is excused only by a listed decision that says the phrase.** A hit is exempt when a decision the task's
  plan lists in `decisions` has a `text` holding the same phrase, by the rule of the hit itself (a word, case folded,
  a dotted version no hit). The decision itself says "v1", so the task delivers what was decided; naming a decision
  the action cuts excuses nothing.
- **D4. The judged plan check has three decision inputs.** `decisions` is `self.decisions` (what the plan claims),
  `phase_decisions` is the phase's decisions (the old `decisions`) and `all_decisions` stays every decision: the
  checker must still see a contradiction with a decision the plan does not list. A recorded plan-check verdict goes
  stale once (W318).
- **D5. gsd/E519, a truth proves a requirement of its plan.** A truth of a phase's plan (not a quick plan) with no
  `requirement`, or with one its plan's `requirements` does not hold, is an error on the truth. Setting
  `specforge.truth_requirements`, off in gsd-compat and gsd-strict, on in gsd-plus.
- **D6. gsd/E521, a task commit is linked by its trailer.** On a recorded commit whose task was found by its subject's
  title (`linked_by title`), or that names no task while its type is one of the executor's task types other than
  `docs` (`feat`, `fix`, `test`, `refactor`, `perf`, `style`, `chore`). A `docs` commit (a plan's metadata commit) and
  a commit with no type are not judged. Setting `specforge.task_trailers`, on in gsd-plus only. A collector's plan is
  never refused for it (ADR 0303): the commit is already made.
- **D7. gsd/W520, a complete plan's commits touched its paths.** On a complete plan with at least one recorded commit
  (`files_touched` written):
  - a planned path (`files_modified` and each scoped artifact's `path`) that no commit touched; a directory path is
    touched by any file under it;
  - a `files_deleted` path the host serves as still present; a path it did not serve is not judged.

  Paths compare project-relative on both sides. W520 never says "missing": E016 does (ADR 0350). Setting
  `specforge.plan_paths_touched`, off in gsd-compat and on in gsd-strict and gsd-plus, gated as W407 is.
- **D8. The three settings** are rows of `@gsd/core`'s SpecForge-only gates, each a boolean defaulting to false that
  gates its one code, and the presets set them per profile as D5 to D7 say.
- **D9. The assets write both.** The planner lists each decision a plan implements in `decisions`, and the plan
  checker reads coverage from that list. The executor's and git-integration's task commits end with the paragraph
  `Gsd-Task: {phase}-{plan}/{task}`, written as a second `-m` (`specforge git commit` joins its messages with a blank
  line, so the trailer is the last paragraph git's trailer parser reads). Every rewritten body line is a row of
  `tests/common/thread_rewrites.rs`, and every rewritten block still calls only `specforge`.

- **D10. A field that names its reverse is its own relation** (amends ADR 0150 D4). `decision.plan` (the quick plan
  whose context recorded a decision) and `plan.decisions` link the two kinds both ways as two relations, and W162's
  kind-pair rule took them for one. A reference field with an `inverse_of` (a word its target kind does not use, else
  W162 still fires on the name) pairs with nothing: `plan.decisions` is read back as `implemented_by`. Two kinds
  linked both ways by fields that name no reverse stay W162.

## Rejected

- **The soft phrase.** Six words in common are no citation, and a paraphrase that loses a word loses coverage.
- **`decisions` on the task.** Upstream's checker cross-references the decision a plan claims to implement, and
  `requirements` is on the plan too; a task-level list doubles the planner's bookkeeping for no rule that reads it.
- **Narrowing the judged inputs to `self.decisions` alone.** It hides the phase's other settled decisions from the
  checker, which must spot a contradiction with one the plan does not list.
- **W520 always on.** GSD has no such rule, and gsd-compat is GSD as it is.
- **The trailer as a new asset line.** The port keeps upstream's line counts; the closing line of the message is the
  one place a paragraph can be added.
- **Moving `plan.decisions` into a block member** to dodge W162's kind-pair rule, as a block member is not paired:
  the relation is the plan's own list, and the rule was the imprecise part.
- **E521 on every commit not linked by its trailer.** GSD's plan-metadata commit names no task by design.

## Consequences

- A plan that cites a context decision only in prose gets E310 (and W317, and W506 once its phase is verified); its
  remedy names the `decisions` field.
- Under gsd-plus, a write that adds a truth without a requirement of its plan is refused (E080).
- Commit paths are repository-relative until gsd-git writes them project-relative; until then a project under a
  subdirectory of its repository sees every planned path untouched, and the rule's row says so.

## What would reopen it

- A decision that a plan implements only in part: `decisions` says all or nothing.
- A commit that serves several tasks: `linked_by` and `task` are single values (ADR 0331).
