# Rules

Every GSD check as a SpecForge rule. Codes are namespaced (K10): `gsd/` is `@gsd/core`.

| Kind | Meaning |
|---|---|
| decl | a declarative rule descriptor (`missing_required_field`, `field_value_constraint`, `cycle_detection` …) |
| pass | a check pass over the graph, run on every compile |
| judged | a judged rule (K7): holds while a fresh verdict from the named agent says so |
| guard | enforced by the host when it applies an edit plan or runs a command, not on compile; a guard pass that declares `reports` also answers at a gate |
| record | a context route's stale code: the host re-reads a recorded pack on compile and reports the record when the pack moved ([Context routes](#context-routes)) |

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
| gsd/E102 | phase `depends_on` is acyclic and points at earlier phases of its milestone only; plan `depends_on` is acyclic and names plans of its own phase only (a token that does not resolve in the phase is dropped by GSD, which then places the wave unreliably; a quick plan is checked for cycles only) | pass | E | E | D3, `src/phase.cts` (`depends_on` resolution and cycle error) |
| gsd/W103 | a phase has 2–5 success criteria (reads the list's items) | pass | W | E | roadmap template |
| gsd/W104 | phase numbers (GSD's `\d+[A-Z]?(\.\d+)*`: `12A`, `3.2.1`) are continuous, one warning per run of missing integers; two ids do not share a number; an inserted phase follows the phase it is numbered from. A name outside the grammar and a `999.x` backlog phase are exempt | pass | W | W | roadmap |
| gsd/E105 | an `out_of_scope` requirement has a reason | decl | E | E | requirements template |
| gsd/E106 | a milestone completes only with a passing (or accepted) audit: at the gate, the subject milestone's latest audit is `passed`, or `tech_debt` with a person's `debt_accepted`; no audit is refused too. Silent outside a gate | guard | E | E | complete-milestone |
| gsd/E107 | a timestamp field holds an RFC 3339 UTC time (every authored `ts` field, and every authored `ts` member of an authored block, is a `str` that `matches` `YYYY-MM-DDTHH:MM:SS[.f]Z` with ASCII digits and RFC 3339's ranges for month, day, hour, minute and second; the day is not checked against the month; generated per field and member. `project.updated` is computed, so the first is `blocker.resolved.at` (r38, fired by a black-box test); `answer.at`, `done.at`, `pause.at` and `blocker.raised` are harness values and get none) | decl | E | E | (new: SpecForge has no timestamp type) |
| gsd/E108 | a repository has one `project` (the kernel keeps `singleton` as metadata, so the second and later `project` entities, by id, are reported) | pass | E | E | (new: `PROJECT.md` is one file) |

## Discuss, spec, research

| Code | Rule | Kind | compat | strict | GSD source |
|---|---|---|---|---|---|
| gsd/E201 | a context has canonical refs (`missing_required_field` and `non_empty` under one code) | decl | E | E | `gate-phase-context`, context template |
| gsd/E202 | a phase spec meets its ambiguity gates | judged (prompt `spec_phase`) | E | E | spec-phase |
| gsd/W203 | a locked requirement has a falsifiable acceptance | judged (prompt `spec_phase`) | W | E | spec template |
| gsd/W204 | an open question of a phase's research, or a quick plan's, is resolved by a decision before planning (the project's research, the one named `RESEARCH`, is exempt: the plan checker reads the phase's `RESEARCH.md` only; a quick plan's research, which names it in `plan`, is judged as a phase's, since upstream's quick planner reads its `RESEARCH.md` too; a research with neither `phase` nor `plan` and another name is E206 and is judged as a phase's. Upstream's plan checker reports this as `severity: blocker`; here it is a warning by design (D-r36-12), an Error under `gsd-strict`, where it blocks `/gsd-plan-phase`) | pass | W | E | plan checker (research resolution) |
| gsd/W205 | a package in research has a legitimacy verdict | judged (phase researcher) | W | E | package-legitimacy audit |
| gsd/E206 | a phase has at most one context, phase spec, discussion log and research, a quick plan at most one context and research, and the project at most one research, which must be named `RESEARCH` (the kernel's scopes would allow two; the second and later, by id, are reported, a quick plan's counted by the `plan` they name, since they share the project scope; a research with neither `phase` nor `plan` that is not named `RESEARCH` is always reported, with the suggestion to add its `phase`). It also holds the owner: a context names exactly one of `phase` and `plan`, a decision and a research at most one, a `plan` must be a quick plan, one with no phase, and a quick plan's research is not named `RESEARCH`, the project's research's name, which gsd/W204 exempts (widened in G3 rather than a new code: E206 already held the owner rule of a phaseless research) | pass | E | E | (new: each is one file of the phase or quick task directory) |
| gsd/E523 | an edge or spec prohibition of a phase spec carries what its status allows: a resolved row its tier (and an edge its resolution), a dismissed row its reason, an unresolved row neither; its requirement is a locked requirement of its spec, and one edge per requirement and category (ADR 0333; the `check_*` descriptor is not judged) | pass | E | E | probe-core validateResolution, edge-probe analyzeCoverage |

## Plan

The plan checker's dimensions, and the gates `gsd-tools check` runs at `plan:post`.

| Code | Rule | Kind | compat | strict | GSD source |
|---|---|---|---|---|---|
| gsd/E301 | every requirement of a planned phase is in at least one of its plans' `requirements`; one error per requirement, on the phase. A phase with no plan is not judged (GSD runs the checker at `plan:post`) | pass | E | E | D1 |
| gsd/E302 | a plan of a phase names at least one requirement; a quick plan (no `phase`) is exempt, it names no roadmap requirement (no declarative check reads "`phase` is unset") | pass | E | E | plan template (`requirements` non-empty), D1 |
| gsd/E303 | an `auto`, `tdd` or `tracer` task has `files` and a verify of some form (`verify`, `verify_text` or `verify_human`); `action` and `done` are required fields, so the host reports those; `manual` is exempt. Severity divergence: GSD's `verify plan-structure` warns on a missing `<verify>`, `<files>` and `<done>` and errors on a missing `<action>`; the plan checker's D2 blocks all four, and that is the severity kept | pass | E | E | D2, `verify plan-structure` |
| gsd/W304 | a plan has at most 3 tasks (4 warns; the count is GSD's `task_count`: tasks plus checkpoints). Not ported: GSD's `verify plan-structure` warns "No <task> elements found" for a plan with none. Here tasks are separate entities declared after their plan, and a plan with none (a plan being drafted, the quick plan of the `plan` fixture) is not an error; E303, W309 and the judged plan check (r40) catch an empty plan that is executed | pass | W | E | D5 |
| gsd/W305 | plans of one wave share no path in `files_modified ∪ files_deleted`; under compat that wave runs sequentially (one warning per shared path, on the later plan by id; paths are the list's items, compared as written; a plan with no computed wave is not compared) | pass | W | E | `agents/gsd-planner.md:753-761`, `workflows/execute-phase.md:548-578` |
| gsd/W306 | a task's files are among its plan's `files_modified ∪ files_deleted` (one warning per item) | pass | W | E | (new: no upstream check) |
| gsd/E307 | `auto_select` (of a `decision` checkpoint that has options, as GSD) and `answer.option` name one of the checkpoint's options, option names are unique, and a checkpoint's `after` names a task of its own plan (pass); each checkpoint kind has its fields: `human_verify` `what_built` and `how_to_verify`, `decision` `decision` and `options`, `human_action` `instructions`, `action` and `verification` (seven `conditional_field_required` rules under one code; `resume_signal` is a required field) | decl + pass | E | E | checkpoint validation (`src/verify.cts:1206-1232`) |
| gsd/W308 | a `one_way` task follows a decision checkpoint of its plan (a checkpoint without `after` stands before the first task; tasks are ordered by the number in their name, so an `after` without one, or naming a task of another plan, stands before nothing), or names the decision that already settles it in `reversibility.settled_by`: GSD warns, "never error", and sees no "settled" (the model first made it an error; renamed to a warning for GSD's severity) | pass | W | E | `src/verify.cts:1397-1415` (reversibility), `planner-reversibility.md` |
| gsd/W309 | a plan (a quick plan too) has a truth and an artifact | pass | W | E | must-haves |
| gsd/E310 | every trackable decision of a planned phase's context (a `decision` scoped by the phase with `origin context`: a CONTEXT `D-NN` bullet; discretion items and deferred ideas are not decisions) is listed in the `decisions` of one of the phase's plans: a typed reference resolved in the plan's phase (ADR 0351), so another phase's namesake covers nothing and a name or a paraphrase in prose is no citation (upstream matches `\bD-NN\b` or the first six normalised words in the plans' text). `settled` plays no part (GSD reads none; it is `gsd/W311`'s input). GSD's `[informational]` and `[folded]` tags are not modelled. One error per uncited decision, on the decision. Judges a phase only once it has a plan | pass | E | E | `gate-decision-coverage-plan`, `src/decisions.cts:70`, `:525-551` |
| gsd/W311 | the plan achieves its phase goal, keeps to scope and contradicts no settled decision | judged (plan checker) | W | E | D7 and the goal and scope dimensions |
| gsd/E312 | no task action reduces scope: the planner's twelve prohibited phrases ("v1", "v2", "simplified version", "static for now", "hardcoded for now", "future enhancement", "placeholder", "basic version", "minimal implementation", "will be wired later", "dynamic in future phase", "skip for now"), case-insensitive and word-bounded (`v10` and `placeholders` do not match; `api v1` does, as the planner forbids it outright; a dotted version such as `v1.0` is not a hit). A hit is exempt only when a decision the task's plan lists in `decisions` says the same phrase in its `text`, by the rule of the hit (ADR 0351): upstream's checker cross-references each hit with the decision and asks whether the task delivers it or a reduced version; naming a decision in the action excuses nothing (upstream's own red flag, "D-26 cost references (v1, static labels)", is ALWAYS BLOCKER). Each hit is excused on its own. One error per task, naming the first phrase not excused. The checker's wider list (stub, hours, too complex) is judgment, `gsd/W311` | pass | E | E | `agents/gsd-planner.md:81-85` (no code matcher exists upstream), D7b |
| gsd/W313 | key links are wired between artifacts the plan names | judged (plan checker) | W | E | plan checker (key links) |
| gsd/E314 | each task has an automated verify or a wave-0 item (Nyquist). Judged only for a phase that has a `validation` (GSD skips D8 without one); a `manual` task is exempt; a wave-0 item satisfies it when its `for` names a requirement of the task's plan | pass | E | E | D8, `nyquist` |
| gsd/W315 | data contracts between plans match (`requires` / `provides`) | judged (plan checker) | W | E | plan checker (data contracts) |
| gsd/W316 | the plan follows the project's agent instructions and the mapped patterns | judged (plan checker) | W | W | plan checker (CLAUDE.md, patterns) |
| gsd/W317 | the post-planning gap report finds nothing open: every requirement of a planned phase is in the `requirements` of one of its plans, and every context decision (`origin context`) in the `decisions` of one (ADR 0351; upstream searches each id as a word in the plans' text). One warning per phase, on the phase, `⚠ {u} of {n} items not covered by any plan: <ids>`; the report table (`## Post-Planning Gap Analysis`, GSD's rows) is the suggestion. Sort divergence: GSD orders rows by `localeCompare` of the natural key (ICU, case-insensitive first); here it is a byte compare, so ids that differ in letter case (`REQ-a1`, `REQ-B1`) can swap, and rows with equal keys (`D-1`, `D-01`) keep id order, not CONTEXT order. The upper-case ids GSD's grammar writes sort alike. A phase with no plan is not judged (GSD runs the gate at `plan:post`) | pass | W | W | `gate-gap-analysis-plan-post`, `src/gap-checker.cts` |
| gsd/W318 | the plan-check verdict is fresh before execution (reads the judgements K7 hands the guard pass): at the gate, a plan in scope has a stale judgement; a plan that has a summary is no longer checked, so its plan-check verdicts (W311, W313, W315, W316) are skipped. Silent outside a gate | guard | W | E | plan-check loop (up to 3 rounds) |
| gsd/E319 | a prohibition with `verification test` names a test (`conditional_field_required`) | decl | E | E | the prohibition probe's LOCATE guard (a test-tier prohibition with no check target fails closed), `src/prohibition-enforcement.cts:746-756` |
| gsd/E320 | a task's verify command runs in a directory that exists (`cd` chains fold with absolute-reset, `npm --prefix`, `&amp;` decoded; the `MISSING` sentinel is skipped) and holds the manifest its tool needs (`package.json` for npm, npx, pnpm, yarn and bun; `Makefile` for make). While the task is pending, a directory a pending plan's typed planned paths (ADR 0350: its `files_modified`, its tasks' `files` and `read_first`, its artifacts' and key links' paths, one path per item) equal or cover is pending creation, not missing; once the task is done it is E320 (more lenient than GSD, which does not strip a trailing `/` of the target: `cd web/` is covered by `web/a.js` here). A dynamic or outside target is `gsd/W324`. Divergences: (1) the guest does not know the project root, so GSD's probe of a `../sibling` directory and of an absolute path inside the root is not made; both count as outside (GSD reports a missing sibling as a blocker; an open item for the kernel round that sends the root); (2) the host has no `is_dir` fact, so a directory is a path that exists and has no bytes: an unreadable regular file reads as a directory, while a withheld (`.env`) or oversized file is not one; (3) the host serves a manifest up to 64 KiB (GSD 512 KiB), once per distinct path: a path the host did not serve (a `budget` or `too_large` error, or past its 2000 facts) is not checked, never E320 or W324, so a larger `package.json` is silent where GSD warns `manifest_unreadable` | pass | E | E | `gate-verify-command-paths` |
| gsd/I321 | the deterministic half of D3b: a `coupling_justified` item naming a peer of the same phase and wave is reported as the applied exemption (`coupling between 'A' and 'B' is justified: <reason>`), and one naming a peer in another wave or phase as covering nothing. Finding the coupling itself is judgment, `gsd/W311`. Information under every profile | pass | I | I | D3b, `planner-coupling.md:25-35` |
| gsd/E322 | a plan has fewer than 5 tasks | pass | E | E | D5 (5 or more is a blocker) |
| gsd/W323 | a plan is named `<phase>-<NN>` in its phase (`02.1-01` in phase `02.1`: the phase's name, a dash, two or more digits), and by its quick id `YYMMDD-xxx` without one (three base-36 digits; a forgotten `phase` would otherwise silently make a quick plan) | pass | W | E | `phase-plan-index` (non-canonical plan names, `describeNonCanonicalPlans`) |
| gsd/W324 | a verify command is not checked, and upstream never blocks on it: its `cd` or `--prefix` directory is dynamic (`$`, a backtick, `*`, `?`, a newline, a leading `~`; `dynamic_path`) or outside the project (absolute, or climbing out; `outside_root`), or its `package.json` cannot be read as an object (`manifest_unreadable`; a file over the host's 64 KiB cap, or one it did not serve, is not checked) or has no script the `npm run` names (`script_missing`). A broken directory or manifest is `gsd/E320` instead | pass | W | W | `gate-verify-command-paths` (warning reasons) |
| gsd/E507 | each task's verify commands name their failing direction: one `fails_when` per `automated` item, not missing, empty or a whole-value placeholder (`tbd`, `todo`, `n/a`, `na`, `none`, `unknown`, `tba`, `?`, `-`, any case); the `MISSING - Wave 0 ...` sentinel is exempt, and a bare `verify_text` is not a command. One finding per item, on the task | pass | E | E | `gate-verify-failure-directions`, plan checker 8f |

## Execute

| Code | Rule | Kind | compat | strict | GSD source |
|---|---|---|---|---|---|
| gsd/E401 | a summary's `requirements_completed` equals its plan's `requirements`, as sets of resolved ids; one error on the summary names the missing and the extra ids, and a quick plan with none on either side is clean. Faithful to the executor ("MUST copy verbatim"), so a `halted` summary lists every requirement of its plan, although its derived requirement status is `blocked` (r38 divergence, `project.md`) | pass | E | E | execute-plan |
| gsd/E402 | a halted summary says why (`halted_because`) | decl | E | E | summary template |
| gsd/W403 | a `tdd` task (its own `type`, or its plan's) with a recorded `feat` or `fix` commit has one that descends from a `test` commit that names it (#5275): by the commits' recorded ancestry (`commit.descends_from`, merges included, so a RED merged from a branch counts and a RED on a sibling branch does not, whatever the clock says); each GREEN is judged by its own record, and one with no ancestry (recorded before the collector read parents) by time, as before, so a newer commit's ancestry never turns an old pair into a warning. A RED that exists but is no GREEN's ancestor is named `green_before_red`. A commit that names only the plan counts when the task is the plan's only task: GSD's executor words a commit `{type}({phase}-{plan}): {description}` and so names the plan, not the task. Divergences from upstream's `tdd.review-checkpoint`: `fix` counts as GREEN (upstream: `feat` only); it is judged per task (upstream: per plan); the RED run's failure is not checked (upstream reads a run record, and a commit is all there is here). A warning, not an error (REVIEW-r38 M1): a commit already made cannot be repaired, and an error would make the host refuse the collector's write, and with it every other commit | pass | W | W | `gate-tdd-red-evidence` |
| gsd/W404 | the TDD review checkpoint ran after execution: at the gate, a `tdd` plan in scope has a summary and no answered `tdd_review` checkpoint. The answer is recorded by a person (`answer_checkpoint` is a human-attested command, ADR 0301). Silent outside a gate | guard | W | E | `gate-tdd-review-checkpoint` |
| gsd/E405 | a phase that changed an ORM's schema files records its database push. The files are its plans' `files_modified` (trimmed) matched by `SCHEMA_PATTERNS`; the evidence is the push command in its summaries' and deviations' text or its commits' subjects (`schema-detect.cts:20-73`). GSD runs the gate at `execute:wave:post`, so only a phase with a summary is judged (REVIEW-r38 M1): the first schema commit of a phase must be recorded before its push commit or summary can exist. One error per phase names the ORMs and their push commands | pass | E | E | `gate-schema-drift` (`execute:wave:post`) |
| gsd/W406 | a research, spec or validation records the pack of `gsd/phase-context` (a quick plan's research: `gsd/quick-context`) it was written against; the host reports it when that pack moves (its `context_pack`, written by `record-research`, `record-spec` or `record-validation`). A phase with no context records the pack of its empty sections, so a context written later moves it; the project's research has no context, and a misnamed phaseless research is E206, so neither records one. Upstream also judges `PATTERNS.md`: not covered, because `pattern_map` (`@gsd/codebase`) records no pack yet and should gain one when that extension is planned | record | W | E | `gate-context-drift` |
| gsd/W407 | every task of a `complete` plan has a recorded commit that names it, one warning per task; a `manual` task is exempt, and so is every task of a halted plan (the later ones never ran). A commit names its task when its description, without a squash-merge ` (#12)` and the TDD verbs `add failing test for ` and `implement `, equals or holds exactly one task title; GSD's other wordings name the plan only. Off until `specforge.task_commits` is on (gsd-strict and gsd-plus) | pass | — | W | (new: the git collector) |
| gsd/W408 | a deliverable that is not human-judged has checks and none has status `fail` or `unknown`; one that is human-judged has a `rationale`. A check with no status (no test fact collected) is not judged: GSD's auto-pass rule (`summary.md:195`). Once a test fact is collected, a check no test ran is `unknown` | pass | W | E | summary coverage |
| gsd/W409 | the branch a phase or a milestone computes is a name git takes (`git check-ref-format --branch`, and a file name's length), with a slug when its template asks one. The field is absent otherwise, and the fences that read it stop | pass | W | E | (new: upstream hands the name to git) |
| gsd/W524 | a plan's executor pack, recorded by `start-plan`, has not moved while the plan is open: its plan, tasks, must-haves, phase, requirements, decisions, the earlier plans' summaries, research items, instructions or settings changed since the executor started, or a checkpoint was answered or a setup confirmed (each followed by `start-plan`, as the continuation does, records the pack again). Silent once the plan is complete (its `context_pack` is checked while the plan is open) | record | W | W | (new: the executor's context route) |

## Verify

| Code | Rule | Kind | compat | strict | GSD source |
|---|---|---|---|---|---|
| gsd/E501 | the verifier's verdict on each truth, artifact and key link is fresh | judged (verifier) | E | E | verifier, honest-verifier |
| gsd/E502 | an explicit truth is verified only with a passing test fact for it (one whose `[gsd:<id>]` tag names it, or one its `proof` holds, ADR 0331): a fresh passing `gsd/E501` verdict over test facts that hold no passed result, or a failed one, is E502. Silent while no test fact is collected; a `backstop` truth is never E502. Tests count that name the truth (a tag is one of a test fact's `names`), its proof's facts, and the facts the verdict's `cited_tests` names (a fact's id, or `file#name`), wherever they ran (ADR 0304): a verifier may satisfy the "no passing test" branch by citing any passing test by name, as upstream's explicit tier does, and a failing linked or cited test still raises E502. A recorded result stales the verdict first (`tests test(self.proof, self<test.names)` is an input of E501), so E502 appears when a verifier records `verified` over results that contradict it, and that record is refused (E080 names `gsd/E502`, paired by its subject: ADR 0302; the verifier records `failed` instead, `tests/cli/e502_probe.rs`). `derive` reads a fresh `verified` the tests contradict as not verified, with the same predicate: `failed` for a failing test, unknown for none passing. `verification` is a person's to write (`U`), so an agent cannot turn the guard off through SpecForge, nor through a guarded edit tool (a shell `sed` or an unguarded host is not covered) | pass | E | E | honest-verifier |
| gsd/E503 | an override names who accepted the truth and when: `by` and `at` are not required fields, so this is their one diagnostic, naming which is missing | pass | E | E | verifier overrides |
| gsd/W504 | more than three overrides in a phase: one warning on the verification | pass | W | W | verifier |
| gsd/E505 | debt markers (TBD, FIXME, XXX) in the phase's files link to an issue: a whole-word marker in a file a plan names in `files_modified`, on a line with no `#N`, `issue #N`, `PR #N` or `DEF-N`; one error per file and line, naming `file:line`. Judged over the authored `files_modified`, never a commit's file list. A file the host did not serve (a budget, a cap) is not checked, never a finding. `TODO`, `HACK` and `PLACEHOLDER` are warnings in `verification.anti_patterns`, not diagnostics | pass | E | E | verifier |
| gsd/W506 | every settled decision maps to something shipped: for a phase whose verification is `passed`, each decision of its context (`origin context`, the ones E310 counts) is listed in the `decisions` of a `complete` plan of the phase (ADR 0351); a summary or a commit subject that names it is no claim. GSD's gate reads the shipped files and the plans' text: neither is read here. Off with `workflow.context_coverage_gate` | pass | W | W | `gate-decision-coverage-verify` |
| gsd/E509 | authored UAT evidence that makes a session unpassable: a `skipped` test with no `reason`, an `issue` test with no `reported` (a test a collector reported, `source automated`, is not judged). The session's own status is computed from the predicate (pending, blocked, unresolved or undeferred skipped), so it cannot be contradicted | pass | E | E | UAT predicate |
| gsd/W510 | a planned phase whose research has a Validation Architecture section has a validation, and a verification exists only once the validation has an `approval`. A plain planned phase is not warned: `validate-phase` is opt-in (`validate-phase.md:38-43`) | pass | W | E | Nyquist 8a–8f |
| gsd/E511 | a phase ships only when its verification passed (`ship.md:59-74`, `phase.cts:4000-4001`; a gate code of `close_milestone` and `/gsd-ship`): at the gate, a phase in scope (the subject, a milestone's phases, or all) has no `passed` verification. UAT completeness is E509 and the verify-work gate, not this. Silent outside a gate | guard | E | E | phase-status ladder |
| gsd/E512 | no broken window is open at ship: at the gate, one error per open window. Silent outside a gate, so a write is never refused. Proved at `/gsd-ship`: under `gsd-strict` the prompt is refused while a window is open | guard (ship:pre) | E | E | broken-windows gate |
| gsd/E513 | a resolved window cannot be waived or fixed again: `waiver` and `fixed_at` are never both set (each is immutable, so a ledger that holds both came from two commands acting on one window) | pass | E | E | `src/broken-windows.cts` |
| gsd/W514 | a refactor proposal is tracked as an open `deviation` window while `workflow.windows_enforce` is off, so ship will not be blocked by it: one warning on the window. Gated while the setting is on (`gates_when_on`, ADR 0305). Header names `complexity-trigger.cjs` | pass | W | W | `refactor_strict_not_enforcing` |
| gsd/E515 | a complete requirement of a phase in scope traces to a task of its plans (not `manual`), a recorded commit of that task and a passing test linked to the task or the requirement, or named in the `proof` of a truth that names the requirement (ADR 0331), with no failing one: at the gate, one error per untraced requirement, on the gate's subject, naming the first missing link (`no task of its plans`, `no commit of its tasks`, `no passing test of its tasks`). A guard, not a check (a write is never refused by it); a gate code of `close_milestone`, `/gsd-complete-milestone` and `/gsd-ship`. Silent outside a gate. The setting `specforge.traceability_gate` (on in gsd-plus) gates it | guard | — | E (plus) | (new: gsd-plus) |
| gsd/E516 | a stale verdict blocks every workflow step (reads the judgements K7 hands the guard pass): at the gate, one error per entity with a stale judgement, except the plan-check verdicts of a plan whose summary has `completed`. The diagnostic names the gate's subject (a project gate, the stale entity), so a stale verdict anywhere refuses a scoped gate. Silent outside a gate | guard | — | E (plus) | (new: gsd-plus) |
| gsd/E517 | active plans of different workstreams share no path in `files_modified ∪ files_deleted` (trimmed): a plan is active until its summary says when it `completed`; one error per pair, on the later plan; a plan outside a workstream is not judged. The setting `specforge.cross_workstream_conflicts` (on in gsd-plus) gates it | pass | — | E (plus) | (new: gsd-plus) |
| gsd/E518 | the integration checker's verdict on a milestone audit is fresh | judged (integration checker) | E | E | audit-milestone integration check |
| gsd/E519 | a truth of a phase's plan proves one of its plan's requirements: one error per truth, on the truth, when it names no `requirement` or one its plan's `requirements` does not hold (ADR 0351, amending ADR 0331 D1). A quick plan's truths are not judged. The setting `specforge.truth_requirements` (on in gsd-plus) gates it; under gsd-plus a write that adds such a truth is refused (E080) | pass | — | E (plus) | (new: gsd-plus) |
| gsd/W520 | a complete plan's commits touched every path it planned, and the files it deletes are gone (ADR 0351): on a `complete` plan with at least one recorded commit (`files_touched` written), one warning on the plan naming each path of `files_modified` and of its artifacts' `path` that no commit touched (a directory path is touched by any file under it), and one naming each `files_deleted` path the host serves as still present (a path it did not serve is not judged). Plan paths and a commit's paths are both project-relative (the git collector writes them so, ADR 0352), in a project under a subdirectory of its repository too; a missing planned file is E016's (ADR 0350), never this rule's. The setting `specforge.plan_paths_touched` (on in gsd-strict and gsd-plus) gates it | pass | — | W | (new: the git collector) |
| gsd/E521 | a task commit names its task by its `Gsd-Task:` trailer (ADR 0351, amending ADR 0331 D3): one error per recorded commit, on the commit, when the gsd-git collector found its task by the subject's title (`linked_by title`), or when it names no task and its type is one of the executor's task types other than `docs` (`feat`, `fix`, `test`, `refactor`, `perf`, `style`, `chore`). A `docs` commit (a plan's metadata commit) and a commit with no type are not judged. The setting `specforge.task_trailers` (on in gsd-plus) gates it; a collector's plan is never refused for it (ADR 0303) | pass | — | E (plus) | (new: gsd-plus) |
| gsd/W522 | a code item a task or a commit names is anchored by a plan that reaches a requirement (ADR 0352 D7): for each item a `task.items` (its plan the task's) or a `commit.items_changed` (the commit's plan) names, the anchors in a quick plan dropped, when some remain and none of their plans reaches a requirement (a `requirements` entry that resolves to one, or a truth of the plan whose `requirement` resolves to one), one warning on the first anchoring task by id (else commit), naming the item and the plans. The item need not be held. On under every profile | pass | W | W | (new: the items a task names) |

## Retired and kernel-provided

- `gsd/E508` (what a verdict cites exists when it is recorded) is retired: the host already refuses it. `set_verdict`
  is refused `verdict_refused` with reason `cite` (ADR 0289 D6), and a changed cited input stales the verdict (ADR 0290
  D3). The roster maps `gate-evidence` to that refusal.
- Rules that read project files are *buildable* since protocol 1.11.0 (K11): they declare `files` inputs and are sent the facts and
  capped text of those files. `gsd/E320` is built (r37; a compute pass writes `task.verify_paths`, which the check pass
  reads; what counts as pending creation is read from the kinds' typed planned paths and the task's owner state, ADR 0350). `gsd/E505` is built (r39), with the scan pass that computes the file facts under `gsd/E501`: `artifact.state`
  (`missing`, `min_lines` as upstream counts, `contains`, `exports` by substring as upstream) and `key_link.state` (a
  plain-text `pattern`, searched in the `from` and the `to` file; one with a regular-expression metacharacter is
  `unchecked`). `gsd/E505` judges the plans that have a summary only, so writing a plan is never refused for a file it
  names. A file the host did not serve (its 1 MiB text budget is charged per entity and path) makes the phase's
  verification `scan_partial`, which keeps it from `passed`.
- Rules that read the tests in a compile pass read `@code/core`'s test facts and the graph's `names` and `proof` edges,
  never the recorded report: `gsd/W408` is built, as is the derive pass for a deliverable check's status; `gsd/E515`
  is built as a guard. `gsd/W403` reads the recorded commits.
- Rules that count list items are buildable since 1.11.0 (K13): `gsd/W103` (r36), `gsd/W305` and `gsd/W306` (r37) are built;
  `gsd/E517` is built (r41) and reads the items in `values`.
- A judge that is a prompt, not an agent (`gsd/E202`, `gsd/W203`), is built (r40): `prompt:spec_phase`, a short judging prompt that names the two rules and how to record; r42 ports GSD's spec-phase workflow into the same id.

- **Kernel behaviour the rules rely on.** The host's E080 no longer counts a judged rule's finding as an error a write adds (K17, ADR 0297), and pairs an extension's finding by its subject (ADR 0302); a collector's plan is judged for host errors only (ADR 0303). The human-only guarantee holds for every existing entity of a human-only kind and against deleting a person's value (K17, ADR 0298): an agent cannot change an `override` or delete a `milestone_audit` that carries a `debt_accepted`.

## Setting-gated rules

GSD runs these checks only while a toggle is on: the `when` of the capability's gate, or the workflow's skip. The rule
is off while the setting is false, whatever the profile; a profile only presets the setting
([settings.md](settings.md#profiles)).

| Setting | Default (gsd-compat) | gsd-strict, gsd-plus | Rules |
|---|---|---|---|
| `workflow.windows_enforce` | false | true | gsd/E512 |
| `workflow.tdd_mode` | false | false | gsd/W403, gsd/W404 |
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
| `specforge.truth_requirements` | false | false (gsd-plus: true) | gsd/E519 |
| `specforge.plan_paths_touched` | false | true | gsd/W520 |
| `specforge.task_trailers` | false | false (gsd-plus: true) | gsd/E521 |

`gsd/W514` is gated the other way (`gates_when_on`, ADR 0305): `workflow.windows_enforce` silences it while it is on, and it is reported while the setting is off.

## Judged rules' inputs

Each judged rule's named inputs (K7 D8), and the fields its verdict binds (`V`). Every verdict also digests its `cited`
and `cited_tests` lists. `file("p")` paths are from the project root; `file(S.f)` reads its field's base, `project` for
every @gsd file field (K3 D5); `test()` reads the collected test results (K7 D11). A gated rule is never evaluated, so it
is neither fresh nor stale.

| Rules | Target | Inputs | Binds |
|---|---|---|---|
| gsd/W311, W313, W315, W316 (plan check) | plan | `plan` self · `body` self<task\|checkpoint\|truth\|artifact\|key_link\|prohibition\|user_setup.plan · `phase` self.phase · `requirements` self.requirements · `decisions` self.decisions (what the plan claims to implement) · `phase_decisions` self.phase<decision.phase · `all_decisions` decision (a superset of both: the selector grammar cannot filter on an unset `phase`, so any decision change stales the plan check, the safe side, and the checker sees a contradiction with a decision the plan does not list) · `context` self.phase<context\|deferred_idea.phase · `research` self.phase<research.phase · `research_items` self.phase<research.phase<finding\|stack_choice\|pattern\|pitfall\|dont_hand_roll\|open_question.research · `siblings` self.phase<plan.phase · `instructions` file("CLAUDE.md", "AGENTS.md") | — |
| gsd/E501 | truth | `truth` self · `plan` self.plan · `phase` self.plan.phase · `tests` test(self.proof, self<test.names) · `files` file(self.plan.files_modified) | — |
| gsd/E501 | artifact | `artifact` self · `plan` self.plan · `file` file(self.path) | — |
| gsd/E501 | key_link | `link` self · `plan` self.plan · `from` file(self.from) · `to` file(self.to); not judged while `pattern` is set (`unless_set pattern`: a pass searches it). A literal pattern the scan finds (`wired`) verifies the link; a regular expression is `unchecked`, which is neither a pass nor a failure, so the phase stays `pending` until the pattern is literal | — |
| gsd/E202 | phase_spec | `spec` self · `items` self<locked_req\|interview.spec · `phase` self.phase | `scores` (its recorded values) |
| gsd/W203 | locked_req | `req` self · `traces` self.traces | — |
| gsd/W205 | package_audit | `audit` self | `legitimacy` (its outcome: legitimate \| suspicious \| unknown, passing legitimate) |
| gsd/E518 | milestone_audit | `audit` self · `phases` self.milestone<phase.milestone · `verifications` self.milestone<phase.milestone<verification.phase · `requirements` self.milestone<phase.milestone.requirements | `integration` (its outcome: passed \| issues, passing passed) |

gsd/E501 reads the files a truth's plan (`files_modified`), an artifact (`path`) and a key link (`from`, `to`) name (ADR 0304: a `file()` input may end at a string or string-list field; a missing file digests `absent`), so a code edit stales the verdict, and `covered_files` lists them. The plan checks read no file: they judge the plan before its files exist, and a code edit must not stale an in-flight check.

A summary is no plan-check input (writing it after execution stales nothing), and no verifier input either (its claims
are not evidence).

## Context routes

What each role reads of the entity it is spawned for (ADR 0356), as `specforge context gsd/<route> <id>` serves it:
the core sections are never cut and a budget below them fails; the enrich sections are served whole in order while the
budget lasts. A route bound to an agent gives the agent's installed body a `## Context` section naming it. The plan
checker's and the integration checker's sections are their judged rules' inputs, so a pack digests as the verdict's
basis. The executor reads no `files_modified`: it edits those files. `phase-context` and `quick-context` are read by
no agent; they bound the context a research, a spec or a validation is written against.

| Route | Subject | Agent | Core | Enrich | Budget |
|---|---|---|---|---|---|
| `gsd/executor` | plan | gsd-executor | `plan` self · `body` self<task\|checkpoint\|truth\|artifact\|key_link\|prohibition\|user_setup.plan · `phase` self.phase · `requirements` self.requirements · `decisions` self.decisions | `context` self.phase<context\|decision[origin=context]\|deferred_idea.phase · `quick` self<context\|decision[origin=context]\|research.plan · `prior` self.depends_on<summary.plan · `research` self.phase<research.phase<pitfall\|pattern\|dont_hand_roll.research · `project` project · `instructions` file("CLAUDE.md", "AGENTS.md"); settings `workflow.auto_advance`, `workflow.human_verify_mode`, `workflow.tdd_mode`, `git.allow_default_branch_commits` | 6000 |
| `gsd/plan-checker` | plan | gsd-plan-checker | the plan check's inputs (above) | — | 8000 |
| `gsd/integration-checker` | milestone_audit | gsd-integration-checker | gsd/E518's inputs (above) | — | 8000 |
| `gsd/planner` | phase | gsd-planner | `phase` self · `requirements` self.requirements · `context` self<context\|deferred_idea.phase · `decisions` self<decision[origin=context].phase | `research` self<research.phase · `research_items` its findings, choices, patterns, pitfalls and questions · `plans` self<plan.phase · `milestone` self.milestone · `gaps` self<verification.phase<verify_gap.verification · `uat_gaps` self<uat_session.phase<uat_test.session<uat_gap.test · `prior` self.depends_on<plan.phase<summary.plan · `project_decisions` decision[origin=project] · `project` · `instructions`; settings `workflow.tdd_mode`, `workflow.human_verify_mode` | 8000 |
| `gsd/verifier` | phase | gsd-verifier | `phase` self · `requirements` self.requirements · `must_haves` self<plan.phase<truth\|artifact\|key_link.plan · `plans` self<plan.phase · `summaries` self<plan.phase<summary.plan | `files` file(self<plan.phase.files_modified) · `context` self<context\|decision[origin=context]\|deferred_idea.phase · `pitfalls` self<research.phase<pitfall.research | 8000 |
| `gsd/phase-researcher` | phase | gsd-phase-researcher | `phase` self · `requirements` self.requirements · `context` self<context\|deferred_idea.phase · `decisions` self<decision[origin=context].phase | `milestone` self.milestone · `siblings` self.milestone<phase.milestone · `project` · `instructions` | 6000 |
| `gsd/advisor-researcher`, `gsd/assumptions-analyzer` | phase | gsd-advisor-researcher, gsd-assumptions-analyzer | as the phase researcher's | `milestone` · `siblings` | 6000 |
| `gsd/nyquist-auditor` | phase | gsd-nyquist-auditor | `phase` self · `plans` self<plan.phase · `tasks` self<plan.phase<task.plan · `validation` self<validation.phase · `rows` self<validation.phase<validation_row.validation | `requirements` self.requirements · `research` self<research.phase · `summaries` self<plan.phase<summary.plan | 8000 |
| `gsd/project-researcher` | project | gsd-project-researcher | `project` self · `requirements` requirement · `decisions` decision[origin=project] | `milestones` milestone | 8000 |
| `gsd/research-synthesizer` | project | gsd-research-synthesizer | `project` self · `research` research · `research_items` research<finding\|stack_choice\|pattern\|pitfall\|dont_hand_roll\|open_question.research | `requirements` requirement | 8000 |
| `gsd/roadmapper` | project | gsd-roadmapper | `project` self · `requirements` requirement · `decisions` decision[origin=project] | `research` research · `research_items` (as the synthesizer's) · `milestones` milestone | 8000 |
| `gsd/phase-context` | phase | — | `context` self<context.phase · `decisions` self<decision[origin=context].phase · `deferred` self<deferred_idea.phase | — | 50000 |
| `gsd/quick-context` | plan | — | `context` self<context.plan · `decisions` self<decision[origin=context].plan | — | 50000 |

## Derivations, not rules

These are computed fields, not checks; they are listed so the parity suite can compare them with GSD's own values:
phase status (`complete` once its verification is `passed`), plan wave, `autonomous`, plan and task status (blocked by halt propagation), a summary's duration, actuals and key files, a deliverable check's status, requirement status, milestone state,
verification status, score and re-verification, UAT totals and status, validation status, audit status, broken-window status and the
project's window counts, refactor proposal status, a review concern's status, a review round's open counts, a revision conflict's status, a quick batch's status and eligible items, a UAT session's follow-up count, `STATE.md` progress.

## Elsewhere

GSD gates that belong to capability extensions keep their GSD source there: the UI gates (`gate-ui-plan`,
`gate-ui-safety`) in `@gsd/ui`, API coverage (`gate-api-coverage-verify-pre`) in `@gsd/ai`, codebase drift
(`gate-codebase-drift`) in `@gsd/codebase`, open threats at `ship:pre` in `@gsd/security`. Broken windows
(`open_count == 0` at `ship:pre`, when `workflow.windows_enforce`) stays in `@gsd/core`, as gsd/E512. The remaining `gate-*` modules (`args`, `config`, `exit`, `predicate`, `predicate-evaluator`,
`evaluation-scope`, `verdict`) are GSD's gate machinery, which the kernel replaces. The harness ledger's checks (an `H`
value with no ledger line) are kernel diagnostics (K4), not `gsd/` codes.
