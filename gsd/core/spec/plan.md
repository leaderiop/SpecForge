# Plans and tasks

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
([execute.md](execute.md)). A quick task (`/gsd-quick`) is a plan with no phase, in `spec/gsd/ops/quick/`.

## plan

GSD: `NN-MM-PLAN.md` frontmatter and body. Name `03-02`, scoped by its phase (a quick plan: its quick id, no phase).

| Field | Type | Flags | GSD |
|---|---|---|---|
| phase | → phase | R I | `phase:` (absent for a quick plan) |
| type | enum{execute, tdd} | R | `type:` |
| depends_on | →* plan | | `depends_on:` |
| files_modified | files | R | `files_modified:` |
| files_deleted | strs | | `files_deleted:` |
| coupling_justified | {peer → plan R, reason str R} | | `coupling_justified:`; exempts only non-file coupling (D3b, gsd/I321), never file overlap (gsd/W305) |
| requirements | →* requirement | | `requirements:` (non-empty: not `R`, so gsd/E302 is the one diagnostic of its absence) |
| wave0 | bool | | `wave: 0`: the plan builds the phase's Nyquist wave-0 items (test stubs, fixtures, framework install) |
| gap_closure | →* verify_gap \| uat_gap | | `gap_closure: true` and `gap_ids:` |
| agent_hint | str | | `agent_hint:` |
| estimate | {tokens int} | | `estimate:` |
| objective | str | R | `<objective>` |
| purpose, output | str | | `Purpose:` / `Output:` |
| context_refs | files | | `<context>` @-refs |
| verification | strs | | `<verification>` |
| success_criteria | strs | | `<success_criteria>` |
| wave | int | C | `wave:`: longest `depends_on` path + base (0 if the phase has a `wave0` plan, else 1); file overlap never moves it (see gsd/W305). Exactly GSD's executed wave (phase-plan-index); GSD's authored `wave:` is only a hint. |
| autonomous | bool | C | `autonomous:`: false exactly when the plan has a checkpoint |
| status | enum{draft, checked, ready, executing, complete, halted, blocked} | C | checked: the plan-check verdict is fresh and passed (with `workflow.plan_check` off the plan-check rules are gated and a plan goes from draft to ready, never checked); complete / halted: from its summary; blocked: a `depends_on` chain reaches a halted plan |

## task

GSD: `<task type="auto|tdd">`. Name `T1`, scoped by its plan; the order is the name's number.

| Field | Type | Flags | GSD |
|---|---|---|---|
| plan | → plan | R I | parent |
| type | enum{auto, tdd, tracer, manual} | R | `type=` |
| files | files | | `<files>` (required for `auto`, a rule) |
| read_first | files | | `<read_first>` |
| precondition | str | | `<precondition>` |
| reversibility | {rating enum{reversible, costly, one_way} R, why str} | | `<reversibility>` |
| action | text | R | `<action>` |
| verify | {automated str R, fails_when str} | | `<verify><automated>` and its `<fails_when>`: a block list, N commands with N failing-direction statements (required for `auto`; gsd/E303, gsd/E507) |
| verify_human | str | | `<verify><human-check>`: harvested into UAT |
| acceptance | strs | | `<acceptance_criteria>` |
| done | str | R | `<done>` |
| commits | →* commit | C | the git collector links commits naming the task |
| status | enum{pending, done} | C | done when its plan's summary lists it complete |

## checkpoint

GSD: `<task type="checkpoint:*">`. Name `C1`, scoped by its plan.

| Field | Type | Flags | GSD |
|---|---|---|---|
| plan | → plan | R I | parent |
| after | → task | | position in the task order |
| kind | enum{human_verify, decision, human_action, tdd_review} | R | `type=` |
| gate | enum{blocking, blocking_human, advisory} | | `gate=` (blocking when absent) |
| what_built, how_to_verify | str | | required for `human_verify` |
| decision | str | | required for `decision` |
| options | {name str R, pros strs, cons strs} | | required for `decision`; option names unique within the checkpoint |
| auto_select | str | | `auto_select=`: must name one of `options` |
| instructions | str | | required for `human_action` |
| resume_signal | str | R | `<resume-signal>` |
| answer | {option str, text str, by str, at ts} | H | the human's reply, written by the command that resumes, with the host's clock (K14, r35) |

An option is a block, not an entity: nothing outside its checkpoint names it, so it needs no identity (K3 D2).

## must-haves

GSD: the `must_haves:` block. Each item is an entity, so a verifier's verdict can name it.

| Kind | Fields |
|---|---|
| truth | `plan R I`, `text str R` (user-observable), `verification enum{explicit, backstop}`, `criterion int` (the phase success criterion it proves). Name `TR1`. |
| artifact | `plan R I`, `path file R`, `provides str R`, `min_lines int`, `exports strs`, `contains str`. Name `AR1`. |
| key_link | `plan R I`, `from file R`, `to file R`, `via str R`, `pattern str`. Name `KL1`. |
| prohibition | `plan R I`, `statement str R`, `verification enum{test, judgment} R`, `test str` (the test it names when `verification test`, gsd/E319). Name `PR1`. |

## disposition

GSD: the plan's `## Review Dispositions Ledger`, written by `/gsd-plan-review-convergence`. Append-only. Name `DSP1`.
`plan → plan R`, `round int R`, `reviewer str R`, `feedback str R`, `outcome enum{addressed, deferred} R`, `reason str`.

## user_setup

GSD: `user_setup:` and `NN-USER-SETUP.md`. Name `US1`. `plan → plan R`, `service str R`, `why str R`,
`env_vars {name str R, source str}`, `dashboard {task str, location str, details str}`, `local_dev strs`;
`done {at ts, by str}` is `H`, written when the human confirms (`confirm_setup`).

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
