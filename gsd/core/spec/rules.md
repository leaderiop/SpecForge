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
| gsd/E310 | every trackable decision of a planned phase's context (a `decision` scoped by the phase with `origin context`: a CONTEXT `D-NN` bullet; discretion items and deferred ideas are not decisions) is cited by one of the phase's plans: `\bD-NN\b`, or the first six normalised words of its `text` (when it has six), in the plan's `objective`, `purpose` or `output`, or in any field of its tasks, checkpoints, truths, artifacts, key links or prohibitions. `settled` plays no part (GSD reads none; it is `gsd/W311`'s input). An entity's title is not scanned (the pass input carries none): a decision cited only in a title is a false E310, cite it in a field. GSD's `[informational]` and `[folded]` tags are not modelled. One error per uncited decision, on the decision. Judges a phase only once it has a plan | pass | E | E | `gate-decision-coverage-plan`, `src/decisions.cts:70`, `:525-551` |
| gsd/W311 | the plan achieves its phase goal, keeps to scope and contradicts no settled decision | judged (plan checker) | W | E | D7 and the goal and scope dimensions |
| gsd/E312 | no task action reduces scope: the planner's twelve prohibited phrases ("v1", "v2", "simplified version", "static for now", "hardcoded for now", "future enhancement", "placeholder", "basic version", "minimal implementation", "will be wired later", "dynamic in future phase", "skip for now"), case-insensitive and word-bounded (`v10` and `placeholders` do not match; `api v1` does, as the planner forbids it outright; a dotted version such as `v1.0` is not a hit). A hit is exempt when the action cites a decision of the plan's phase (`\bD-NN\b`, as E310 matches): upstream's checker cross-references each hit with the decision before it blocks. One error per task, naming the first phrase found. The checker's wider list (stub, hours, too complex) is judgment, `gsd/W311` | pass | E | E | `agents/gsd-planner.md:81-85` (no code matcher exists upstream), D7b |
| gsd/W313 | key links are wired between artifacts the plan names | judged (plan checker) | W | E | plan checker (key links) |
| gsd/E314 | each task has an automated verify or a wave-0 item (Nyquist). Judged only for a phase that has a `validation` (GSD skips D8 without one); a `manual` task is exempt; a wave-0 item satisfies it when its `for` names a requirement of the task's plan | pass | E | E | D8, `nyquist` |
| gsd/W315 | data contracts between plans match (`requires` / `provides`) | judged (plan checker) | W | E | plan checker (data contracts) |
| gsd/W316 | the plan follows the project's agent instructions and the mapped patterns | judged (plan checker) | W | W | plan checker (CLAUDE.md, patterns) |
| gsd/W317 | the post-planning gap report finds nothing open: every requirement of a planned phase and every `D-NN` of its context (trackable or not) appears as a word in the text of the phase's plans (plans, tasks, checkpoints and must-haves; a plan's `requirements` items count). One warning per phase, on the phase, `⚠ {u} of {n} items not covered by any plan: <ids>`; the report table (`## Post-Planning Gap Analysis`, GSD's rows) is the suggestion. Sort divergence: GSD orders rows by `localeCompare` of the natural key (ICU, case-insensitive first); here it is a byte compare, so ids that differ in letter case (`REQ-a1`, `REQ-B1`) can swap, and rows with equal keys (`D-1`, `D-01`) keep id order, not CONTEXT order. The upper-case ids GSD's grammar writes sort alike. A phase with no plan is not judged (GSD runs the gate at `plan:post`) | pass | W | W | `gate-gap-analysis-plan-post`, `src/gap-checker.cts` |
| gsd/W318 | the plan-check verdict is fresh before execution (reads the judgements K7 hands the guard pass): at the gate, a plan in scope has a stale judgement; a plan that has a summary is no longer checked, so its plan-check verdicts (W311, W313, W315, W316) are skipped. Silent outside a gate | guard | W | E | plan-check loop (up to 3 rounds) |
| gsd/E319 | a prohibition with `verification test` names a test (`conditional_field_required`) | decl | E | E | the prohibition probe's LOCATE guard (a test-tier prohibition with no check target fails closed), `src/prohibition-enforcement.cts:746-756` |
| gsd/E320 | a task's verify command runs in a directory that exists (`cd` chains fold with absolute-reset, `npm --prefix`, `&amp;` decoded; the `MISSING` sentinel is skipped) and holds the manifest its tool needs (`package.json` for npm, npx, pnpm, yarn and bun; `Makefile` for make). A directory the plan's own task files (each item split on commas and white space, as GSD splits `<files>`) or artifact paths equal or cover is pending creation, not missing (more lenient than GSD, which does not strip a trailing `/` of the target: `cd web/` is covered by `web/a.js` here). A dynamic or outside target is `gsd/W324`. Divergences: (1) the guest does not know the project root, so GSD's probe of a `../sibling` directory and of an absolute path inside the root is not made; both count as outside (GSD reports a missing sibling as a blocker; an open item for the kernel round that sends the root); (2) the host has no `is_dir` fact, so a directory is a path that exists and has no bytes: an unreadable regular file reads as a directory, while a withheld (`.env`) or oversized file is not one; (3) the host serves a manifest up to 64 KiB (GSD 512 KiB), once per distinct path: a path the host did not serve (a `budget` or `too_large` error, or past its 2000 facts) is not checked, never E320 or W324, so a larger `package.json` is silent where GSD warns `manifest_unreadable` | pass | E | E | `gate-verify-command-paths` |
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
| gsd/W406 | the context the plan was written against has not drifted (the `context_digest` a research, spec or validation recorded differs from the context's current digest; a quick plan's research is judged against its quick plan's context (both name it in `plan`); the project's research has no context, and a misnamed phaseless research is E206, so neither is judged. Upstream also judges `PATTERNS.md`: not covered, because `pattern_map` (`@gsd/codebase`) records no `context_digest` yet and should gain one when that extension is planned) | pass | W | E | `gate-context-drift` |
| gsd/W407 | every task of a `complete` plan has a recorded commit that names it, one warning per task; a `manual` task is exempt, and so is every task of a halted plan (the later ones never ran). A commit names its task when its description, without a squash-merge ` (#12)` and the TDD verbs `add failing test for ` and `implement `, equals or holds exactly one task title; GSD's other wordings name the plan only. Off until `specforge.task_commits` is on (gsd-strict and gsd-plus) | pass | — | W | (new: the git collector) |
| gsd/W408 | a deliverable that is not human-judged has checks and none has status `fail` or `unknown`; one that is human-judged has a `rationale`. A check with no status (no test report) is not judged: GSD's auto-pass rule (`summary.md:195`). Once a test runner has recorded a result (`gsd-junit`; the report `gsd-git` writes holds none), a check no test ran is `unknown` | pass | W | E | summary coverage |

## Verify

| Code | Rule | Kind | compat | strict | GSD source |
|---|---|---|---|---|---|
| gsd/E501 | the verifier's verdict on each truth, artifact and key link is fresh | judged (verifier) | E | E | verifier, honest-verifier |
| gsd/E502 | an explicit truth is verified only with a passing test recorded for it (its `[gsd:<id>]` link): a fresh passing `gsd/E501` verdict over recorded tests that hold no passed result, or a failed one, is E502. Silent without a report (I205 says there is none); a `backstop` truth is never E502. Tests count that are linked to the truth and those the verdict's `cited_tests` names, anywhere in the report (ADR 0304): a verifier may satisfy the "no passing test" branch by citing any passing test by name, as upstream's explicit tier does, and a failing linked or cited test still raises E502. A recorded result stales the verdict first (`tests test(self)` is an input of E501), so E502 appears when a verifier records `verified` over results that contradict it, and that record is refused (E080 names `gsd/E502`, paired by its subject: ADR 0302; the verifier records `failed` instead, `tests/cli/e502_probe.rs`). `derive` reads a fresh `verified` the tests contradict as not verified, with the same predicate: `failed` for a failing test, unknown for none passing. `verification` is a person's to write (`U`), so an agent cannot turn the guard off through SpecForge, nor through a guarded edit tool (a shell `sed` or an unguarded host is not covered) | pass | E | E | honest-verifier |
| gsd/E503 | an override names who accepted the truth and when: `by` and `at` are not required fields, so this is their one diagnostic, naming which is missing | pass | E | E | verifier overrides |
| gsd/W504 | more than three overrides in a phase: one warning on the verification | pass | W | W | verifier |
| gsd/E505 | debt markers (TBD, FIXME, XXX) in the phase's files link to an issue: a whole-word marker in a file a plan names in `files_modified`, on a line with no `#N`, `issue #N`, `PR #N` or `DEF-N`; one error per file and line, naming `file:line`. Judged over the authored `files_modified`, never a commit's file list. A file the host did not serve (a budget, a cap) is not checked, never a finding. `TODO`, `HACK` and `PLACEHOLDER` are warnings in `verification.anti_patterns`, not diagnostics | pass | E | E | verifier |
| gsd/W506 | every settled decision maps to something shipped: for a phase whose verification is `passed`, each decision of its context (`origin context`, the ones E310 counts) is mentioned by one of the phase's summaries (their text) or by the subject of one of its commits, by E310's matcher (its id as a word, or its first six words). GSD's gate also reads the shipped files: not read here. Off with `workflow.context_coverage_gate` | pass | W | W | `gate-decision-coverage-verify` |
| gsd/E509 | authored UAT evidence that makes a session unpassable: a `skipped` test with no `reason`, an `issue` test with no `reported` (a test a collector reported, `source automated`, is not judged). The session's own status is computed from the predicate (pending, blocked, unresolved or undeferred skipped), so it cannot be contradicted | pass | E | E | UAT predicate |
| gsd/W510 | a planned phase whose research has a Validation Architecture section has a validation, and a verification exists only once the validation has an `approval`. A plain planned phase is not warned: `validate-phase` is opt-in (`validate-phase.md:38-43`) | pass | W | E | Nyquist 8a–8f |
| gsd/E511 | a phase ships only when its verification passed (`ship.md:59-74`, `phase.cts:4000-4001`; a gate code of `close_milestone` and `/gsd-ship`): at the gate, a phase in scope (the subject, a milestone's phases, or all) has no `passed` verification. UAT completeness is E509 and the verify-work gate, not this. Silent outside a gate | guard | E | E | phase-status ladder |
| gsd/E512 | no broken window is open at ship: at the gate, one error per open window. Silent outside a gate, so a write is never refused. Proved at `/gsd-ship`: under `gsd-strict` the prompt is refused while a window is open | guard (ship:pre) | E | E | broken-windows gate |
| gsd/E513 | a resolved window cannot be waived or fixed again: `waiver` and `fixed_at` are never both set (each is immutable, so a ledger that holds both came from two commands acting on one window) | pass | E | E | `src/broken-windows.cts` |
| gsd/W514 | a refactor proposal is tracked as an open `deviation` window while `workflow.windows_enforce` is off, so ship will not be blocked by it: one warning on the window. Gated while the setting is on (`gates_when_on`, ADR 0305). Header names `complexity-trigger.cjs` | pass | W | W | `refactor_strict_not_enforcing` |
| gsd/E515 | a complete requirement of a phase in scope traces to a task of its plans (not `manual`), a recorded commit of that task and a passing test linked to the task or the requirement, with no failing one: at the gate, one error per untraced requirement, on the gate's subject, naming the first missing link (`no task of its plans`, `no commit of its tasks`, `no passing test of its tasks`). A guard, not a check (a write is never refused by it); a gate code of `close_milestone`, `/gsd-complete-milestone` and `/gsd-ship`. Silent outside a gate. The setting `specforge.traceability_gate` (on in gsd-plus) gates it | guard | — | E (plus) | (new: gsd-plus) |
| gsd/E516 | a stale verdict blocks every workflow step (reads the judgements K7 hands the guard pass): at the gate, one error per entity with a stale judgement, except the plan-check verdicts of a plan whose summary has `completed`. The diagnostic names the gate's subject (a project gate, the stale entity), so a stale verdict anywhere refuses a scoped gate. Silent outside a gate | guard | — | E (plus) | (new: gsd-plus) |
| gsd/E517 | active plans of different workstreams share no path in `files_modified ∪ files_deleted` (trimmed): a plan is active until its summary says when it `completed`; one error per pair, on the later plan; a plan outside a workstream is not judged. The setting `specforge.cross_workstream_conflicts` (on in gsd-plus) gates it | pass | — | E (plus) | (new: gsd-plus) |
| gsd/E518 | the integration checker's verdict on a milestone audit is fresh | judged (integration checker) | E | E | audit-milestone integration check |

## Retired and kernel-provided

- `gsd/E508` (what a verdict cites exists when it is recorded) is retired: the host already refuses it. `set_verdict`
  is refused `verdict_refused` with reason `cite` (ADR 0289 D6), and a changed cited input stales the verdict (ADR 0290
  D3). The roster maps `gate-evidence` to that refusal.
- Rules that read project files are *buildable* since protocol 1.11.0 (K11): they declare `reads` and are sent the facts and
  capped text of those files. `gsd/E320` is built (r37; a compute pass writes `task.verify_paths`, which the check pass
  reads). `gsd/E505` is built (r39), with the scan pass that computes the file facts under `gsd/E501`: `artifact.state`
  (`missing`, `min_lines` as upstream counts, `contains`, `exports` by substring as upstream) and `key_link.state` (a
  plain-text `pattern`, searched in the `from` and the `to` file; one with a regular-expression metacharacter is
  `unchecked`). `gsd/E505` judges the plans that have a summary only, so writing a plan is never refused for a file it
  names. A file the host did not serve (its 1 MiB text budget is charged per entity and path) makes the phase's
  verification `scan_partial`, which keeps it from `passed`.
- Rules that read the test report in a compile pass are buildable since 1.11.0 (K12), by declaring `reads_tests`: `gsd/W408`
  (r38) is built, as is the derive pass for a deliverable check's status; `gsd/E515` is built (K20a) as a guard.
  `gsd/W403` reads the recorded commits, not the report.
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

`gsd/W514` is gated the other way (`gates_when_on`, ADR 0305): `workflow.windows_enforce` silences it while it is on, and it is reported while the setting is off.

## Judged rules' inputs

Each judged rule's named inputs (K7 D8), and the fields its verdict binds (`V`). Every verdict also digests its `cited`
and `cited_tests` lists. `file("p")` paths are from the project root; `file(S.f)` reads its field's base, `project` for
every @gsd file field (K3 D5); `test()` reads the collected test results (K7 D11). A gated rule is never evaluated, so it
is neither fresh nor stale.

| Rules | Target | Inputs | Binds |
|---|---|---|---|
| gsd/W311, W313, W315, W316 (plan check) | plan | `plan` self · `body` self<task\|checkpoint\|truth\|artifact\|key_link\|prohibition\|user_setup.plan · `phase` self.phase · `requirements` self.requirements · `decisions` self.phase<decision.phase · `all_decisions` decision (a superset of `decisions`: the selector grammar cannot filter on an unset `phase`, so any decision change stales the plan check, the safe side) · `context` self.phase<context\|deferred_idea.phase · `research` self.phase<research.phase · `research_items` self.phase<research.phase<finding\|stack_choice\|pattern\|pitfall\|dont_hand_roll\|open_question.research · `siblings` self.phase<plan.phase · `instructions` file("CLAUDE.md", "AGENTS.md") | — |
| gsd/E501 | truth | `truth` self · `plan` self.plan · `phase` self.plan.phase · `tests` test(self) · `files` file(self.plan.files_modified) | — |
| gsd/E501 | artifact | `artifact` self · `plan` self.plan · `file` file(self.path) | — |
| gsd/E501 | key_link | `link` self · `plan` self.plan · `from` file(self.from) · `to` file(self.to); not judged while `pattern` is set (`unless_set pattern`: a pass searches it). A literal pattern the scan finds (`wired`) verifies the link; a regular expression is `unchecked`, which is neither a pass nor a failure, so the phase stays `pending` until the pattern is literal | — |
| gsd/E202 | phase_spec | `spec` self · `items` self<locked_req\|interview.spec · `phase` self.phase | `scores` (its recorded values) |
| gsd/W203 | locked_req | `req` self · `traces` self.traces | — |
| gsd/W205 | package_audit | `audit` self | `legitimacy` (its outcome: legitimate \| suspicious \| unknown, passing legitimate) |
| gsd/E518 | milestone_audit | `audit` self · `phases` self.milestone<phase.milestone · `verifications` self.milestone<phase.milestone<verification.phase · `requirements` self.milestone<phase.milestone.requirements | `integration` (its outcome: passed \| issues, passing passed) |

gsd/E501 reads the files a truth's plan (`files_modified`), an artifact (`path`) and a key link (`from`, `to`) name (ADR 0304: a `file()` input may end at a string or string-list field; a missing file digests `absent`), so a code edit stales the verdict, and `covered_files` lists them. The plan checks read no file: they judge the plan before its files exist, and a code edit must not stale an in-flight check.

A summary is no plan-check input (writing it after execution stales nothing), and no verifier input either (its claims
are not evidence).

## Derivations, not rules

These are computed fields, not checks; they are listed so the parity suite can compare them with GSD's own values:
phase status (`complete` once its verification is `passed`), plan wave, `autonomous`, plan and task status (blocked by halt propagation), a summary's duration, actuals and key files, a deliverable check's status, requirement status, milestone state,
verification status, score and re-verification, UAT totals and status, validation status, audit status, broken-window status and the
project's window counts, refactor proposal status, `STATE.md` progress.

## Elsewhere

GSD gates that belong to capability extensions keep their GSD source there: the UI gates (`gate-ui-plan`,
`gate-ui-safety`) in `@gsd/ui`, API coverage (`gate-api-coverage-verify-pre`) in `@gsd/ai`, codebase drift
(`gate-codebase-drift`) in `@gsd/codebase`, open threats at `ship:pre` in `@gsd/security`. Broken windows
(`open_count == 0` at `ship:pre`, when `workflow.windows_enforce`) stays in `@gsd/core`, as gsd/E512. The remaining `gate-*` modules (`args`, `config`, `exit`, `predicate`, `predicate-evaluator`,
`evaluation-scope`, `verdict`) are GSD's gate machinery, which the kernel replaces. The harness ledger's checks (an `H`
value with no ledger line) are kernel diagnostics (K4), not `gsd/` codes.
