# `@gsd/core` model

**Status: being designed; nothing is built yet. Tracks gsd-core 1.18.0.** The kinds, edges, settings and rules `@gsd/core` declares. Every GSD
file of `.planning/` maps to kinds here or in a capability extension ([the file table](#every-gsd-file-maps-to-a-kind));
nothing is kept as Markdown.

| File | Kinds |
|---|---|
| [project.md](project.md) | project, requirement, milestone, phase, workstream, retrospective, lesson, methodology |
| [discuss.md](discuss.md) | context, decision, deferred_idea, phase_spec, locked_req, interview, discussion, research and its items |
| [plan.md](plan.md) | plan, task, checkpoint, truth, artifact, key_link, prohibition, disposition, user_setup |
| [execute.md](execute.md) | summary, deliverable, deviation, commit, handoff, pause, blocker; the session state |
| [verify.md](verify.md) | verification, verify_gap, override, human_item, uat_session, uat_test, uat_gap, validation, validation_row, wave0_item, manual_check, broken_window, refactor_proposal |
| [settings.md](settings.md) | the settings and profiles that replace `config.json` |
| [rules.md](rules.md) | every GSD check as a rule, with its code, kind and profile |

## Notation

```
 str     quoted string            text   triple-quoted prose          int     integer        int{1..4}  bounded
 bool    true/false               date   YYYY-MM-DD                   ts      ISO-8601 string
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
| `U` | human-only: refused in every extension's plan and every MCP write; the CLI writes it only from an interactive terminal; hand edits stay free | K5 |

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
| task, truth, artifact, key_link | `T1`, `TR1`, `AR1`, `KL1` | plan | `03-02/T1` |
| decision | `D-01` | phase (project-level decisions: project) | `03/D-01` |
| uat_gap | `G-03-1` | phase | `03/G-03-1` |
| deliverable | `D1` | summary | `03-02/D1` |
| broken_window | `BW1` | project | `BW1` |
| refactor_proposal | `RP1` | phase | `03/RP1` |

A workstream is a scope too: `auth-rework/v1.0/03` names phase 03 of milestone v1.0 of the workstream `auth-rework`.
Commands accept the short form where the scope is known (`/gsd-plan-phase 03.1` in the active milestone), and any
unique suffix; several matches are refused, naming each.

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
  verify_automated "npm test -- session"
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
| git history (no GSD file) | commit (collected, never authored) | core |
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
