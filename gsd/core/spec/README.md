# `@gsd/core` model

**Status: the foundation is built (the eight kinds of [project.md](project.md), the code book, the settings and the roadmap checks) and so are the sixteen kinds of [discuss.md](discuss.md) with the rules built so far (`gsd/E201`, `gsd/W204`, `gsd/E206`, `gsd/W406`; `gsd/W205` is judged by the phase researcher and `gsd/E202` and `gsd/W203` by the `spec_phase` prompt (r40)) the nine kinds of [plan.md](plan.md) with their non-judged rules (r37) and the seven kinds of [execute.md](execute.md) with `gsd/E401` to `gsd/E405` (E403 is `gsd/W403`), `gsd/W407` and `gsd/W408` and the two collectors (r38); the fourteen kinds of [verify.md](verify.md) with their computed values, the scan pass and the rules and gate guards listed in [rules.md](rules.md) (r39; `gsd/W514` and E502 wait; E501 is judged by the verifier (r40)); the rest is being designed. Tracks gsd-core 1.18.0.** The kinds, edges, settings and rules `@gsd/core` declares. Every GSD
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
| `U` | human-only: refused in every extension's plan and every MCP write when it is created, deleted or retitled (an agent also cannot set, unset or link an existing one, nor delete an entity holding a person's value: ADR 0298); the CLI writes it only from an interactive terminal; hand edits stay free (a field the host must also refuse an agent's direct edit of is `H`, written by a command that takes a reason) | K5 |

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
[rules](rules.md) and ported prompts ([contributions](../contributions/)), not data; the text a prompt does not inline
is embedded in the extension and served by `specforge gsd reference <path>`. Sketch and spike HTML stays throwaway
output outside the spec tree.

## Waiting on the kernel

ADR 0299 put the computed values and judgements in the graph a call receives, so a command reads a phase's `status`
and a context's `digest`, and `record_research`, `record_spec`, `record_validation`, `/gsd-health`, `/gsd-stats` and the progress
report are built. K17 closed the E080 treatment of a judged rule's finding, a collector's plan judged before its own report, and the
human-only guarantee on existing entities and deletes (ADR 0297, ADR 0298), and `gsd/E502` is built on top of them. What
still waits for kernel work (K19):

- **Baseline re-verification (T13).** `re_verification.previous_status` and `previous_score` are not built, and the
  baseline view of an artifact or key link is not used. The git adapter loads only `*.spec` and `specforge-harness` at a
  commit, so at the baseline every source file reads as absent (`exists: false`): the scan says `missing`, never
  `unchecked`, and the baseline's status and score are wrong. `re_verification` therefore compares fresh verdicts and
  verify gaps only. The baseline compile must serve source files from the commit through the on-demand blob fetch
  (with `baseline-format` 3, ADR 0300 when it is built). Its `regressions`, a verdict-based `gaps_closed` and a reopened gap are tested on
  synthetic baselines, not black-box (the judgement list is empty on a baseline).
- **A person's answer is recorded by the caller (K19).** `answer_checkpoint`, `confirm_setup`, `set_uat_result`,
  `approve_validation`, `waive_window`, `dispose_refactor` and `pause` write the `H` fields of a person's reply, as
  upstream's agent transcribes the conversation. Each requires a non-empty `--by` (an empty one is refused), but `--by`
  is the caller's **attestation**, not a proof of a person: an agent can record any name (`set_uat_result` echoes it and
  stores it nowhere, a `uat_test` has no `by`). So a gate that such a record satisfies (`gsd/W404`'s answered
  `tdd_review` checkpoint, and the sign-offs r42's gates read) is an honest-agent gate. `override` and `debt_accepted`
  stay human-only because no command writes them. A human-attested command kind (the host refuses an agent-facing call
  and records the caller) is a K19 item; the ignored tests `cmd_attest` carry the desired behaviour.
- **`gsd/E516` is subject-scoped at a command gate (K19).** The host keeps only the diagnostics that name an entity
  inside the gate's subject, and E516 names the stale entity itself. So `start_plan <plan>` is refused by a stale
  verdict inside that plan only, and `close_milestone` by one under the milestone only, not project-wide as the rule
  reads. The fix is for the guard to name the subject (K19); an ignored test carries it.
- **A command or prompt cannot read its extension's state (K19).** The host sends `state` to a `reads_state` command or
  prompt, but the SDK's `CommandCall` and `PromptCall` have no accessor (a hook has `HookCall::facts()`). So
  `/gsd-workstreams status` cannot name the active workstream and `--ws` is text. The fix is an SDK change, so a re-vendor.
- **A rendered step has no agent field (K19)**: the step's title carries the agent. **There is no free-text tail for a
  prompt's arguments (K19)**: `--name` and `--task` are quoted. **An agent cannot create an artifact through the CLI
  (K19)**: `--path` is the CLI's own option, MCP's `specforge.edit` creates it. Each is a wire or CLI change.
- **The hooks wait for five host items (K19)**: git facts in `HookInput`, a `file_changed` event, a hook that runs without
  a full compile, Cursor's `beforeSubmitPrompt`, and a hook reading another extension's settings.
- **The refactor trigger differs from upstream's.** `propose_refactor` records one proposal per function while one is
  untriaged (it answers `nothing new` and writes nothing, so a hook may call it on every phase event), as upstream
  freezes the anchor. It does not port upstream's anchor on first observation: the baseline is the score of the latest
  *disposed* proposal, so a function that creeps below the threshold in small steps never fires `jump` until it crosses.
- **`specforge <ext> help <cmd>` is gone for every extension.** T17 turned off clap's `help` subcommand on an
  extension's command line (the extension's own `help` command shadowed it). Use `specforge <ext> <cmd> --help`;
  `specforge gsd help` lists the commands.
- **A slash command's `when` takes one setting (K19).** A skill in two clusters follows its first, so disabling
  `utility` leaves `/gsd-help` (also `core_loop`) shown.
- **`gsd/W514`** is not built: a pass is sent no settings, and a setting gates a code only while it is on. It needs
  `SettingDescriptor.gates_when_on`.
- **A `file()` input names a file-reference field only.** `gsd/E501` and the plan checks read no file as an input,
  because a plan, an artifact and a key link name files that do not exist yet and the host checks every file-reference
  field for existence (E016). The verifier cites the files it read (`cited`); a file it did not cite can change
  without staling its verdict. Likewise `gsd/E518`'s `requirements` input is every requirement: a selector cannot go
  on past a reference (`phase.milestone.requirements`).
- **A `number` setting type**, so the settings that accept decimals (`hooks.context_*_threshold`) are whole numbers.
- **A JSON number for a top-level integer plan op**: gsd writes the decimal text (`collectors/git.rs`).
- **`collect` re-keys short ids**: `gsd-junit` resolves `[gsd:<id>]` itself.
- **The SDK exports `TriggerKind`**: `gates.rs` reads the kind by name.
- **A collector's plan judged for host errors only** (REVIEW-r38 M1), for `gsd/E515` (`gsd-plus` only).
- **What stays after K17.** `cited` defaults to `[]` and code files are not E501 inputs, so a verdict that cites no file
  never goes stale on a code change; `check` is red from planning on, since gsd/E501 is an Error from the first planned
  truth until it is verified; an agent cannot create an artifact through the CLI (`--path` is the CLI's own option); and
  the kernel reports `(0 findings)` in every judged message (it counts a list of values, not of blocks). `gsd/E502`
  counts the tests recorded for a truth (its `[gsd:<id>]` link), not the tests a verdict cites: a pass is not sent
  `cited_tests`.
- **What E502 guarantees today, and what it does not.** `gsd/E502` is enforced only through E080: a plan that records
  `verified` over tests that fail the truth is refused, and the verifier records `failed`. It does not hold when (a) one
  plan holds two `set_verdict` ops that swap an E502 (one clears an existing one, the other adds one on another truth:
  E080 matches diagnostics by message shape with the digits collapsed, so the multiset is unchanged), which a verdict
  recorded before K17 or a hand edit can make reachable; (b) `derive` reads a fresh `verified` as `Verified` whether or
  not E502 stands, so a verification can read `passed` while the Error is reported (gates still fail on the Error, but
  anything reading `status` does not); (c) `verification` (`explicit` or `backstop`) is agent-writable, so an agent can
  turn the guard off for a truth before it records `verified` (the diff shows it, and the verdict goes stale when the
  field flips). K19: make `derive` treat a truth whose fresh verdict the recorded tests contradict as not `Verified`,
  with the predicate E502 uses; key E080's pairing by subject for pass codes; make `verification` human-only or
  harness-only.
- **A judged code reused by a pass (K19).** The registry refuses a declarative rule that shares a judged rule's code, not
  a pass that `reports` it, nor a guard's fail-closed error for it: E080 exempts those findings (ADR 0297 D1b). No gsd
  pass reports a judged code.
