# A plan is placed by what it shares, threaded before its check, and measured

**Status:** accepted (2026-11-19). Builds on [ADR 0370](0370-gsd-core-holds-exactly-gsd-cores-kinds-and-fields.md)
D1 (gsd-core's `wave` stays its executed wave; SpecForge adds computed values and guards),
[ADR 0204](0204-which-edges-an-entity-depends-on-is-one-graph-answer.md) (which edges an entity depends on),
[ADR 0076](0076-the-plan-check-reads-the-snapshot-and-takes-its-spec-type.md) and the kernel's `trace --plan`,
[ADR 0299](0299-the-graph-a-call-receives-carries-what-every-surface-shows.md) (the graph a command receives carries
computed values), [ADR 0313](0313-a-rendered-step-names-its-agent.md) (a rendered step names what runs it),
[ADR 0334](0334-engine-tools-are-one-catalogue-below-the-surfaces.md) (engine tools and the grants that name them),
[ADR 0361](0361-a-kind-field-may-be-a-number-an-exact-decimal-written-as-one-word.md) (a number keeps at most 15
significant digits) and [ADR 0373](0373-gsd-core-is-vendored-on-a-pinned-rustc-with-its-load-and-size-measured.md) D3
(the blob budget). It amends and extends no ADR. No protocol change and no kernel change: `trace --plan` already ships
in the kernel and on MCP, and every decision below is a computed value, a check pass, a command or a prompt step of
`@gsd/core`.

## Context

- **Two plans of one wave may modify the same file.** gsd-core places a plan by its `depends_on` edges alone.
  `gsd/W305` says when two plans of one wave share a file, and execute-phase then runs that wave sequentially, but
  nothing says where each plan really runs, and a file a task names outside its plan's lists is never compared.
- **Nothing checked a phase's order against the graph before the plan checker.** The kernel's `trace --plan` reports
  an entry that names no entity and an entry ordered before what it depends on, but plan-phase never ran it: the
  checker judged an order nobody had threaded.
- **No figure said which chain of plans bounds a phase.** The levels say what may run together, not which chain of
  plans, task by task, decides how long the phase takes.
- **No figure said how far an estimate was from what a plan took.** A plan's `estimate` and its summary's `actuals`
  sit side by side, and nothing compared them.

## D1. A plan's strict wave places it after the plans of its phase it shares a path with

The computed `plan.strict_wave` is the longest-path level over the phase's `depends_on` edges plus an implied edge from
the earlier plan to the later one for every pair of plans of the phase that share a path. The earlier plan is the one
of smaller declared wave, then of smaller id. A plan's paths are its `files_modified`, its `files_deleted` and the
`files` of each of its tasks: a task's file outside its plan's lists is only a warning under gsd-compat (`gsd/W306`),
so the plan's lists alone would miss it. Paths compare as written, and a path holding a comma is one path. The strict
wave has the same offset as `wave` (0 when a plan of the phase is `wave0`, else 1).

One level function serves both fields: `wave` is that function over the `depends_on` edges, `strict_wave` the same
function over those edges and the implied ones. Every implied edge goes from a smaller (wave, id) to a larger one, so
it never closes a cycle the `depends_on` edges do not already hold. A plan of a later declared wave that shares a path
with an earlier plan is pushed only when that earlier plan is itself pushed into the later plan's wave or past it:
otherwise the two already run in different waves, and they would share a strict wave only through that push. The
field is absent where `wave` is: on a phase whose `depends_on` edges hold a cycle, and on a quick plan. `wave` itself
is unchanged.

Tasks of one plan run in order, by one executor, so the wave is the unit two tasks must not share: two tasks that
touch the same path never share a strict wave. The coupling exemption (`coupling_justified`) covers only non-file
coupling and exempts no shared path.

The reason: `wave` is gsd-core's own answer and its executor runs it; where a plan really runs is a computed value
beside it, read from the paths the plans and their tasks name.

## D2. A pushed plan says where it runs

`gsd/W526` "Wave collision" is reported once per plan whose strict wave is above its wave, a warning under gsd-compat
and gsd-strict. It says where the plan runs; the defect, two plans of one declared wave sharing a file, is `gsd/W305`'s,
which stays as it is, co-fires on a same-wave collision and is an error under gsd-strict.

The plan that binds a push is the one it waits for with the highest strict wave, ties to the smaller id. A direct push
(the binding plan shares a path with it and is not one of its `depends_on`) names the shared paths, the earlier plan,
the tasks that name those paths and both strict waves. It is reported across declared waves too when the earlier plan
is itself pushed; there `gsd/W526` fires alone, since the declared waves differ and `gsd/W305` has nothing to say. A
transitive push (the binding plan is one it depends on) names the plan it waits for and that plan's strict wave.

The reason: a planner who reads that a plan runs later, and behind what, can decide whether to declare the dependency,
keep the path in one plan, or accept the order.

## D3. plan-phase threads its plans through `trace --plan` before the plan checker

The read-only command `thread_plan <phase>` answers an agent plan, `{plan_id, entries}`: each plan of the phase in
strict-wave then id order, each followed by its tasks in id order. The `plan_phase` step `thread`, between `plan` and
`check`, runs it and feeds the answer to `specforge trace --plan`. A result with `ok: false` (an entry that names no
entity, `E003`, or an ordering gap) is fixed before the check, and the result is passed to the plan checker.
gsd-planner holds the command in its grants, and the checker's role reads it. The ported upstream workflow text is not
edited. `plan_review_convergence` re-runs `plan_phase`, so it threads too; `ultraplan_phase` only checks.

The reason: a step's command is the extension's own, and the order of a phase is a graph answer the kernel already
checks; the extension only has to state the order it means to run.

## D4. A phase's critical path is its heaviest chain of plans, named task by task

The computed `phase.critical_path` (the task ids, in execution order) and `phase.critical_weight` name the heaviest
chain of plans along the strict placement's edges. A plan weighs its `estimate.tokens` when it has one (a negative
value counts 0), else its task count. Of two chains of equal weight the longer one (more plans) wins, then the one of
smaller plan id, so a plan of weight 0 after a chain (an estimate of 0 tokens, a plan with no task yet) stays on it.
The tasks of a plan come in id order. Both fields are absent when no plan of the phase has a level, and the path is
absent when the chain names no task.

The reason: the chain that bounds a phase is the one to shorten, and it moves with a dependency or a shared path
because it reads the strict placement.

## D5. A plan's estimate error is its estimate against what it took

The plan's `estimate` gains `tasks`, which gsd-core writes. The computed `plan.estimate_error {tasks}` is
`(actual - estimate) / estimate` for a plan whose summary is complete, present only when the estimate writes `tasks`
above 0, rounded to 4 decimal places. The actual is the plan's task count, as the summary's `actuals.tasks` counts it.
`progress-report` adds a line per measured plan.

The token estimate is not measured. gsd-core pairs `estimate.tokens` with the summary's `actuals.tokens`, which its
executor writes as chars/4 over the files the plan changed. Here a summary's `actuals` is computed, so nobody may write
a member of it (E082), and the harness ledger vouches for a field, never for a member of a block: no command could
record the tokens a plan took and have the host tell them from typed ones. A `tokens` member that could never be
present would promise a measurement the model cannot make.

The reason: an estimate is only worth recording if what it missed by is read back; absent, never zero, where nothing
was measured.

## Rejected

- **Moving gsd-core's `wave` by overlap.** ADR 0370 D1: `wave` is gsd-core's field and its executor's answer.
- **A `Do::Command("trace")` step.** A step's command is the extension's own; `trace` is a kernel tool.
- **Editing the ported plan-phase workflow.** Its port hashes, install copies and upstream parity would all move for a
  step the extension can add beside it.
- **A coupling exemption for a shared path.** `coupling_justified` covers non-file coupling; a shared path is the
  collision itself.
- **A second level computation for the strict wave.** Two level loops could disagree about the same edges.

## Limits

- The order of tasks inside a plan is not checked: tasks carry no `depends_on`.
- The order `thread_plan` answers satisfies `depends_on` and `task.plan` by construction, so a gap comes from a
  coupling peer that runs later or from a reordered list.
- `missing_plan_entry` gaps count every testable entity of the project, and are warnings.
- The token estimate is not measured: a summary holds no `actuals.tokens` (D5).
- Plans with and without an estimate weigh in different units on one critical path.

## Consequences

- Two plans of one phase that share a path, or whose tasks do, never share a strict wave.
- A pushed plan says where it runs and behind which plan.
- plan-phase hands the checker a phase order that `trace --plan` has already read.
- A phase names the chain of tasks that bounds it.
- A completed plan says how far its task estimate was from the tasks it took.
- The `@gsd/core` blob grows, within the budget of ADR 0373 D3 or by raising it there.

## Would reopen it

- A `depends_on` on tasks, so task order inside a plan can be checked.
- A harness ledger that vouches for a member of a block, so a command can record the tokens a plan took beside the
  computed `actuals.tasks` and `actuals.commits`.
- An executor that runs gsd-core's waves by the strict wave.
