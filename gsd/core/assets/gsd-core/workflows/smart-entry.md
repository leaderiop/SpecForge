<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/smart-entry.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
<purpose>
GSD smart entry — the state-aware front door. Detect the current project situation via `specforge gsd smart-entry --format json`, present a short menu of the right next actions, and dispatch to exactly one existing GSD command. This is a launcher/router only; it never does the work itself.

This is a *menu* front door, not a second router. For in-project forward motion (planning → executing → verify-pending) the recommended action is `/gsd:progress --next`, which delegates to the single gated advancement engine (`workflows/next.md`: Route 0 resume-incomplete-phase + Gates 1-3). smart-entry adds value only where `--next` cannot reach: pre-project, remediation (paused/blocked/verify-failed), and lifecycle exits (idle-stranded/complete). See `docs/adr/1787-gsd-next-smart-entry.md`.
</purpose>

<required_reading>
Read all files referenced by the invoking prompt's `execution_context` before starting.
</required_reading>

<process>

<step name="text_mode">
**TEXT_MODE handling (non-Claude runtimes).**

Set `TEXT_MODE=true` if `--text` is present in `$ARGUMENTS` OR `text_mode` from init JSON is `true`. When TEXT_MODE is active, replace every `AskUserQuestion` call with a plain-text numbered list and ask the user to type their choice number. This is required for non-Claude runtimes (OpenAI Codex, Antigravity, etc.) where `AskUserQuestion` is not available.
</step>

<step name="resolve">
**There is no launcher to resolve.**

The commands below are specforge commands on PATH: nothing is searched for and no function is defined. The `runtime` setting names the host the menu is written for.

```bash
```
</step>

<step name="detect">
**Detect the situation.**

```bash
RUNTIME="{{setting:runtime|@host}}"   # runtime (the settings); the commands below are specforge commands: there is no launcher to define
RESPONSE_LANGUAGE="{{setting:response_language|}}"   # response_language (the settings)
SNAPSHOT=$(specforge gsd smart-entry --format json 2>/dev/null)
```

**If `response_language` is set:** All user-facing output of this workflow — narration between tool calls, status updates, progress notes, findings, questions, prompts, and explanations — MUST be presented in `{response_language}`. Technical terms, code, file paths, and subagent prompts stay in English — only user-facing output is translated.

Parse `SNAPSHOT` as JSON. It has the shape:

```json
{
  "situation": "executing",
  "recommended": "progress-next",
  "summary": "Phase 2 of 5 · 60% · executing",
  "signals": { "...": "..." },
  "actions": [
    { "id": "progress-next", "label": "Advance to the next step", "command": "/gsd:progress --next", "recommended": true },
    { "id": "execute-phase", "label": "Continue executing phase 2", "command": "/gsd:execute-phase", "recommended": false }
  ]
}
```

`situation` is one of: `no-project`, `paused`, `blocked`, `verify-failed`, `needs-first-phase`, `planning`, `executing`, `verify-pending`, `idle-stranded`, `complete`, `unknown`.

**Fallback (never strand the user):** `smart-entry --json` can fail for two reasons, and each has a different recovery. Parse `SNAPSHOT`; if it is empty, not valid JSON, or missing `actions`, apply the first matching recovery below — do NOT error.

1. **`specforge` itself fails** (the failure is a `command not found` / crash, not just an empty result). Probe by running `specforge gsd progress-report` — if THAT also errors, the whole tool layer is down and routing to `/gsd:progress` would dead-end too (it also needs specforge). **Recover by reading state directly:**
   - Read the spec files directly with the Read tool (`spec/*.spec`; `specforge query` is down too). Extract: the open `milestone`, its `phase` entities (not the `999.x` backlog), which of their `plan` entities have a `summary`, and any `blocker` or `pause`. The computed `status` is not in the files, so judge it from those plans and summaries.
   - Synthesize a minimal result: `situation` = your best guess from the status text (`executing`/`verifying`/`planning`/`complete`/`paused`), `summary` = a one-line read ("Phase N of M · status"), and an `actions` list built from status (e.g. verifying → `/gsd:verify-work`, executing → `/gsd:execute-phase`, else `/gsd:progress`), always including `/gsd:quick` and `/gsd:help`.
   - Print one line first: `smart-entry unavailable (the CLI failed) — reading state directly.`
   - Proceed to the `present` step with this synthesized result.

2. **Only `smart-entry` is unavailable** (e.g. older gsd-core without the subcommand; `state-snapshot` still works). Run `/gsd:progress` and stop. Print one line first: `smart-entry unavailable — showing progress.`
</step>

<step name="present">
**Present the menu.**

Show the `summary` line to orient the user, then offer the actions.

**If TEXT_MODE is false:** call `AskUserQuestion` with:
- `header`: a short label derived from `situation` (e.g. `executing` → "Continue work", `blocked` → "Unblock", `no-project` → "Get started", `complete` → "What next?").
- `question`: the `summary` line, then "What would you like to do?"
- `options`: the first 4 entries of `actions[]` in order. For each, `label` = the action's `label`, `description` = the action's `command`. The recommended action is already first; surface it as the first option. The user may also type a custom command (handled automatically).

**If TEXT_MODE is true:** print the `summary`, then a numbered list of ALL `actions[]` (not capped to 4 — text has no limit), then ask the user to type the number of their choice:

```
{summary}

  1. {actions[0].label}  ({actions[0].command})
  2. {actions[1].label}  ({actions[1].command})
  ...

Type a number, or describe what you want to do.
```

Wait for the user's response before continuing. Map the chosen number to the corresponding action.
</step>

<step name="display">
**Show the routing decision.**

```
### GSD ► SMART ENTRY

**Situation:** {situation}
**Routing to:** {chosen command}
```
</step>

<step name="dispatch">
**Dispatch and stop.**

Invoke the chosen action's `command`. If the user typed a free-form response instead of picking an action, treat it as freeform intent and route via `/gsd:progress --do "<their text>"`.

After invoking the command, **stop**. The dispatched command owns everything from here. Do not continue, do not chain, do not re-enter this workflow.
</step>

</process>

<success_criteria>
- [ ] Situation detected via `specforge gsd smart-entry --format json`
- [ ] Summary shown to orient the user
- [ ] Menu offered (AskUserQuestion, or numbered list under TEXT_MODE)
- [ ] Routing decision displayed before dispatch
- [ ] Exactly one command dispatched
- [ ] Any detection failure falls back to /gsd:progress (never strands the user)
- [ ] No work done directly — launcher only
</success_criteria>
