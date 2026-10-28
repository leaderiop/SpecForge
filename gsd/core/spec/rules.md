# Rules

Every GSD check as a SpecForge rule. Codes are namespaced (K10): `gsd/` is `@gsd/core`.

| Kind | Meaning |
|---|---|
| decl | a declarative rule descriptor (`missing_required_field`, `field_value_constraint`, `cycle_detection` …) |
| pass | a check pass over the graph, run on every compile |
| judged | a judged rule (K7): holds while a fresh verdict from the named agent says so |
| guard | enforced by the host when it applies an edit plan or runs a command, not on compile; a guard pass that declares `reports` also answers at a gate |

**compat** is the severity under `gsd-compat` (as GSD has it), **strict** under `gsd-strict` and `gsd-plus`.
`—` means the rule is off: a profile switches a rule off only through a gate setting it presets to false (K6), and
raises a severity, never lowers it. A code's letter is its compat severity (for a SpecForge-only rule, its severity
when on); a stricter profile raises the severity and the code
keeps its letter; an `I` code is information under every profile. A rule GSD runs only behind a toggle is off, under
every profile, while that setting is false; its letter is still the severity it has when on
([Setting-gated rules](#setting-gated-rules)). GSD's source names the gate module
(`src/gate-*.cts`), the plan-checker dimension (`D1`…) or the workflow that holds the check today.

## Project and roadmap

| Code | Rule | Kind | compat | strict | GSD source |
|---|---|---|---|---|---|
| gsd/E101 | every v1 requirement maps to exactly one phase (waits for the first phase: requirements written before the roadmap are not unmapped yet) | pass | E | E | roadmapper coverage, `REQUIREMENT_UNMAPPED` |
| gsd/E102 | phase and plan `depends_on` are acyclic and point at earlier ones only | decl + pass | E | E | D3 |
| gsd/W103 | a phase has 2–5 success criteria (reads the list's items) | pass | W | E | roadmap template |
| gsd/W104 | phase numbers (GSD's `\d+[A-Z]?(\.\d+)*`: `12A`, `3.2.1`) are continuous, one warning per run of missing integers; two ids do not share a number; an inserted phase follows the phase it is numbered from. A name outside the grammar and a `999.x` backlog phase are exempt | pass | W | W | roadmap |
| gsd/E105 | an `out_of_scope` requirement has a reason | decl | E | E | requirements template |
| gsd/E106 | a milestone completes only with a passing (or accepted) audit | guard | E | E | complete-milestone |
| gsd/E107 | a timestamp field holds an RFC 3339 UTC time (every authored `ts` field, and every authored `ts` member of an authored block, is a `str` that `matches` `YYYY-MM-DDTHH:MM:SS[.f]Z` with ASCII digits and RFC 3339's ranges for month, day, hour, minute and second; the day is not checked against the month; generated per field and member, and no kind has one yet: `project.updated` is computed, and the first is `blocker.resolved.at`, r38) | decl | E | E | (new: SpecForge has no timestamp type) |
| gsd/E108 | a repository has one `project` (the kernel keeps `singleton` as metadata, so the second and later `project` entities, by id, are reported) | pass | E | E | (new: `PROJECT.md` is one file) |

## Discuss, spec, research

| Code | Rule | Kind | compat | strict | GSD source |
|---|---|---|---|---|---|
| gsd/E201 | a context has canonical refs (`missing_required_field` and `non_empty` under one code) | decl | E | E | `gate-phase-context`, context template |
| gsd/E202 | a phase spec meets its ambiguity gates | judged (prompt `spec_phase`) | E | E | spec-phase |
| gsd/W203 | a locked requirement has a falsifiable acceptance | judged (prompt `spec_phase`) | W | E | spec template |
| gsd/W204 | an open question of a phase's research is resolved by a decision before planning (the project's research is exempt: the plan checker reads the phase's `RESEARCH.md` only) | pass | W | E | plan checker (research resolution) |
| gsd/W205 | a package in research has a legitimacy verdict | judged (phase researcher) | W | E | package-legitimacy audit |
| gsd/E206 | a phase has at most one context, phase spec, discussion log and research, and the project at most one research (the kernel's scopes would allow two; the second and later, by id, are reported) | pass | E | E | (new: each is one file of the phase directory) |

## Plan

The plan checker's dimensions, and the gates `gsd-tools check` runs at `plan:post`.

| Code | Rule | Kind | compat | strict | GSD source |
|---|---|---|---|---|---|
| gsd/E301 | every requirement of a phase is in at least one of its plans | pass | E | E | D1 |
| gsd/E302 | a plan names at least one requirement (`missing_required_field` and `non_empty` under one code) | decl | E | E | plan template (`requirements` non-empty) |
| gsd/E303 | an `auto` or `tdd` task has files, action, automated verify and done | decl | E | E | D2 |
| gsd/W304 | a plan has at most 3 tasks (4 warns) | pass | W | E | D5 |
| gsd/W305 | plans of one wave share no path in `files_modified ∪ files_deleted`; under compat that wave runs sequentially | pass | W | E | wave guard (#3003), execute-phase overlap check |
| gsd/W306 | a task's files are among its plan's `files_modified` | pass | W | E | plan checker (task files) |
| gsd/E307 | `auto_select` and `answer.option` name one of the checkpoint's options; each checkpoint kind has its fields | decl + pass | E | E | checkpoint validation |
| gsd/E308 | a `one_way` task follows a decision checkpoint or a settled decision | pass | E | E | reversibility gate |
| gsd/W309 | a plan has a truth and an artifact | pass | W | E | must-haves |
| gsd/E310 | every settled decision of the phase is honoured by some plan | pass | E | E | `gate-decision-coverage-plan` |
| gsd/W311 | the plan achieves its phase goal, keeps to scope and contradicts no settled decision | judged (plan checker) | W | E | D7 and the goal and scope dimensions |
| gsd/E312 | no task action reduces scope ("for now", "simplified", "v1") | pass | E | E | D7b |
| gsd/W313 | key links are wired between artifacts the plan names | judged (plan checker) | W | E | plan checker (key links) |
| gsd/E314 | each task has an automated verify or a wave-0 item (Nyquist) | pass | E | E | D8, `nyquist` |
| gsd/W315 | data contracts between plans match (`requires` / `provides`) | judged (plan checker) | W | E | plan checker (data contracts) |
| gsd/W316 | the plan follows the project's agent instructions and the mapped patterns | judged (plan checker) | W | W | plan checker (CLAUDE.md, patterns) |
| gsd/W317 | the post-planning gap report finds nothing open | pass | W | W | `gate-gap-analysis-plan-post` |
| gsd/W318 | the plan-check verdict is fresh before execution (reads the judgements K7 hands the guard pass) | guard | W | E | plan-check loop (up to 3 rounds) |
| gsd/E319 | a prohibition with `verification test` names a test | pass | E | E | `gate-prohibition-enforcement` |
| gsd/E320 | a task's verify command names paths that exist | pass | E | E | `gate-verify-command-paths` |
| gsd/I321 | same-wave non-file coupling is declared by `depends_on` or `coupling_justified` naming the peer | pass | I | I | D3b |
| gsd/E322 | a plan has fewer than 5 tasks | pass | E | E | D5 (5 or more is a blocker) |
| gsd/E507 | each task's verify commands name their failing direction (`fails_when`), one statement per `automated` | pass | E | E | `gate-verify-failure-directions`, plan checker 8f |

## Execute

| Code | Rule | Kind | compat | strict | GSD source |
|---|---|---|---|---|---|
| gsd/E401 | a summary's `requirements_completed` equals its plan's `requirements` | pass | E | E | execute-plan |
| gsd/E402 | a halted summary says why | decl | E | E | summary template |
| gsd/E403 | a `tdd` task's GREEN commit follows a recorded RED failure | pass | E | E | `gate-tdd-red-evidence` |
| gsd/W404 | the TDD review checkpoint ran after execution | guard | W | E | `gate-tdd-review-checkpoint` |
| gsd/E405 | the schema has not drifted from the plan's declared migrations | pass | E | E | `gate-schema-drift` (`execute:wave:post`) |
| gsd/W406 | the context the plan was written against has not drifted (the `context_digest` a research, spec or validation recorded differs from the context's current digest) | pass | W | E | `gate-context-drift` |
| gsd/W407 | every task has a commit | pass | — | W | (new: the git collector) |
| gsd/W408 | a summary's deliverable checks pass or are human-judged with a rationale | pass | W | E | summary coverage |

## Verify

| Code | Rule | Kind | compat | strict | GSD source |
|---|---|---|---|---|---|
| gsd/E501 | the verifier's verdict on each truth, artifact and key link is fresh | judged (verifier) | E | E | verifier, honest-verifier |
| gsd/E502 | an explicit truth is verified only with a named passing test | pass | E | E | honest-verifier |
| gsd/E503 | an override has its human author and date (`missing_required_field` on `by` and `at`) | decl | E | E | verifier overrides |
| gsd/W504 | more than three overrides in a phase | pass | W | W | verifier |
| gsd/E505 | debt markers (TBD, FIXME, XXX) in the phase's files link to an issue | pass | E | E | verifier |
| gsd/W506 | every settled decision maps to something shipped | pass | W | W | `gate-decision-coverage-verify` |
| gsd/E509 | a UAT passes only with no pending, blocked, unresolved or undeferred skipped test | pass | E | E | UAT predicate |
| gsd/W510 | a planned phase has a validation, validated before verify | pass | W | E | Nyquist 8a–8f |
| gsd/E511 | a phase is complete only when verification passed and UAT is complete (part of the phase-status derivation; a gate code of `complete_milestone` and `/gsd-ship`) | guard | E | E | phase-status ladder |
| gsd/E512 | no broken window is open at ship | guard (ship:pre) | E | E | broken-windows gate |
| gsd/E513 | a resolved window cannot be waived or fixed again (`waiver` and `fixed_at` are never both set) | pass | E | E | `src/broken-windows.cts` |
| gsd/W514 | no refactor `deviation` window is recorded while `workflow.windows_enforce` is off | pass | W | W | `refactor_strict_not_enforcing` |
| gsd/E515 | a complete requirement traces to a task, a commit and a passing test | pass | — | E (plus) | (new: gsd-plus) |
| gsd/E516 | a stale verdict blocks every workflow step (reads the judgements K7 hands the guard pass) | guard | — | E (plus) | (new: gsd-plus) |
| gsd/E517 | active plans of different workstreams share no path in `files_modified ∪ files_deleted` | pass | — | E (plus) | (new: gsd-plus) |
| gsd/E518 | the integration checker's verdict on a milestone audit is fresh | judged (integration checker) | E | E | audit-milestone integration check |

## Retired and kernel-provided

- `gsd/E508` (what a verdict cites exists when it is recorded) is retired: the host already refuses it. `set_verdict`
  is refused `verdict_refused` with reason `cite` (ADR 0289 D6), and a changed cited input stales the verdict (ADR 0290
  D3). The roster maps `gate-evidence` to that refusal.
- Rules that read project files are *buildable* since protocol 1.11.0 (K11), and are not yet built (r37 for `gsd/E320`, r39 for the rest): `gsd/E320`, `gsd/E505`, and the file facts under
  `gsd/E501` (artifact `missing`, `min_lines`, `contains`, `exports`; a key link's `pattern`) declare `reads` and are sent
  the facts and capped text of those files.
- Rules that read the test report in a compile pass are buildable since 1.11.0 (K12), not yet built, by declaring `reads_tests`: `gsd/E403`,
  `gsd/E502`, `gsd/W408`, `gsd/E515`.
- Rules that count list items are buildable since 1.11.0 (K13): `gsd/W103` is built (r36); `gsd/W305`, `gsd/W306` and `gsd/E517`
  are not yet, and read the items in `values`.
- A judge that is a prompt, not an agent (`gsd/E202`, `gsd/W203`), is buildable since 1.11.0 (K15), not yet built: `prompt:spec_phase`.

## Setting-gated rules

GSD runs these checks only while a toggle is on: the `when` of the capability's gate, or the workflow's skip. The rule
is off while the setting is false, whatever the profile; a profile only presets the setting
([settings.md](settings.md#profiles)).

| Setting | Default (gsd-compat) | gsd-strict, gsd-plus | Rules |
|---|---|---|---|
| `workflow.windows_enforce` | false | true | gsd/E512 |
| `workflow.tdd_mode` | false | false | gsd/E403, gsd/W404 |
| `workflow.nyquist_validation` | true | true | gsd/E314, gsd/W510 |
| `workflow.plan_check` | true | true | gsd/W311, gsd/W313, gsd/W315, gsd/W316, gsd/W318 |
| `workflow.post_planning_gaps` | true | true | gsd/W317 |
| `workflow.context_coverage_gate` | true | true | gsd/E310, gsd/W506 |
| `workflow.schema_drift_gate` | true | true | gsd/E405 |
| `workflow.context_drift_precheck` | true | true | gsd/W406 |
| `specforge.task_commits` | false | true | gsd/W407 |
| `specforge.traceability_gate` | false | false (gsd-plus: true) | gsd/E515 |
| `specforge.stale_verdict_gate` | false | false (gsd-plus: true) | gsd/E516 |
| `specforge.cross_workstream_conflicts` | false | false (gsd-plus: true) | gsd/E517 |

## Judged rules' inputs

Each judged rule's named inputs (K7 D8), and the fields its verdict binds (`V`). Every verdict also digests its `cited`
and `cited_tests` lists. `file("p")` paths are from the project root; `file(S.f)` reads its field's base, `project` for
every @gsd file field (K3 D5); `test()` reads the collected test results (K7 D11). A gated rule is never evaluated, so it
is neither fresh nor stale.

| Rules | Target | Inputs | Binds |
|---|---|---|---|
| gsd/W311, W313, W315, W316 (plan check) | plan | `plan` self · `body` self<task\|checkpoint\|truth\|artifact\|key_link\|prohibition\|user_setup.plan · `phase` self.phase · `requirements` self.requirements · `decisions` self.phase<decision.phase · `all_decisions` decision (a superset of `decisions`: the selector grammar cannot filter on an unset `phase`, so any decision change stales the plan check, the safe side) · `context` self.phase<context\|deferred_idea.phase · `research` self.phase<research.phase · `research_items` self.phase<research.phase<finding\|stack_choice\|pattern\|pitfall\|dont_hand_roll\|open_question.research · `siblings` self.phase<plan.phase · `instructions` file("CLAUDE.md", "AGENTS.md") | — |
| gsd/E501 | truth | `truth` self · `plan` self.plan · `phase` self.plan.phase · `code` file(self.plan.files_modified) · `tests` test(self) | — |
| gsd/E501 | artifact | `artifact` self · `plan` self.plan · `code` file(self.path) | — |
| gsd/E501 | key_link | `link` self · `plan` self.plan · `from` file(self.from) · `to` file(self.to); not judged while `pattern` is set (`unless_set pattern`: a pass searches it) | — |
| gsd/E202 | phase_spec | `spec` self · `items` self<locked_req\|interview.spec · `phase` self.phase | `scores` (its recorded values) |
| gsd/W203 | locked_req | `req` self · `traces` self.traces | — |
| gsd/W205 | package_audit | `audit` self | `legitimacy` (its outcome: legitimate \| suspicious \| unknown, passing legitimate) |
| gsd/E518 | milestone_audit | `audit` self · `phases` self.milestone<phase.milestone · `verifications` self.milestone<phase.milestone<verification.phase · `requirements` self.milestone<phase.milestone.requirements | `integration` (its outcome: passed \| issues, passing passed) |

A summary is no plan-check input (writing it after execution stales nothing), and no verifier input either (its claims
are not evidence).

## Derivations, not rules

These are computed fields, not checks; they are listed so the parity suite can compare them with GSD's own values:
phase status, plan wave, `autonomous`, plan status (blocked by halt propagation), requirement status, milestone state,
verification status, score and re-verification, UAT totals and status, validation status, audit status, broken-window status and the
project's window counts, refactor proposal status, `STATE.md` progress.

## Elsewhere

GSD gates that belong to capability extensions keep their GSD source there: the UI gates (`gate-ui-plan`,
`gate-ui-safety`) in `@gsd/ui`, API coverage (`gate-api-coverage-verify-pre`) in `@gsd/ai`, codebase drift
(`gate-codebase-drift`) in `@gsd/codebase`, open threats at `ship:pre` in `@gsd/security`. Broken windows
(`open_count == 0` at `ship:pre`, when `workflow.windows_enforce`) stays in `@gsd/core`, as gsd/E512. The remaining `gate-*` modules (`args`, `config`, `exit`, `predicate`, `predicate-evaluator`,
`evaluation-scope`, `verdict`) are GSD's gate machinery, which the kernel replaces. The harness ledger's checks (an `H`
value with no ledger line) are kernel diagnostics (K4), not `gsd/` codes.
