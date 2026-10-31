# `@gsd/security` model

**Status: built. Tracks gsd-core 1.18.0.** The kinds, computed values, rules, settings, commands and hooks that
`@gsd/security` declares beside `@gsd/core` ([ADR gsd/0002](../../adr/0002-a-capability-extension-plugs-into-core-at-its-hook-points.md)).
GSD's `<threat_model>` blocks and `NN-SECURITY.md` are entities here; nothing is kept as Markdown. Field flags are
those of [`gsd/core/spec/README.md`](../../core/spec/README.md): `R` required, `I` immutable, `C` computed, `H` written
by a command (a harness field), `U` a person's. Codes are `gsd.security/<letter><3 digits>`.

## Kinds

| Kind | Name | Scoped by | Fields | GSD |
|---|---|---|---|---|
| `trust_boundary` | `TB1` | its plan | `plan → plan R I`, `boundary str R`, `description str R`, `data_crossing str` | `<threat_model>` Trust Boundaries; SECURITY.md Trust Boundaries |
| `threat` | `T-03-01`, `T-03-SC` | its plan | `plan → plan R I`, `category enum{spoofing, tampering, repudiation, information_disclosure, denial_of_service, elevation_of_privilege} R`, `component str R`, `severity enum{critical, high, medium, low} R`, `disposition enum{mitigate, accept, transfer} R`, `mitigation str R`, `files strs`, `status enum{open, closed} H`, `evidence str H`, `retroactive bool H`, `blocking bool C` | STRIDE register row |
| `security_audit` | `03-SECURITY` | its phase | `phase → phase R I`, `asvs_level int{1..3} H`, `created ts H`, `threats_open int C`, `status enum{draft, verified} C`, `register_planned bool C` | `NN-SECURITY.md` frontmatter |
| `accepted_risk` | `AR1` | its audit | `audit → security_audit R I`, `threat → threat R I`, `rationale str R I`, `accepted_by str R I`, `at date H I`; append-only | Accepted Risks Log |
| `audit_run` | `AU1` | its audit | `audit → security_audit R I`, `at ts H I`, `total int H I`, `closed int H I`, `open int H I`, `run_by str H I`; append-only | Security Audit Trail / `## Security Audit` |
| enhancement of core's `validation_row` | | | `threat → threat`, `secure_behavior str` | VALIDATION.md Threat Ref, Secure Behavior |

`accepted_risk` is **not** a human-only kind (plan 16 said it would be): the host refuses a human command's plan to
create a human-only entity, so `accept_risk` could not write it. Its `at` is a harness field instead, written only by
`accept_risk` with its ledger line, and only a risk with an `at` counts (`[accepted-risk-not-human-kind]`): an agent's
`specforge new accepted_risk` is refused `harness_field` with `--at` and accepts nothing without it. What is left is a
hand edit of the `.spec` file with a forged ledger line (the digest has no secret). A kernel round that lets a human
command write a human kind would close that.

Every column of upstream's `templates/SECURITY.md` is a field above (`declaration.rs` pins the map).

Computed, severity order critical > high > medium > low, `none` blocks nothing:

- `threat.blocking`: `status != closed`, no `accepted_risk` names it, and `severity ≥ workflow.security_block_on`.
- `security_audit.threats_open`: the number of blocking threats of the phase's plans (`[threats-open-live]`).
- `security_audit.status`: `verified` when it has an `audit_run` and `threats_open == 0`, else `draft`.
- `security_audit.register_planned`: a threat of the phase's plans has `retroactive` unset
  (`register_authored_at_plan_time`).

## Codes

| Code | Kind | Severity | Gated by | Says | Upstream |
|---|---|---|---|---|---|
| `gsd.security/E101` | threat | error | none (upstream applies it whatever the hook) | "threat ID `<id>` is claimed by more than one plan of phase `<p>`: `<plans>`; renumber after the phase's highest `T-<p>-NN`" (digit-suffixed IDs only; `-SC` never) | #4683 |
| `gsd.security/W301` | phase | warning | `workflow.security_enforcement` | "`<p>-SECURITY` is missing / is still draft / has `<n>` threat(s) open: run /gsd-secure-phase `<p>`", on a phase whose status is `complete` | #4972 |

## Settings

| Key | Type | Default | Notes |
|---|---|---|---|
| `workflow.security_enforcement` | bool | `true` | gates `W301`; the `when` of all three hooks |
| `workflow.security_asvs_level` | int 1..3 | `1` | the level the audit runs at |
| `workflow.security_block_on` | enum critical, high, medium, low, none | `high` | the lowest blocking severity |
| `security.injection_blocking` | bool | `false` | read by `@gsd/core`'s injection scanner (`[blocking-needs-security]`) |
| `model_overrides.gsd-security-auditor` | enum heavy, standard, light | preset by `@gsd/core:model_profile` | the auditor's host tier |
| `specforge.agent_variant` | enum full, compact | preset by `@gsd/core:workflow.compact_content` | the auditor's body |

## Commands

`specforge gsd <command>` (MCP `specforge.gsd.<id>`).

| Id | Writes | Answers | Upstream twin |
|---|---|---|---|
| `security_hooks <point>` | no | `{activeHooks, skippedHooks}`: the capability's hooks at `<point>`, `when` read from the settings, `configValues` filled; `skippedHooks` is always empty | `loop render-hooks <point>` (the `capId: security` entries) |
| `security_gate <phase>` | no | `{block, message, details: {code, exists, status, threats_open}}`; blocks with no audit (`SECURITY_SHIP_GATE_NO_REVIEW`) or `threats_open != 0` (`SECURITY_SHIP_GATE_OPEN_THREATS`); `block: false` when enforcement is off | `check predicate` and ship.md's security branch |
| `threat_ids <phase>` | no | `{threat_id_duplicates: [{id, plans}], threat_id_duplicate_count}` | `init execute-phase` / `init plan-phase` fields |
| `record_security_audit <phase> --verdict '<json>' [--run-by <who>]` | yes | `{appended, threats_open}`; creates the audit, writes each named threat's `status` and `evidence`, creates retroactive threats in a plan of the phase, appends an `audit_run` only if (total, closed, open) changed | workflow step 6 and `verification.append-audit` |
| `accept_risk <threat> <rationale> <by>` | yes, `human` | an `accepted_risk` under the phase's audit (created if absent) | workflow step 4, option 2 |

## Hooks

From `capabilities/security/capability.json`, answered by `security_hooks`:

| Point | Kind | What | `when` |
|---|---|---|---|
| `plan:pre` | contribution into the planner | the fragment, naming `trust_boundary` and `threat` (`[fragment-repointed]`); `configValues` `security_asvs_level`, `security_block_on` | `workflow.security_enforcement` |
| `verify:post` | step | `/gsd-secure-phase`, `produces: ["SECURITY.md"]` as upstream (the plan's table said `security_audit`; the code keeps upstream's value, which the workflow's `ref.skill` check does not read), `onError: halt` | `workflow.security_enforcement` |
| `ship:pre` | gate | `artifact-frontmatter-equals SECURITY.md threats_open 0`, blocking, `onError: halt`; `security_gate` is the verdict | `workflow.security_enforcement` |

## Prompt and agent

`/gsd-secure-phase` is the prompt `secure_phase` (steps: the agent `gsd-security-auditor`, the command
`record_security_audit`, the command `accept_risk`); the auditor is return-only (#2119): the orchestrator writes the
audit with `record_security_audit`.
