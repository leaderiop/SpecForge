# `@gsd/core` model

**Status: the foundation is built (the eight kinds of [project.md](project.md), the code book, the settings and the roadmap checks) and so are the sixteen kinds of [discuss.md](discuss.md) with the rules built so far (`gsd/E201`, `gsd/W204`, `gsd/E206`, `gsd/W406`; `gsd/E202`, `gsd/W203` and `gsd/W205` over them wait for r40) the nine kinds of [plan.md](plan.md) with their non-judged rules (r37) and the seven kinds of [execute.md](execute.md) with `gsd/E401` to `gsd/E405` (E403 is `gsd/W403`), `gsd/W407` and `gsd/W408` and the two collectors (r38); the fourteen kinds of [verify.md](verify.md) with their computed values, the scan pass and the rules and gate guards listed in [rules.md](rules.md) (r39; `gsd/W514` and E502 wait; E501 is judged by the verifier (r40)); the rest is being designed. Tracks gsd-core 1.18.0.** The kinds, edges, settings and rules `@gsd/core` declares. Every GSD
file of `.planning/` maps to kinds here or in a capability extension ([the file table](#every-gsd-file-maps-to-a-kind));
nothing is kept as Markdown.

| File | Kinds |
|---|---|
| [project.md](project.md) | project, requirement, milestone, phase, workstream, retrospective, lesson, methodology |
| [discuss.md](discuss.md) | context, decision, deferred_idea, phase_spec, locked_req, interview, discussion, research and its items |
| [plan.md](plan.md) | plan, task, checkpoint, truth, artifact, key_link, prohibition, disposition, user_setup |
| [execute.md](execute.md) | summary, deliverable, deviation, commit, handoff, pause, blocker; the session state |
| [verify.md](verify.md) | verification, verify_gap, override, human_item, uat_session, uat_test, uat_gap, validation, validation_row, wave0_item, manual_check, milestone_audit, broken_window, refactor_proposal |
| [settings.md](settings.md) | the settings and profiles that replace `config.json` |
| [rules.md](rules.md) | every GSD check as a rule, with its code, kind and profile |

## Notation

```
 str     quoted string            text   triple-quoted prose          int     integer        int{1..4}  bounded
 bool    true/false               date   YYYY-MM-DD                   ts      RFC 3339 UTC string (a `str` that `matches`, gsd/E107)
 enum{}  one closed value         enums{} list of closed values       id      bare identifier
 strs    list of strings          file   path, existence-checked      files   list of paths
 → K     reference to kind K      →* K   list of references           → A|B   reference to a kind union
 {..}    repeatable typed block, its own fields inside
```

`enums{}`, `{..}`, `→ A|B` and `int{a..b}` are kernel field types proposed in K3.

## Who writes a field

| Flag | Meaning | Kernel |
|---|---|---|
| (none) | authored by a human or an agent, through an edit plan | today |
| `R` | required | today |
| `I` | immutable, write-once: once the git baseline (HEAD, or the merge-base under `check --since`) holds a value, changing or removing it is an error; references compare by target, so a rename is no change | K4 |
| `+` | append-only list: the baseline list is a prefix of the new one, items compared by value. On a kind (deviation, disposition, interview): no entity of the baseline is removed, and their fields are immutable | K4 |
| `C` | **computed**: derived by a compute pass (a collector files a report the pass reads), held in the compiled graph only, never in `.spec`; writing it is an error. A value that compares with the past (`re_verification`) compares with the git baseline | K4 |
| `H` | set only by a command's or a collector's plan; its digest goes in `specforge-harness`; any other value is an error | K4 |
| `V` | its value comes from the fresh, recorded verdict of the judged rule that binds it (its outcome or a value it records); absent otherwise; written by hand it is an error (K4 E082) | K7 |
| `U` | human-only: refused in every extension's plan and every MCP write when it is created, deleted or retitled (editing an existing one is not yet checked, K17); the CLI writes it only from an interactive terminal; hand edits stay free (a field the host must also refuse an agent's direct edit of is `H`, written by a command that takes a reason) | K5 |

The rule this table carries: an agent may author a plan, but it may not author `status passed`.

Rename or renumber an entity that holds an `H` value through SpecForge (`/gsd-phase`, `specforge.rename`, the editor's
rename): its ledger lines follow, and a move record (`<new> moved:<old>`) lets the `I` and `+` checks follow it to the baseline. A rename by hand leaves its `H` values unrecorded and its history unmatched.

## Identity

Entity names are GSD's own identifiers (K1), unique within their scope (K2). A kind declares which reference field
scopes it; the field is optional, and an entity without it is in the project scope (a project decision, a quick plan).
A scope is never its file's path. A reference resolves along its own scope chain (itself, its scope, each enclosing
scope, the project); anything else is written as a path (`03/D-01`), and a leading `/` names from the project
(`/D-01`, the project decision beside a phase's `D-01`). SpecForge writes a scope field as its full path
(`phase v1.0/03`), so renumbering phases per milestone never makes it ambiguous; a hand-written `phase 03` works
while it is unique.

| Kind | Example | Unique within | From outside |
|---|---|---|---|
| requirement | `AUTH-01` | project | `AUTH-01` |
| milestone | `v1.0` | project (workstream) | `v1.0` |
| phase | `03`, `03.1`, `999.1` | milestone | `v1.0/03` |
| plan | `03-02` | phase | `03/03-02` |
| task, checkpoint, truth, artifact, key_link | `T1`, `C1`, `TR1`, `AR1`, `KL1` | plan | `03-02/T1` |
| decision | `D-01` | phase (project-level decisions: project) | `03/D-01` |
| verify_gap | `VG1` | verification | `03/03-VERIFICATION/VG1` |
| uat_test | `U1` | uat_session | `03/03-UAT/U1` |
| uat_gap | `G-03-1` | uat_test | `03/03-UAT/U1/G-03-1` |
| deliverable | `D1` | summary | `03-02-SUMMARY/D1` |
| broken_window | `BW1` | project | `BW1` |
| refactor_proposal | `RP1` | phase | `03/RP1` |

### Names of every kind

Every entity needs a name (K1, K2), and a kind with one entity per parent cannot be named after its keyword (E013). It
takes GSD's own file stem; every other kind takes a letter prefix and a number, in order.

| Name | Kinds |
|---|---|
| any word | project (by convention the slug of its name) |
| the GSD id | requirement `AUTH-01`, milestone `v1.0`, phase `03`, workstream `auth-rework`, plan `03-02` (a quick plan: its quick id, `260321-a1b`), decision `D-01`, uat_gap `G-03-1` |
| the GSD file stem | context `03-CONTEXT`, phase_spec `03-SPEC`, discussion `03-DISCUSSION-LOG`, research `03-RESEARCH` (project research: `RESEARCH`), summary `03-02-SUMMARY`, verification `03-VERIFICATION`, uat_session `03-UAT`, validation `03-VALIDATION`, retrospective `v1.0-RETROSPECTIVE` |
| a letter prefix and a number | task `T1`, checkpoint `C1`, truth `TR1`, artifact `AR1`, key_link `KL1`, prohibition `PR1`, disposition `DSP1`, user_setup `US1`, deliverable `D1`, deviation `DV1`, pause `PS1`, handoff `HO1`, blocker `BL1`, deferred_idea `DI1`, locked_req `LR1`, interview `IV1`, finding `F1`, stack_choice `SC1`, pattern `P1`, pitfall `PF1`, dont_hand_roll `DH1`, open_question `OQ1`, source `SRC1`, package_audit `PA1`, verify_gap `VG1`, override `OV1`, human_item `HI1`, uat_test `U1`, validation_row `VR1`, wave0_item `W0-1`, manual_check `MC1`, milestone_audit `A1`, broken_window `BW1`, refactor_proposal `RP1`, lesson `L1`, methodology `M1` |
| the first 12 hex digits of the hash | commit |

Each prefix-numbered kind is scoped (`scoped_by`) by its parent reference: plan for task, checkpoint, truth, artifact,
key_link, prohibition, disposition, user_setup and summary; summary for deliverable and deviation; phase for decision,
deferred_idea, the one-per-phase kinds, refactor_proposal and (optionally) pause and handoff; research for its items;
phase_spec for locked_req and interview; verification for verify_gap, override and human_item; uat_session for
uat_test; uat_test for uat_gap; validation for validation_row, wave0_item and manual_check; retrospective for lesson;
milestone for milestone_audit and retrospective. A blocker, a broken_window, a methodology and a commit are in the
project scope. A one-per-parent kind's name is unique within its parent, so `03-CONTEXT` of phase `03` and of phase `04` coexist
as `v1.0/03/03-CONTEXT`; a kind without an entry above is named by the same rule.
A workstream is a scope too: `auth-rework/v1.0/03` names phase 03 of milestone v1.0 of the workstream `auth-rework`.
A prompt or command argument that names an entity is resolved project-wide, before anything runs and with no session
state (ADR 0291 D1): `/gsd-plan-phase 03.1` works while `03.1` is unique in the project, and any unique suffix works
the same way. An ambiguous short form is refused, naming each candidate, and the agent passes the full path
(`v1.0/03.1`). The session's active milestone or workstream never picks one, so an argument means the same entity on
every machine.

## What it looks like

`spec/gsd/milestones/v1.0/phases/03-auth/plans/03-02.spec`:

```spec
plan 03-02 "Session tokens" {
  phase        03
  type         execute
  depends_on   [03-01]
  requirements [AUTH-02, AUTH-03]
  files_modified ["src/auth/session.ts", "src/auth/session.test.ts"]
  objective    "Issue and rotate session tokens."
}

truth TR1 "A signed-in user stays signed in after a reload" {
  plan         03-02
  text         "A signed-in user stays signed in after a reload"
  verification explicit
}

artifact AR1 "Session module" {
  plan     03-02
  path     "src/auth/session.ts"
  provides "token issue and rotation"
}

key_link KL1 "Middleware reads the session" {
  plan 03-02
  from "src/auth/session.ts"
  to   "src/http/middleware.ts"
  via  "requireSession"
}

task T1 "Issue a token on sign-in" {
  plan     03-02
  type     auto
  files    ["src/auth/session.ts"]
  action   """
    Issue a signed token on sign-in; store its id in the session table.
  """
  verify   [{ automated "npm test -- session" fails_when "no token is returned or the middleware rejects it" }]
  done     "sign-in returns a token the middleware accepts"
}
```

Nothing in it is computed: `wave`, `autonomous` and `status` are derived (`specforge query 03-02` shows them), and
the plan's summary, written by the executor at the end, lands in the same file.

## Every GSD file maps to a kind

| GSD file | Kinds | Extension |
|---|---|---|
| `PROJECT.md` | project, requirement, decision | core |
| `REQUIREMENTS.md` | requirement | core |
| `ROADMAP.md`, `MILESTONES.md`, `milestones/*` | milestone, phase | core |
| `STATE.md` | computed (progress, position) + pause, blocker + session state | core |
| `config.json` | settings | core |
| `RETROSPECTIVE.md`, `LEARNINGS.md`, `METHODOLOGY.md` | retrospective, lesson, methodology | core |
| `NN-CONTEXT.md`, `NN-DISCUSSION-LOG.md` | context, decision, deferred_idea, discussion | core |
| `NN-SPEC.md` | phase_spec, locked_req, interview | core |
| `NN-RESEARCH.md`, `research/*.md` | research and its items | core |
| `NN-MM-PLAN.md` | plan, task, checkpoint, truth, artifact, key_link, prohibition, disposition, user_setup | core |
| `NN-MM-SUMMARY.md` | summary, deliverable, deviation, decision | core |
| `.continue-here.md` | handoff | core |
| `NN-VERIFICATION.md` | verification, verify_gap, override, human_item, and verdicts | core |
| `NN-UAT.md` | uat_session, uat_test, uat_gap | core |
| `NN-VALIDATION.md` | validation, validation_row, wave0_item, manual_check | core |
| `v<version>-MILESTONE-AUDIT.md` | milestone_audit | core |
| git history (no GSD file) | commit (written by the git collector, never by an agent); a JUnit report is read by the `gsd-junit` collector, which records tests, not entities | core |
| `WINDOWS.md` | broken_window | core |
| `NN-REFACTOR.md` | refactor_proposal | core |
| `quick/*` | plan, summary (a quick plan has no phase) | core |
| `workstreams/*` | workstream (a scope) | core |
| `SECURITY.md`, threat model | threat, trust_boundary, accepted_risk, audit_run | security |
| `AI-SPEC.md`, `NN-EVAL-REVIEW.md` | ai_spec, ai_failure, eval_dimension, guardrail, eval_review | ai |
| `NN-UI-SPEC.md`, `NN-UI-REVIEW.md` | ui_spec, tokens, components, ui_review, findings | ui |
| `codebase/*`, `intel/*`, `NN-PATTERNS.md` | codebase kinds, pattern_map | codebase |
| `debug/*`, `NN-REVIEW.md`, `todos/*`, `seeds/*`, `threads/*`, `BACKLOG.md`, `USER-PROFILE.md` | ops kinds | ops |

Not modelled, on purpose: GSD's `references/*` and `workflows/*` are GSD's own rules and procedures, so they become
[rules](rules.md) and ported prompts ([contributions](../contributions/)), not data. Sketch and spike HTML stays
throwaway output outside the spec tree.

## Waiting on the kernel

Three things r39 could not build, or could not guarantee, wait for kernel work (K17):

- **Baseline re-verification (T13).** `re_verification.previous_status` and `previous_score` are not built, and the
  baseline view of an artifact or key link is not used. The git adapter loads only `*.spec` and `specforge-harness` at a
  commit, so at the baseline every source file reads as absent (`exists: false`): the scan says `missing`, never
  `unchecked`, and the baseline's status and score are wrong. `re_verification` therefore compares fresh verdicts and
  verify gaps only. The baseline compile must serve source files from the commit through the on-demand blob fetch
  (with `baseline-format` 3). Its `regressions`, a verdict-based `gaps_closed` and a reopened gap are tested on
  synthetic baselines, not black-box (the judgement list is empty on a baseline).
- **`gsd/W514`** is not built: a pass is sent no settings, and a setting gates a code only while it is on. It needs
  `SettingDescriptor.gates_when_on`.
- **Human-only on existing entities.** The guarantee of `override` (and `debt_accepted`) holds for creating, deleting
  and retitling only: the host's `edit_flags` checks the kind's writer on those, not on `set`, `unset` or `link` of an
  existing entity, so an agent can change or re-point an existing override. The black-box test
  `an_agent_cannot_edit_an_existing_override` is ignored until the kernel fix. An agent can also delete a milestone
  audit that carries a person's `debt_accepted` (gsd/E106 then refuses the gate).

