# A live plan-phase run under gsd-strict

A real agent ran the plan-phase prompt of `@gsd/core` end to end, on a real phase of this repository's own spec, under
`profile: gsd-strict`: to see the workflow, its agents and the profile's refusals meet outside a fixture, and to see the
thread step run before the plan check. This file records what the run did, every refusal it met and how it was cleared,
and every defect it showed, fixed or logged.

Each statement about a refusal or a rule names the test of this repository that pins the same behaviour, or says
"observed in the run only". The transcripts and stream logs of the run are archived outside the repository. The run's
spec writes are not in the repository either: their ledger lines are not, so they would not check on a fresh checkout.
This record quotes their shape and counts. The decisions the run led to are
[ADR 0384](../../docs/adr/0384-a-phases-plans-are-written-in-one-edit-and-its-spec-is-committed-like-code.md).

## Run

| | |
|---|---|
| Commit | `37cc5a471` |
| `specforge --version` | `specforge 0.3.0 (37cc5a471, protocol 2.3.0)` |
| Host | Claude Code 2.1.330, `claude -p "/gsd-plan-phase v1.0.0/03 --research --text"` |
| Models in the stream | `claude-opus-5-5` (the session, the planner), `claude-sonnet-5-5` (the researcher, the checker) |
| Profile | `gsd-strict` |
| Phase | `v1.0.0/03` "Graph Protocol Schema Versioning": two requirements (FEATURE-120, FEATURE-048), the feature already built |
| Date | 2026-11-21 |
| Turns | 2: the first stopped at the workflow's own question, the second ran to the end |
| Wall time | 32 s, then 1245 s |
| Cost | $0.67, then $8.11 |
| Model turns | 11, then 38 |
| Tool calls (second turn) | 200: 37 by the session, 28 by the researcher, 69 by the planner, 32 by the checker, 18 by the revising planner, 16 by the re-verifying checker |
| Spawned agents | 5: researcher, planner, plan checker, planner (revise), plan checker (re-verify) |

## Setup

An isolated clone of this repository at that commit, with no remote; the binary built from that commit first on `PATH`.

```bash
specforge --version
specforge settings set @gsd/core profile gsd-strict
specforge install claude-code
specforge check              # 0 errors, 0 warnings, 0 infos
claude -p "/gsd-plan-phase v1.0.0/03 --research --text" \
  --output-format stream-json --verbose --permission-mode default \
  --allowedTools "<the allowlist below>" --disallowedTools WebFetch WebSearch \
  --strict-mcp-config --mcp-config .mcp.json --setting-sources project,local
```

The second turn resumed the same session with the operator's answer as its prompt.

## Operator rules and permissions

- Allowed: `Bash(specforge:*)`, `Bash(git:*)`, `Bash(cat:*)`, `Bash(ls:*)`, `Bash(grep:*)`, `Bash(head:*)`,
  `Bash(tail:*)`, `Bash(wc:*)`, `Bash(mkdir:*)`, `Bash(echo:*)`, `Bash(printf:*)`, `Bash(test:*)`, `Bash(jq:*)`,
  `Bash(python3:*)`, Read, Edit, Write, Glob, Grep, Agent, Task, TodoWrite, and the `specforge` MCP server's tools.
- Denied: WebFetch, WebSearch. Every other tool and command needed approval, which a non-interactive run cannot give, so
  it was denied.
- `--permission-mode default`, `--setting-sources project,local`: only the clone's own settings and MCP configuration.
- The operator answered only the workflow's own questions, and wrote nothing into the clone during the run.

## Steps

Times are the session's (UTC), observed in the run only. A step number is the plan-phase workflow's; "thread" is the
step the prompt's step list runs between the planner and the plan checker.

| Time | Workflow step | What the agent ran | Outcome |
|---|---|---|---|
| 03:30:50 | render | MCP `specforge.prompt` of `specforge.gsd.plan_phase`, argv `v1.0.0/03 --research --text` | the prompt and its step list |
| 03:31:09 | phase | `specforge query v1.0.0/03` | the phase, its two requirements, no plan, no research |
| 03:31:13 | 5.6, plan:pre hooks | `specforge gsd <ai, codebase, ops, security, ui>-hooks plan:pre` | exit 2 each: none served, their steps skipped |
| | 4, context | none found | the workflow's question: continue without context, or discuss first; the first turn ends |
| 03:31:53 | 5, research | the researcher spawned | one research, four findings, three pitfalls; two open questions refused (see Refusals) |
| 03:33:34 | 5, research | `specforge gsd record-research v1.0.0/03/03-RESEARCH` | exit 0: the pack's digest recorded |
| 03:33:54 | 5.5, validation | `specforge new validation 03-VALIDATION ...` | written (its first form, a compound line, was denied by the allowlist) |
| 03:34:10 | 7.95, spec-less probe | `specforge gsd edge-probe v1.0.0/03 --format json` | 7 applicable probes, 7 unresolved (three forms redirecting to a file were denied first; observed in the run only) |
| | 7.8, pattern mapper | none | skipped: the pattern mapper is not served |
| 03:34:46 | 8, plan | the planner spawned | three refused dry runs and an accepted one, then one 34-op `specforge.edit` written at 03:44:26; returned at 03:44:54 |
| 03:44:58 | thread | `specforge gsd thread-plan v1.0.0/03` (shell, redirected) | denied by the allowlist (an output redirection) |
| 03:45:00 | thread | MCP `specforge.gsd.thread_plan` for `v1.0.0/03` | four entries: execute 03-01, implement T1, T2, T3 |
| 03:45:05 | thread | `specforge trace --plan <the entries> --format json` | `ok: true`, no gap |
| 03:45:14 | check | `specforge check --format json` | the plan's four judged rules without a verdict, and I213 |
| 03:45:29 | 10, plan check | the plan checker spawned | ISSUES FOUND: 0 blockers, 1 warning, 3 info; verdicts recorded 03:47:46 to 03:48:14 |
| 03:48:53 | 12, revise | the planner spawned to revise | two `specforge.edit` calls of set ops (10, then 1) on the plan's tasks |
| 03:50:53 | 12, re-verify | the plan checker spawned again | VERIFICATION PASSED |
| 03:52:05 | final | `specforge trace --route gsd/requirement-thread --in v1.0.0/03 --partial`, `specforge check` | both requirements threaded to the plan's tasks; one info (I213) |
| 03:52:09 | final | `rm` of the thread entries file | denied by the allowlist: the file stayed in the ignored `.specforge/` |

## Refusals

| Agent | Code | Message as the run printed it | The call | How it was cleared | Proof |
|---|---|---|---|---|---|
| researcher | E080 (`compilation_failed`) with gsd/W204 twice | "the edit would leave the project with 2 errors it does not have; nothing was written"; "open question 'v1.0.0/03/03-RESEARCH/03-Q1' is resolved by no decision" (and 03-Q2) | one 9-op `specforge.edit`: four findings, three pitfalls, two open questions, batched against the researcher's rule of one item per call, so the two refused questions took the seven other items down with them | sent again as 7 ops without the two questions: they were lost | `assets_shell::researchers::an_open_question_strict_refuses_is_returned_not_dropped` (the rule now names a batch refused this way and says to send it again without the refused question); `assets_shell::researchers::plan_phase_records_the_users_answer_then_the_question` |
| planner | `invalid_input` | "'verify_paths' of task is computed by a pass of @gsd/core and is never written" | a 2-op dry run: the plan and one task | the field left out | observed in the run only |
| planner | E080 (`compilation_failed`) with gsd/W309 and gsd/E301 | "plan 'v1.0.0/03/03-01' has no truth and no artifact"; "requirement 'FEATURE-120' of phase 'v1.0.0/03' is in no plan of the phase" | the same 2-op dry run | the whole plan in one edit | `assets_shell::planner_edit::a_plan_written_alone_is_refused_and_nothing_is_written` |
| planner | `invalid_input` | "edit 2 (create task T1): 'rationale' is not a member of 'reversibility' (members: rating, why, settled_by)" | a 34-op dry run | the member renamed from the schema | observed in the run only |
| planner | none: accepted | gsd/W311, gsd/W313, gsd/W315 (errors) and gsd/W316 (a warning): "is not checked ...: no verdict", and I213 | the 34-op dry run, then the same edit written | nothing to clear: judged rules, settled by the checker's verdicts | `assets_shell::planner_edit::the_planner_writes_a_phase_in_one_edit`: the edit accepted, each plan owing exactly these four, errors under gsd-strict (warnings under gsd-compat) but gsd/W316 |
| plan checker | E062 | "the token budget of 8000 cannot hold route `gsd/plan-checker`'s core sections for `v1.0.0/03/03-01` (21039 tokens: the envelope, the subject and its core sections); raise the budget" | `specforge context gsd/plan-checker v1.0.0/03/03-01` | read the plan with `specforge query` instead | observed in the run only |
| plan checker | a warning of the check | the plan's gate commands used `cargo gate quick`, which picks its checks from uncommitted changes and, run after the task's commit, checks nothing | round 1 of the plan check | the revision wrote `cargo gate full --only links,file-size,specs`; the re-verify read the gate's source | observed in the run only |
| the allowlist | denied | 21 shell calls: five `cargo test`, `cargo nextest` or `cargo gate` runs, output redirections, compound lines and one `rm` | see Steps | each agent used an allowed form or went without | observed in the run only |

## Interventions

One. The first turn stopped at step 4: the phase had no context from a discussion, and the workflow asked whether to
continue without it (1) or to run the discuss step first (2). The operator answered "1"; the second turn ran from there
to the end with no other answer.

## Final state

Each line observed in the run only.

- `specforge check`: 0 errors, 0 warnings, 1 info (I213: values that wait for their owner to complete).
- `specforge gsd progress-report`: phase 03 "Planned", with 1 plan; the re-verify of `v1.0.0/03`: 0 items.
- The phase's file grew by 96 lines (the research with its findings and pitfalls, and the validation strategy); one plan
  file of 513 lines: 1 plan, 3 tasks, 11 truths, 7 artifacts, 3 key links, 5 prohibitions, 4 validation rows, written
  by one 34-op `specforge.edit`, then the checker's verdicts.
- The planner made one commit, "docs(03): create phase plan", holding the plan file. The orchestrator's commit step read
  a setting `@gsd/core` does not have, took it as unset and skipped: the research, the validation strategy, the
  revision and the verdicts stayed uncommitted. The run's own summary said the commit was skipped because
  `commit_docs` is unset, and named the planner's commit beside it.

## Thread before the check

Observed in the run only (the session's times, UTC):

| Time | Event |
|---|---|
| 03:44:54 | the planner returned |
| 03:44:58 | `specforge gsd thread-plan v1.0.0/03` (denied as a redirected shell line; run at 03:45:00 through MCP) |
| 03:45:05 | `specforge trace --plan`: `ok: true`, no gap |
| 03:45:29 | the plan checker spawned |

The order the prompt prescribes, the thread step before the plan check, is pinned by
`flow_strict::plan_phase_renders_under_strict_with_the_thread_before_the_check` and
`prompts_plan::plan_phase_threads_the_plans_before_the_check`.

## Defects

| What the run showed | Where | Fixed or logged | Commit on this branch | Tests or reproduction |
|---|---|---|---|---|
| the planner was told to write plan by plan with typed creations, a route a phase refuses (gsd/E301 in every profile, gsd/W309 under gsd-strict); the agent found the one-edit route after refused dry runs | the planner's body and every writer of a phase's plans | fixed | `a28be302f` docs(adr): a phase's plans are written in one edit and its spec is committed like code (ADR 0384); `89fdcf147` fix(gsd): the planner writes a phase's plans in one specforge.edit, the only route a phase accepts; `85ed24dfd` fix(gsd): every writer of a phase's plans names the plan edit, and its uncleared refusals are pinned | `assets_shell::planner_edit::the_planner_writes_a_phase_in_one_edit`, `assets_shell::planner_edit::a_dependency_found_later_is_linked`, `assets_shell::planner_edit::a_plan_written_alone_is_refused_and_nothing_is_written`, `assets_shell::planner_edit::the_plan_format_is_one_edit_of_the_whole_phase`, `assets_shell::planner_edit::every_writer_of_a_phases_plans_names_the_plan_edit`, and the quick pin `cmd_quick::a_quick_plan_minted_without_must_haves_is_refused_under_strict_only` |
| the commit step read `commit_docs`, a key `@gsd/core` does not declare, and skipped; a dozen other steps called it a setting or branched on it | plan-phase step 13d and every asset naming it | fixed | `9fa0bcbac` fix(gsd): plan-phase commits its spec, and reads no commit_docs setting it does not have; `0e72efc5c` fix(gsd): no workflow calls commit_docs a setting or branches on it; the roster decides; `d3636f495` fix(gsd): every line naming commit_docs says it is not a setting here, and no read of it is allowed; `07e9f7547` fix(gsd): the settings workflow counts the settings of its section layout, and a test holds it; `bd7dea4aa` fix(gsd): the committed spec's row of keys is swept whole, uncommitted mode with it; `9379e76c7` fix(gsd): the executor surfaces a gitignored spec as an error, and no step commits it conditionally | `assets_shell::commits::the_plan_commit_step_commits_the_spec_and_an_empty_one_says_nothing_to_commit`, `settings_roster::the_plan_phase_family_reads_no_commit_docs`, `settings_roster::every_key_an_asset_calls_a_setting_is_a_key_of_the_roster`, `settings_roster::no_asset_names_a_not_applicable_key`, `settings_roster::the_settings_workflow_counts_the_settings_of_its_section_layout`, `settings_roster::no_asset_makes_the_spec_commit_conditional_or_skips_an_ignored_spec`, `ported_text::every_setting_a_line_reads_is_a_key_of_the_roster` |
| the researcher dropped its two open questions to get the research written | the researcher's rule and the handling of its return by plan-phase, quick and quick-batch | the text fixed: the question is returned, the planner records no answer of its own, and the user's answer is recorded with the question after the planner; the refusal itself logged | `fd1a8d45e` fix(gsd): an open question gsd-strict refuses is returned to the planner, never dropped; `41b713f8e` fix(gsd): the planner leaves a research question to the user, whose answer is recorded with it | `assets_shell::researchers::an_open_question_strict_refuses_is_returned_not_dropped`, `assets_shell::researchers::plan_phase_records_the_users_answer_then_the_question`, `assets_shell::researchers::quick_and_quick_batch_record_the_users_answer_then_the_question`; reproduction under gsd-strict: `specforge new open_question OQ2 "Sort order" --in <the phase's file> --research <the phase's research> --question "..." --unclear "..."` without `--resolved_by` exits 1, E080 with gsd/W204, nothing written; the fix that would let it through is the kernel question below |
| the agents called the phase "Phase 12: Graph Protocol Schema Versioning" while every command said `v1.0.0/03` | the phase entity's own title | logged: not a product defect, the agents echoed the spec | none | `grep -n '"Phase 12: ' spec/product/milestones.spec` prints `237:phase 03 "Phase 12: Graph Protocol Schema Versioning" {`; 13 phases of that file carry a "Phase N:" title in a global numbering; the rendered prompt holds no "12" |
| found reading the same class, not met by the run: chunked mode writes a phase's first plan alone, refused gsd/E301 in every profile when the phase's requirements span plans | chunked planning mode | logged and pinned; chunked mode reads the planner's `## PLAN BLOCKED` as blocked and offers to plan the rest of the phase in one run | `85ed24dfd` (above); `63715598f` fix(gsd): chunked mode reads a blocked plan as blocked, and its recovery writes only missing plans | `assets_shell::planner_edit::a_whole_plan_written_alone_is_refused_while_a_requirement_of_its_phase_is_in_no_plan`, `assets_shell::planner_edit::the_rest_of_a_chunked_run_writes_only_the_plans_the_graph_lacks`, `assets_shell::planner_edit::every_writer_of_a_phases_plans_names_the_plan_edit` |
| found reading the same class, not met by the run: quick's plan mint is refused gsd/W309 under gsd-strict | quick's mint line | logged and pinned | `85ed24dfd` (above) | `cmd_quick::a_quick_plan_minted_without_must_haves_is_refused_under_strict_only` |
| the plan checker's context route could not hold its core sections for the run's plan (E062, 21039 tokens against 8000) | the `gsd/plan-checker` route's budget | logged | none | observed in the run only: the plan is not in the repository; `specforge context gsd/plan-checker <a plan>` on a plan of that size |

## Limits of the run

- `cargo` was denied by the allowlist, so no agent ran the phase's tests: the plan and the checks rest on reading the
  code, as the checker said in its report.
- Plan-phase only: the executor and the verifier were not run live.
- One phase, one host, one pair of models.
- The phase's feature was already built: the plan closes requirement threads, it builds nothing.

## Open for the owner

- The kernel question of ADR 0384's Limits: E080 counts, on the write that creates its entity, a finding a later step of
  the same workflow settles (an open question under gsd-strict, chunked mode's first plan, quick's mint under
  gsd-strict). Whether E080 should stop counting such a finding is a kernel decision.
- The `gsd/plan-checker` route's budget against a plan of a real phase.

## Closed since the run

- The other rows of the settings' Not applicable table are swept as the row of `commit_docs` was: one guard,
  `settings_roster::no_asset_names_a_not_applicable_key`, reads every row but the secrets one from
  `gsd/core/spec/settings.md` and scans the assets and the prompt and install goldens. The advanced settings no
  longer ask for `graphify.build_timeout` or `workflow.auto_prune_state`
  (`settings_roster::the_advanced_settings_workflow_reads_and_confirms_only_settings`).
- The spec's thirteen phases that carried a global "Phase N:" number are titled by their names, and absorb, which
  wrote those titles, titles a converted phase by its name (`tests::roadmap` and `absorb_e2e` in `xtask`).
