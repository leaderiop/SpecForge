# A phase's plans are written in one edit, and its spec is committed like code

**Status:** accepted (2026-11-21). Builds on [ADR 0343](0343-a-gsd-block-calls-only-specforge.md) D10 (a write is a
typed edit or a `specforge.edit`), [ADR 0167](0167-a-typed-edit-is-compared-with-the-checked-report.md) and
[ADR 0297](0297-a-judged-rules-finding-is-not-an-error-a-write-adds.md) (what E080 counts as an added error, and that
a judged rule's finding is not one),
[ADR 0363](0363-a-codes-findings-may-wait-for-its-owner-state-and-a-write-that-reveals-one-adds-no-error.md) D3 (an
entity a write creates reveals nothing) and the Not applicable table of
[`gsd/core/spec/settings.md`](../../gsd/core/spec/settings.md#not-applicable). No protocol change, no kernel change, no
new code and no new setting. Amended by
[ADR 0385](0385-a-code-may-be-settled-later-and-a-write-that-creates-its-subject-adds-no-error-of-it.md) (a code may be
settled later): D1's reason reworded, D3 replaced, D4 amended and the Limits closed, in the kernel and in `@gsd/core`.

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
write. A quick plan, minted before its planner runs, is filled by typed edits, its must-haves settling the mint's
`gsd/W309`.

The reason (reworded by ADR 0385): one edit is the atomic route. A phase's coverage (`gsd/E301`, `gsd/E310`) and,
under gsd-strict, its plans' must-haves (`gsd/W309`) hold from the edit that writes them, so the phase is never
half-planned. A plan written alone is accepted too: those codes are settled later, so the write reports them, `check`
fails on them until the rest of the phase is written, and `execute_phase` refuses the phase meanwhile.

## D2. The plan-phase steps commit the spec

`commit_docs` is not a setting of `@gsd/core` (the Not applicable table): no step reads it, branches on it or asks for
it. Each step that committed planning documents commits the spec files it changed, unconditionally, and a
`specforge git commit` that finds nothing staged (`git_refused`, "nothing to commit") means an earlier commit of the
same run already holds them. A line naming `commit_docs`, or another key of the Not applicable table
(`search_gitignored`, `pr_strict`, `planning.sub_repos`, `graphify.*` and the other rows but the secrets one), says it
is not a setting here.

The reason: the spec is source; a condition on an undeclared key is read differently by each agent.

## D3. An open question is recorded open, and the user's answer resolves it

Replaced by ADR 0385. The researcher records every open question open, with what is known, what is unclear and the
recommendation, and no `resolved_by`: it invents no decision, drops none and returns none in place of recording it.
`gsd/W204` is settled later, so the write is accepted and reports it. The workflow that ran the research (plan-phase,
quick, quick-batch) shows the questions and hands them to the planner, which records no decision for them: it writes
plans that hold whatever the answer (a task that cannot follows a `decision` checkpoint) and lists the questions in
its return. After the planner, the workflow asks the user, records the answer as a decision and sets it as the
question's `resolved_by`. A question the user leaves open, and every one under `--auto`, stays in the graph: under
gsd-strict `check` fails on it, and plan-phase's entry gate (`gsd/W204`) refuses the next run on the phase until it is
answered.

The reason: the decision that resolves a question is the user's answer, and the question belongs in the record from
the moment it is found. The decision this replaces returned a question gsd-strict refused and recorded it only with
the answer, so a question left open was lost.

## D4. What does not change

Amended by ADR 0385: E080 counts no settled-later finding on what a write creates, `@gsd/core` declares `gsd/W204`,
`gsd/E301`, `gsd/E310` and `gsd/W309` settled later (`gsd/E310` now on the phase, not the decision), and
`execute_phase` gates on `gsd/E301`, `gsd/E310` and `gsd/W309`. What still holds: those codes' severity in each
profile; the judged rules' exemption; the gates of the plan prompts; the planner's git commit step. No CLI batch
command and no new setting.

The reason: every refusal the run met was right about the project it would have left; the texts were wrong, and the
three no text could clear were answered in the kernel (ADR 0385).

## Rejected

- **A CLI batch command.** A CLI and protocol change; MCP already has the atomic edit.
- **Keeping the per-plan route beside the edit.** It was refused in every profile for a phase's first plan. Since
  ADR 0385 a plan written alone is accepted, and chunked mode writes plan by plan; the edit stays the atomic route.
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

Closed by ADR 0385. There were three refusals no `@gsd/core` text could clear, each of the same shape: a rule a later
step of the same workflow settles, counted by E080 on the write that creates its entity. Each write is accepted since,
its finding reported until the later step settles it:

- An open question at research time under gsd-strict (`gsd/W204`): recorded open, and resolved by the user's answer
  later (`assets_shell::researchers::an_open_question_is_recorded_open_and_its_answer_resolves_it_later`).
- Chunked mode's first plan of a phase whose requirements span plans (`gsd/E301` in every profile, and `gsd/E310` when
  a later plan cites a decision of the phase's context): written, and the last plan settles the phase's coverage
  (`assets_shell::planner_edit::a_phases_plans_written_one_at_a_time_are_each_accepted_and_the_last_settles_coverage`,
  `assets_shell::planner_edit::a_plan_written_alone_is_written_and_reports_what_a_later_write_settles`);
  `## PLAN BLOCKED` now means a refused call only.
- Quick's plan mint under gsd-strict (`gsd/W309`): written, and the planner's must-haves settle it
  (`cmd_quick::a_quick_plan_minted_without_must_haves_is_accepted_and_its_finding_reported_under_strict`).

The fix they shared was a kernel one: E080 counts no settled-later finding on an entity the write creates or creates
something under (ADR 0385 D2). A phase left half-planned is refused by `execute_phase`
(`prompts_gates::e301_e310_w309_a_half_planned_phase_refuses_execute_phase`).

## What each decision becomes

"Kept" means the decision holds as written; "applied" means it holds and now reaches text it did not.

| ADR or source | Decision | Outcome | Why |
|---|---|---|---|
| 0343 | D10 (a write is a typed edit or a `specforge.edit`) | kept, applied | a phase's plans are one edit (D1) |
| `gsd/core/spec/settings.md` | Not applicable (every row but the secrets one) | kept, applied | every asset says so (D2) |
| 0363 | D3 (an entity a write creates reveals nothing) | kept | the Limits; ADR 0385 D3 keeps it too |
| 0297 | Decision (a judged rule's finding is no added error) | kept | the plan edit adds only judged findings (D4) |

ADR 0343: 1 kept. ADR 0363: 1 kept. ADR 0297: 1 kept. `gsd/core/spec/settings.md`: 1 kept.

## Tests that hold it

Each test fails when the decision or the limit it holds stops being true. They are in `gsd/core/tests`.

| Decision or limit | Tests |
|---|---|
| D1 | `assets_shell::planner_edit::the_planner_writes_a_phase_in_one_edit`, `assets_shell::planner_edit::a_dependency_found_later_is_linked`, `assets_shell::planner_edit::a_plan_written_alone_is_written_and_reports_what_a_later_write_settles`, `assets_shell::planner_edit::the_plan_format_is_one_edit_of_the_whole_phase`, `assets_shell::planner_edit::every_writer_of_a_phases_plans_names_the_plan_edit` |
| D2 | `assets_shell::commits::the_plan_commit_step_commits_the_spec_and_an_empty_one_says_nothing_to_commit`, `settings_roster::the_plan_phase_family_reads_no_commit_docs`, `settings_roster::every_key_an_asset_calls_a_setting_is_a_key_of_the_roster`, `settings_roster::no_asset_names_a_not_applicable_key`, `settings_roster::the_settings_workflow_counts_the_settings_of_its_section_layout`, `settings_roster::the_advanced_settings_workflow_reads_and_confirms_only_settings`, `settings_roster::no_asset_makes_the_spec_commit_conditional_or_skips_an_ignored_spec`, `ported_text::every_setting_a_line_reads_is_a_key_of_the_roster` |
| D3 | `assets_shell::researchers::an_open_question_is_recorded_open_and_its_answer_resolves_it_later`, `assets_shell::researchers::the_researcher_records_every_open_question_open`, `assets_shell::researchers::plan_phase_resolves_the_recorded_question_with_the_users_answer`, `assets_shell::researchers::quick_and_quick_batch_resolve_the_recorded_question_with_the_users_answer` |
| D4 | `declaration::the_settled_later_codes_are_the_rows_of_rules_md` (the four codes), `prompts_gates::e301_e310_w309_a_half_planned_phase_refuses_execute_phase` (the gate) |
| Limits (closed) | `assets_shell::researchers::an_open_question_is_recorded_open_and_its_answer_resolves_it_later` (gsd/W204 at research under gsd-strict), `assets_shell::planner_edit::a_phases_plans_written_one_at_a_time_are_each_accepted_and_the_last_settles_coverage` and `assets_shell::planner_edit::a_plan_written_alone_is_written_and_reports_what_a_later_write_settles` (chunked mode's first plan, gsd/E301, gsd/E310), `assets_shell::planner_edit::the_rest_of_a_chunked_run_writes_only_the_plans_the_graph_lacks` (its recovery), `cmd_quick::a_quick_plan_minted_without_must_haves_is_accepted_and_its_finding_reported_under_strict` (quick's mint under gsd-strict, gsd/W309) |

The live run itself, with each refusal it met and the test that pins the same rule, is recorded in
[`evaluation/gsd-live/plan-phase.md`](../../evaluation/gsd-live/plan-phase.md).

## Consequences

- The planner, its callers, the plan-phase steps, the researcher and every asset that named `commit_docs` change text.
- The `@gsd/core` blob is re-vendored: its assets changed, its declarations did not.
- The record of the live run cites the tests; "Tests that hold it" lists them by decision.
- ADR 0385 closes the Limits: the researcher, the planner, chunked mode, plan-phase, quick and quick-batch change text
  again, and `execute_phase` gates on a half-planned phase.

## Would reopen it

- A kernel mechanism for rules that stay open while authoring: the Limits close, and D3's return path may become a
  recorded question. It happened: ADR 0385, which replaced D3.
- A CLI batch edit.
- A project whose spec is not source.
