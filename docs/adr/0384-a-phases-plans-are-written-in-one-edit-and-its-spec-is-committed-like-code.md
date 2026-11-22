# A phase's plans are written in one edit, and its spec is committed like code

**Status:** accepted (2026-11-21). Builds on [ADR 0343](0343-a-gsd-block-calls-only-specforge.md) D10 (a write is a
typed edit or a `specforge.edit`), [ADR 0167](0167-a-typed-edit-is-compared-with-the-checked-report.md) and
[ADR 0297](0297-a-judged-rules-finding-is-not-an-error-a-write-adds.md) (what E080 counts as an added error, and that
a judged rule's finding is not one),
[ADR 0363](0363-a-codes-findings-may-wait-for-its-owner-state-and-a-write-that-reveals-one-adds-no-error.md) D3 (an
entity a write creates reveals nothing) and the Not applicable table of
[`gsd/core/spec/settings.md`](../../gsd/core/spec/settings.md#not-applicable). No protocol change, no kernel change, no
new code and no new setting. D4 amended and the Limits answered in the kernel by
[ADR 0385](0385-a-code-may-be-settled-later-and-a-write-that-creates-its-subject-adds-no-error-of-it.md): a code may be settled later; `@gsd/core`'s side is recorded
with the change that declares its codes so.

## Context

A live run of the plan-phase prompt under gsd-strict, on a phase of this repository's own spec, met four things:

- **The planner was told a route the kernel refuses.** Its body said to write plan by plan with typed creations and to
  run a refused line again. The first plan of a phase whose requirements span plans is refused `gsd/E301` in every
  profile (a requirement of the phase is in no plan yet), and under gsd-strict `gsd/W309` too (the plan has no truth
  and no artifact yet). The agent found the one-edit route itself, after refused dry runs.
- **Steps branch on a key nobody can set.** The plan commit step and a dozen other steps read or branch on
  `commit_docs`, which `@gsd/core` does not declare: the spec is source and is committed like code. The agent's own
  summary contradicted the commit it made.
- **An open question cannot be recorded under gsd-strict.** An open question no decision resolves is `gsd/W204`,
  raised to an error by gsd-strict, so E080 refuses the write that records it. The researcher dropped its two
  questions.
- **The run's phase label was the spec's.** The "Phase 12" the agents wrote is the phase entity's own title in the
  spec, not a product defect.

## D1. A phase's plans are written in one `specforge.edit`

The planner writes every plan of the phase, with its tasks, truths, artifacts and key links (and its checkpoints,
prohibitions and user setups when a plan has them), in one MCP `specforge.edit` call of create ops, one file per plan
beside the phase's file, as the roadmap edit writes a milestone's phases. On a refusal nothing is written, and the
planner fixes the list and sends the whole call again. A field the schema marks computed is never written. A
dependency found after the plans exist is linked.

The other writers of a phase's plans (chunked single-plan mode, gap closure) use the same edit for the plans they
write. A quick plan, minted before its planner runs, is filled by typed edits: adding to an existing plan is not the
refused route.

The reason: coverage (`gsd/E301`) binds from a phase's first plan and, under gsd-strict, a plan's must-haves
(`gsd/W309`) from its first write, so the entities those rules judge together must arrive together.

## D2. The plan-phase steps commit the spec

`commit_docs` is not a setting of `@gsd/core` (the Not applicable table): no step reads it, branches on it or asks for
it. Each step that committed planning documents commits the spec files it changed, unconditionally, and a
`specforge git commit` that finds nothing staged (`git_refused`, "nothing to commit") means an earlier commit of the
same run already holds them. A line naming `commit_docs`, or another key of the Not applicable table
(`search_gitignored`, `pr_strict`, `planning.sub_repos`, `graphify.*` and the other rows but the secrets one), says it
is not a setting here.

The reason: the spec is source; a condition on an undeclared key is read differently by each agent.

## D3. An open question gsd-strict refuses is returned, and the user answers it

The researcher records every other item. A question the profile refuses with `gsd/W204` (no decision resolves it yet)
is not forced through with an invented decision and not dropped: it is returned under the research's Open Questions
with what is known, what is unclear and the recommendation. The workflow that ran the research (plan-phase, quick,
quick-batch) shows it and hands it to the planner, which records no decision for it and does not record it: it writes
plans that hold whatever the answer (a task that cannot follows a `decision` checkpoint) and lists the question in its
return. After the planner, the workflow asks the user and records the answer as a decision, then the question
resolved by it, the one order gsd-strict accepts. A question the user leaves open, and every one under `--auto`, is
shown as open and stays out of the graph.

The reason: the decision that resolves a question is the user's answer. Neither agent may invent one, and the run lost
two questions the record should have kept.

## D4. What does not change

E080 and what it counts; `gsd/W204`, `gsd/W309` and `gsd/E301` and their severity in each profile; the judged rules'
exemption; the gates of the plan prompts; the planner's git commit step. No CLI batch command and no new setting.

The reason: every refusal the run met was right about the project it would have left; the texts were wrong.

## Rejected

- **A CLI batch command.** A CLI and protocol change; MCP already has the atomic edit.
- **Keeping the per-plan route beside the edit.** It is refused in every profile for a phase's first plan.
- **Declaring `commit_docs` as a setting.** It would invent a mode in which the spec is not committed, which the Not
  applicable table rules out.
- **Keeping the plan commit step's condition with a default.** An agent still reads a key nobody can set.
- **The researcher inventing a decision that resolves its own question.** A fake answer.
- **The planner answering the question with a decision of its own.** The same invented answer, one step later.
- **Asking the user at birth, before the planner.** The planner knows which plans the answer bears on; after it, the
  user answers with the plans in front of them, and a question no plan depends on does not stop the run.
- **Raising `gsd/W204` only at gates.** A new gate-time severity or setting, and `check` would pass what the gate
  refuses.
- **Dropping gsd-strict's raise of `gsd/W204`.** gsd-strict would stop blocking an unresolved question.

## Limits

Three refusals no `@gsd/core` text can clear, each of the same shape: a rule a later step of the same workflow
settles, counted by E080 on the write that creates its entity.

- An open question at research time under gsd-strict (`gsd/W204`): returned, per D3, and recorded only with the
  user's answer. Under `--auto`, or left open, it stays out of the graph.
- Chunked mode's first plan of a phase whose requirements span plans (`gsd/E301` in every profile): the planner
  returns `## PLAN BLOCKED`, which chunked mode reads as blocked, not stalled, and it then offers to plan the rest of
  the phase in one run, an edit of the plans the graph does not hold yet.
- Quick's plan mint under gsd-strict (`gsd/W309`): the mint writes the plan before its must-haves exist, so under
  gsd-strict quick's and quick-batch's research and answer steps run only on a quick plan minted before the profile
  was set.

The fix they share is a kernel one, outside this decision: E080 must not count, on a write, a finding a later step of
the same workflow settles. It is an owner question.

## What each decision becomes

"Kept" means the decision holds as written; "applied" means it holds and now reaches text it did not.

| ADR or source | Decision | Outcome | Why |
|---|---|---|---|
| 0343 | D10 (a write is a typed edit or a `specforge.edit`) | kept, applied | a phase's plans are one edit (D1) |
| `gsd/core/spec/settings.md` | Not applicable (every row but the secrets one) | kept, applied | every asset says so (D2) |
| 0363 | D3 (an entity a write creates reveals nothing) | kept | the Limits |
| 0297 | Decision (a judged rule's finding is no added error) | kept | the plan edit adds only judged findings (D4) |

ADR 0343: 1 kept. ADR 0363: 1 kept. ADR 0297: 1 kept. `gsd/core/spec/settings.md`: 1 kept.

## Tests that hold it

Each test fails when the decision or the limit it holds stops being true. They are in `gsd/core/tests`.

| Decision or limit | Tests |
|---|---|
| D1 | `assets_shell::planner_edit::the_planner_writes_a_phase_in_one_edit`, `assets_shell::planner_edit::a_dependency_found_later_is_linked`, `assets_shell::planner_edit::a_plan_written_alone_is_refused_and_nothing_is_written`, `assets_shell::planner_edit::the_plan_format_is_one_edit_of_the_whole_phase`, `assets_shell::planner_edit::every_writer_of_a_phases_plans_names_the_plan_edit` |
| D2 | `assets_shell::commits::the_plan_commit_step_commits_the_spec_and_an_empty_one_says_nothing_to_commit`, `settings_roster::the_plan_phase_family_reads_no_commit_docs`, `settings_roster::every_key_an_asset_calls_a_setting_is_a_key_of_the_roster`, `settings_roster::no_asset_names_a_not_applicable_key`, `settings_roster::the_settings_workflow_counts_the_settings_of_its_section_layout`, `settings_roster::the_advanced_settings_workflow_reads_and_confirms_only_settings`, `settings_roster::no_asset_makes_the_spec_commit_conditional_or_skips_an_ignored_spec`, `ported_text::every_setting_a_line_reads_is_a_key_of_the_roster` |
| D3 | `assets_shell::researchers::an_open_question_strict_refuses_is_returned_not_dropped`, `assets_shell::researchers::plan_phase_records_the_users_answer_then_the_question`, `assets_shell::researchers::quick_and_quick_batch_record_the_users_answer_then_the_question` |
| Limits | `assets_shell::researchers::an_open_question_strict_refuses_is_returned_not_dropped` (gsd/W204 at research under gsd-strict), `assets_shell::planner_edit::a_whole_plan_written_alone_is_refused_while_a_requirement_of_its_phase_is_in_no_plan` (chunked mode's first plan, gsd/E301), `assets_shell::planner_edit::the_rest_of_a_chunked_run_writes_only_the_plans_the_graph_lacks` (its recovery), `cmd_quick::a_quick_plan_minted_without_must_haves_is_refused_under_strict_only` (quick's mint under gsd-strict, gsd/W309) |

The live run itself, with each refusal it met and the test that pins the same rule, is recorded in
[`evaluation/gsd-live/plan-phase.md`](../../evaluation/gsd-live/plan-phase.md).

## Consequences

- The planner, its callers, the plan-phase steps, the researcher and every asset that named `commit_docs` change text.
- The `@gsd/core` blob is re-vendored: its assets changed, its declarations did not.
- The record of the live run cites the tests; "Tests that hold it" lists them by decision.

## Would reopen it

- A kernel mechanism for rules that stay open while authoring: the Limits close, and D3's return path may become a
  recorded question.
- A CLI batch edit.
- A project whose spec is not source.
