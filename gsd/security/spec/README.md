# `@gsd/security` model

**Status: built, with its two judged rules, `W302` (a closed threat's mitigation) and `W303` (a phase's threat flags), recorded by `secure_phase` ([Judged mitigation](#judged-mitigation)). Tracks gsd-core 1.19.0.** The kinds, computed values, rules, settings, commands and hooks that
`@gsd/security` declares beside `@gsd/core` ([ADR gsd/0002](../../adr/0002-a-capability-extension-plugs-into-core-at-its-hook-points.md)).
GSD's `<threat_model>` blocks and `NN-SECURITY.md` are entities here; nothing is kept as Markdown. Field flags are
those of [`gsd/core/spec/README.md`](../../core/spec/README.md): `R` required, `I` immutable, `C` computed, `H` written
by a command (a harness field), `U` a person's. Codes are `gsd.security/<letter><3 digits>`.

## Kinds

| Kind | Name | Scoped by | Fields | GSD |
|---|---|---|---|---|
| `trust_boundary` | `TB1` | its plan | `plan → plan R I`, `boundary str R`, `description str R`, `data_crossing str` | `<threat_model>` Trust Boundaries; SECURITY.md Trust Boundaries |
| `threat` | `T-03-01`, `T-03-SC` | its plan | `plan → plan R I`, `category enum{spoofing, tampering, repudiation, information_disclosure, denial_of_service, elevation_of_privilege} R`, `component str R`, `severity enum{critical, high, medium, low} R`, `disposition enum{mitigate, accept, transfer} R`, `mitigation str R`, `files paths<planned>` (checked once the threat is closed, E016), `status enum{open, closed} H`, `evidence str H`, `retroactive bool H`, `blocking bool C`; complete when `status` is `closed` | STRIDE register row |
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
| `gsd.security/W301` | phase | warning | `workflow.security_enforcement` | "`<p>-SECURITY` is missing / is still draft / has `<n>` threat(s) open: run /gsd-secure-phase `<p>`", on a phase whose status is `complete`; one line per phase (open threats before draft), where upstream can say both | #4972 |
| `gsd.security/W302` | threat | warning | `workflow.security_enforcement` | "threat '`<id>`' has no fresh mitigation verdict: no verdict / the verdict of `<at>` is stale: `<inputs>` changed / the verdict of `<at>` is 'broken'", on a closed threat; a judged rule (below) | the auditor's per-threat verification (`gsd-security-auditor`, Threat Verification) |
| `gsd.security/W303` | security_audit | warning | `workflow.security_enforcement` | "security audit '`<id>`' has no fresh threat-flag verdict: no verdict / the verdict of `<at>` is stale: `<inputs>` changed / the verdict of `<at>` is 'unregistered'"; a judged rule (below) | the auditor's `unregistered_flag` and SECURITY.md's Unregistered Flags (`gsd-security-auditor`) |

## Judged rules' inputs

A judged rule (ADR 0377) is answered by a verdict that `record_security_verdict` writes, under the qualified rule
(`verdict gsd.security/W302 holds { … }`), with a line in `specforge-harness`. Its named inputs are digested when it is
recorded; an edit to one makes it stale. Every verdict also digests its `cited` and `cited_tests` lists. The judge is the
prompt `secure_phase`: the auditor returns its findings and the orchestrator records. A gated rule is never evaluated.

| Rules | Target | Inputs | Binds |
|---|---|---|---|
| gsd.security/W302 | threat | `threat` self · `plan` self.plan · `files` file(self.files) · `accepted` self<accepted_risk.threat · `rows` self<validation_row.threat; judged once the threat is closed (`judged_while complete`) | — |
| gsd.security/W303 | security_audit | `audit` self · `summaries` self.phase<plan.phase<summary.plan · `threats` self.phase<plan.phase<threat.plan | — |

## Judged mitigation

Upstream's auditor checks each threat of the register by its disposition and writes the result into `SECURITY.md`,
where nothing checks it again once the code changes. Here each result is a judged rule's verdict (ADR 0377), so it goes
stale when what it read changes, and the workflow judges again exactly the verdicts that are not fresh.

- **What W302 judges.** A closed threat claims its disposition holds: for `mitigate`, the mitigation is in its files;
  for `accept`, an accepted risk records it; for `transfer`, the transfer is documented. The outcomes are `holds` and
  `broken` (passing `holds`); `broken` carries the auditor's findings and is the rule's finding.
- **When it is owed.** Once the threat is closed (`status` `closed`, its kind's `complete_when`, ADR 0362). An open
  threat owes none: W301 already says the audit has it open.
- **What it reads.** The threat, its plan, the files it names in `files`, the accepted risks that name it and the
  `VALIDATION.md` rows that secure it (the inputs above). `files` is written with the threat, a SpecForge field that
  upstream's register has no column for: an edit to a file a threat names stales exactly the threats that name it, and
  a threat that names none is judged over the rest. The vault twin's closed threats name theirs
  ([fixtures/vault](../../parity/fixtures/vault/README.md)).
- **W303.** A phase's audit says whether every threat flag its summaries raise maps to a threat of its register:
  `mapped`, or `unregistered` with the unmapped flags as findings (upstream's `unregistered_flag`). It reads the audit
  and the phase's whole summaries and threats, since a selector cannot pick one string-list field: an edit to any field
  of a summary of the phase stales it, the safe side. It is owed from the audit's creation.
- **Who records.** The prompt `secure_phase` is the judge of both. After `record_security_audit` its step `judge` reads
  each verdict's inputs (`specforge query <entity> --inputs gsd.security/W302 --format json`) and records the outcome
  with `specforge gsd record-security-verdict <entity> <W302|W303> <outcome> <model> '<basis>'` (MCP
  `specforge.gsd.record_security_verdict`). Its closing section lists the closed threats of the phase whose W302 verdict
  is not fresh (missing, stale, unrecorded or unreadable) and the audit while its W303 verdict is not, so a re-run
  judges only those. Each extension writes its own rules: core's `record_verdict` refuses `gsd.security/` rules and
  `record_security_verdict` refuses core's.
- **At a gate.** Under gsd-strict and gsd-plus, `gsd/E516` refuses a workflow step on a verdict that is not fresh. Its
  exception for the workflow that judges a verdict covers `gsd/` rules only, so a stale, unrecorded or unreadable
  `gsd.security/` verdict refuses every core gate, wherever it is, until `secure_phase` judges it again, and a missing
  one refuses the gates whose scope holds its threat or audit.

Both rules are warnings, gated by `workflow.security_enforcement`. A verdict never changes a threat's `blocking` or the
audit's `threats_open`.

## Settings

| Key | Type | Default | Notes |
|---|---|---|---|
| `workflow.security_enforcement` | bool | `true` | gates `W301`, `W302` and `W303`; the `when` of all three hooks |
| `workflow.security_asvs_level` | int 1..3 | `1` | the level the audit runs at |
| `workflow.security_block_on` | enum critical, high, medium, low, none | `high` | the lowest blocking severity |
| `security.injection_blocking` | bool | `false` | read by `@gsd/core`'s injection scanner (`[blocking-needs-security]`) |
| `model_overrides.gsd-security-auditor` | enum heavy, standard, light | preset by `@gsd/core:model_profile` | the auditor's host tier |
| `specforge.agent_variant` | enum full, compact | preset by `@gsd/core:workflow.compact_content` | the auditor's body |

## Commands

`specforge gsd <command>` (MCP `specforge.gsd.<id>`).

| Id | Writes | Answers | Upstream twin |
|---|---|---|---|
| `security_hooks <point>` | no | `{activeHooks, skippedHooks}`: the capability's hooks at `<point>`, `when` read from the settings, `configValues` filled; `--after-fingerprint <phase>` moves the `verify:post` step to `skippedHooks` once the phase has its audit, as upstream (#5105) | `loop render-hooks <point>` (the `capId: security` entries) |
| `security_gate <phase>` | no | `{block, message, details: {code, exists, status, threats_open}}`; blocks with no audit (`SECURITY_SHIP_GATE_NO_REVIEW`) or `threats_open != 0` (`SECURITY_SHIP_GATE_OPEN_THREATS`); `block: false` when enforcement is off | `check predicate` and ship.md's security branch |
| `threat_ids <phase>` | no | `{threat_id_duplicates: [{id, plans}], threat_id_duplicate_count}` | `init execute-phase` / `init plan-phase` fields |
| `record_security_audit <phase> --verdict '<json>' [--run-by <who>]` | yes | `{appended, threats_open}`; creates the audit, writes each named threat's `status` and `evidence`, creates retroactive threats in a plan of the phase, appends an `audit_run` only if (total, closed, open) changed | workflow step 6 and `verification.append-audit` |
| `accept_risk <threat> <rationale> <by>` | yes, `human` | an `accepted_risk` under the phase's audit (created if absent) | workflow step 4, option 2 |
| `record_security_verdict <entity> <rule> <outcome> <model> '<basis>'` | yes | a `verdict gsd.security/<rule>` on the entity and its ledger line; refuses a rule that is not this extension's, an outcome that is not the rule's, and a basis that is not JSON; `--findings`, `--cited`, `--values` as core's `record_verdict` | the auditor's threat verdicts, which the workflow writes |

## Hooks

From `capabilities/security/capability.json`, answered by `security_hooks`:

| Point | Kind | What | `when` |
|---|---|---|---|
| `plan:pre` | contribution into the planner | the fragment, naming `trust_boundary` and `threat` (`[fragment-repointed]`); `configValues` `security_asvs_level`, `security_block_on` | `workflow.security_enforcement` |
| `verify:post` | step | `/gsd-secure-phase`, `produces: ["SECURITY.md"]` as upstream (the plan's table said `security_audit`; the code keeps upstream's value, which the workflow's `ref.skill` check does not read), `onError: halt` | `workflow.security_enforcement` |
| `ship:pre` | gate | `artifact-frontmatter-equals SECURITY.md threats_open 0`, blocking, `onError: halt`; `security_gate` is the verdict | `workflow.security_enforcement` |

## Prompt and agent

`/gsd-secure-phase` is the prompt `secure_phase` (steps: the agent `gsd-security-auditor`, the command
`record_security_audit`, the command `record_security_verdict`, the command `accept_risk`); the auditor is return-only
(#2119): the orchestrator writes the audit with `record_security_audit` and, the prompt being the judge of
`gsd.security/W302` and `W303`, each closed threat's verdict and the audit's with `record_security_verdict`. The
closing section lists the closed threats of the phase whose W302 verdict is not fresh (missing, stale, unrecorded or
unreadable) and the audit while its W303 verdict is not, so a re-run judges only those.
