# Plans and tasks

**Status: built (r37): the nine kinds below, with their non-judged rules in [rules.md](rules.md); the plan checker's four judged rules (gsd/W311, W313, W315, W316) are built (r40), the review ledger waits for a later round; `gap_closure` is built (r39). `commits` is read back from `commit` (r38); the derive pass computes `status` there, `draft` and `checked` from the plan-check verdicts (r40).**

```
 phase ─1──*─ plan ─1──*─ task
                │   ├──*─ checkpoint         (options are a block of it)
                │   ├──*─ truth          ┐
                │   ├──*─ artifact       │ must_haves
                │   ├──*─ key_link       │
                │   ├──*─ prohibition    ┘
                │   ├──*─ disposition    (review ledger, append-only)
                │   └──*─ user_setup
                ├── depends_on →* plan   (a DAG; waves are computed from it)
                └── requirements →* requirement
```

File: `plans/<NN-MM>.spec` in the phase folder: the plan, its tasks, checkpoints and must-haves, its plan-check
verdicts, and later its summary
([execute.md](execute.md)). A quick task (`/gsd-quick`, r43) is a plan with no phase, named by its quick id; the prompt creates it with `specforge new plan`, then opens and completes it with `start-plan` and `complete-plan`.

## plan

GSD: `NN-MM-PLAN.md` frontmatter and body. Name `03-02`, scoped by its phase (a quick plan: its quick id, no phase).

| Field | Type | Flags | GSD |
|---|---|---|---|
| phase | → phase | I | `phase:`; absent for a quick plan, which is in the project scope and named by its quick id (not `R`: a quick plan has none, so a forgotten `phase` is gsd/W323, the name check) |
| type | enum{execute, tdd} | R | `type:` |
| depends_on | →* plan | | `depends_on:` |
| files_modified | strs | R | `files_modified:`: paths from the project root, not existence-checked (a plan names files it will create, so `files` would be E016 on each) |
| files_deleted | strs | | `files_deleted:` |
| coupling_justified | {peer → plan R, reason str R} | | `coupling_justified:`; exempts only non-file coupling (D3b, gsd/I321), never file overlap (gsd/W305) |
| requirements | →* requirement | | `requirements:` (non-empty for a plan of a phase, not `R`, so gsd/E302 is the one diagnostic of its absence; a quick plan names none) |
| wave0 | bool | | `wave: 0`: the plan builds the phase's Nyquist wave-0 items (test stubs, fixtures, framework install) |
| gap_closure | →* verify_gap \| uat_gap | | `gap_closure: true` and `gap_ids:` (a reference union, ADR 0285 D5) |
| agent_hint | str | | `agent_hint:` |
| estimate | {tokens int} | | `estimate:` |
| objective | str | R | `<objective>` |
| purpose, output | str | | `Purpose:` / `Output:` |
| context_refs | strs | | `<context>` @-refs (paths, not existence-checked) |
| verification | strs | | `<verification>` |
| success_criteria | strs | | `<success_criteria>` |
| wave | int | C | `wave:`: longest `depends_on` path + base (0 if the phase has a `wave0` plan, else 1); file overlap never moves it (see gsd/W305). Exactly GSD's executed wave (phase-plan-index); GSD's authored `wave:` is only a hint. |
| autonomous | bool | C | `autonomous:`: false exactly when the plan has a checkpoint |
| status | enum{draft, checked, ready, executing, complete, halted, blocked} | C | checked: all four plan-check verdicts (gsd/W311, W313, W315, W316) are fresh and passing; draft: any of them is missing, stale, unrecorded or failing (with `workflow.plan_check` off the four rules are gated, a plan has no judgement and is `ready`, never `draft` or `checked`); complete / halted: from its summary (case-folded); blocked: a `depends_on` chain inside its phase reaches a halted plan, or a cycle contains it (GSD fails closed; Kahn's algorithm, `plan-dependency-graph.cts`); executing: a recorded commit names it and it has no summary. A summary and blocking win over a commit, and execution wins over the check; before that the plan-check verdicts decide between draft, checked and (check off) ready |

## task

GSD: `<task type="auto|tdd">`. Name `T1`, scoped by its plan; the order is the name's number.

| Field | Type | Flags | GSD |
|---|---|---|---|
| plan | → plan | R I | parent |
| type | enum{auto, tdd, tracer, manual} | R | `type=` |
| files | strs | | `<files>`: paths, not existence-checked (required for `auto`, gsd/E303) |
| read_first | strs | | `<read_first>`: paths, not existence-checked |
| precondition | str | | `<precondition>` |
| reversibility | {rating enum{reversible, costly, one_way} R, why str, settled_by → decision} | | `<reversibility>`; `settled_by` names the decision (a context decision, a project one, a checkpoint's) that already settles a one-way door: GSD's "unless the decision is already settled", made checkable (gsd/W308) |
| action | text | R | `<action>` |
| verify | {automated str R, fails_when str} | | `<verify><automated>` and its `<fails_when>`: a block list, N commands with N failing-direction statements (required for `auto`; gsd/E303, gsd/E507) |
| verify_text | str | | a bare `<verify>` body (the v1.18.0 template's shape): not an `<automated>` command, so neither gsd/E320 nor gsd/E507 reads it |
| verify_human | str | | `<verify><human-check>`: harvested into UAT |
| acceptance | strs | | `<acceptance_criteria>` |
| done | str | R | `<done>` |
| verify_paths | strs | C | the directory and the manifest (`package.json`, `Makefile`) each `automated` command runs in, from its `cd` or `--prefix`, project-relative; the host reads them (K11) for gsd/E320 |
| commits | ←* commit | | read, not written: the inverse of `commit.task`, the commits whose subject names the task (`plan.commits` reads `commit.plan` the same way). The git collector records them |
| status | enum{pending, done} | C | done when a recorded commit names it or its plan's summary is complete; pending otherwise |

## checkpoint

GSD: `<task type="checkpoint:*">`. Name `C1`, scoped by its plan.

| Field | Type | Flags | GSD |
|---|---|---|---|
| plan | → plan | R I | parent |
| after | → task | | position in the task order |
| kind | enum{human_verify, decision, human_action, tdd_review} | R | `type=` |
| gate | enum{blocking, blocking_human, advisory} | | `gate=` (blocking when absent) |
| what_built, how_to_verify | str | | required for `human_verify` (gsd/E307) |
| decision | str | | required for `decision` (gsd/E307) |
| options | {name str R, pros strs, cons strs} | | required for `decision`; option names unique within the checkpoint (gsd/E307) |
| auto_select | str | | `auto_select=`: must name one of `options` (gsd/E307) |
| instructions | str | | required for `human_action` (gsd/E307) |
| action, verification | str | | required for `human_action` (`<action>`, `<verification>`; gsd/E307) |
| resume_signal | str | R | `<resume-signal>` |
| answer | {option str, text str, by str, at ts} | H | the human's reply, written by the command that resumes, with the host's clock (`now`, K14) |

An option is a block, not an entity: nothing outside its checkpoint names it, so it needs no identity (K3 D2).

## must-haves

GSD: the `must_haves:` block. Each item is an entity, so a verifier's verdict can name it.

| Kind | Fields |
|---|---|
| truth | `plan R I`, `text str R` (user-observable), `verification enum{explicit, backstop} U` (a person's to write), `criterion int` (the phase success criterion it proves). Name `TR1`. |
| artifact | `plan R I`, `path str R` (a path the plan creates: not existence-checked), `provides str R`, `min_lines int`, `exports strs`, `contains str`. Name `AR1`. |
| key_link | `plan R I`, `from str R`, `to str R` (paths the plan creates: not existence-checked), `via str R`, `pattern str`. Name `KL1`. |
| prohibition | `plan R I`, `statement str R`, `verification enum{test, judgment} R`, `test str` (the test it names when `verification test`, gsd/E319). Name `PR1`. |

## disposition

GSD: the plan's `## Review Dispositions Ledger`, written by `/gsd-plan-review-convergence`. Append-only. Name `DSP1`.
`plan → plan R`, `round int R`, `reviewer str R`, `feedback str R`, `outcome enum{addressed, deferred} R`, `reason str`.

## user_setup

GSD: `user_setup:` and `NN-USER-SETUP.md`. Name `US1`. `plan → plan R I`, `service str R`, `why str R`,
`env_vars` (a list of `{name str R, source str}`), `dashboard` (a list of `{task str, location str, details str}`, GSD's `dashboard_config`), `local_dev strs`;
`done {at ts, by str}` is `H`, written when the human confirms (`confirm_setup`). Both it and a checkpoint's `answer` are inputs of the plan check, so recording one stales the plan's W311-W316 verdicts; once the plan has a summary with `completed` that is history and the gates W318 and E516 skip it (a summary merely created does not count).

## The plan check

GSD's plan checker is a prompt with twelve dimensions. Here each dimension is a rule: the ones code can decide are
declarative rules or passes; the rest are **judged** (K7): the checker agent records a `verdict` member in the plan's file, and
the verdict goes stale when any of the plan-check inputs changes ([rules.md](rules.md#judged-rules-inputs)): the plan
and its tasks and must-haves, its phase, its requirements, the phase's and the project's decisions, the context,
the research, its sibling plans, the agent instruction files, and what the checker cited. A plan is `checked` only
with a fresh passing verdict, and `/gsd-execute-phase` refuses a phase whose plans are not. The mapping is in
[rules.md](rules.md#plan).

```spec
  verdict gsd/W311 passed {
    judge       "gsd-plan-checker@0.1.0"
    model       "claude-opus-5-5"
    at          "2026-10-26T14:02:11Z"
    cited       [AUTH-07, "src/http/middleware.ts"]
    cited_tests []
    inputs      { plan "sha256:9f1c…" body "sha256:41d7…" phase "sha256:04ab…" requirements "sha256:5e21…"
                  decisions "sha256:c3d0…" all_decisions "sha256:e0b9…" context "sha256:7a4f…"
                  research "sha256:19cd…" research_items "sha256:b6a2…" siblings "sha256:2f80…"
                  instructions "sha256:77e0…" cited "sha256:a8f3…" }
    findings    []
  }
```
