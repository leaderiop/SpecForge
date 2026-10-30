//! The preamble every lifecycle prompt starts with (D-r42-7). It is ours, not GSD's: upstream's workflows talk about
//! files in a planning folder and about a tools script, and SpecForge has neither, so this says where each thing went
//! and what to call instead. The ported text below it keeps upstream's words.

/// The first line of the preamble: tests look for it, and a reader can tell where ours ends.
pub const MARKER: &str = "<specforge_gsd_preamble>";
const END: &str = "</specforge_gsd_preamble>";

/// The core rows of `spec/README.md` ("Every GSD file maps to a kind"): `(GSD file, kinds)`.
pub const FILES: &[(&str, &str)] = &[
    ("`PROJECT.md`", "project, requirement, decision"),
    ("`REQUIREMENTS.md`", "requirement"),
    (
        "`ROADMAP.md`, `MILESTONES.md`, `milestones/*`",
        "milestone, phase",
    ),
    (
        "`STATE.md`",
        "computed (progress, position) + pause, blocker + session state",
    ),
    ("`config.json`", "settings"),
    (
        "`RETROSPECTIVE.md`, `LEARNINGS.md`, `METHODOLOGY.md`",
        "retrospective, lesson, methodology",
    ),
    (
        "`NN-CONTEXT.md`, `NN-DISCUSSION-LOG.md`",
        "context, decision, deferred_idea, discussion",
    ),
    ("`NN-SPEC.md`", "phase_spec, locked_req, interview"),
    (
        "`NN-RESEARCH.md`, `research/*.md`",
        "research and its items",
    ),
    (
        "`NN-MM-PLAN.md`",
        "plan, task, checkpoint, truth, artifact, key_link, prohibition, disposition, user_setup",
    ),
    (
        "`NN-MM-SUMMARY.md`",
        "summary, deliverable, deviation, decision",
    ),
    ("`.continue-here.md`", "handoff"),
    (
        "`NN-VERIFICATION.md`",
        "verification, verify_gap, override, human_item, and verdicts",
    ),
    ("`NN-UAT.md`", "uat_session, uat_test, uat_gap"),
    (
        "`NN-VALIDATION.md`",
        "validation, validation_row, wave0_item, manual_check",
    ),
    ("`v<version>-MILESTONE-AUDIT.md`", "milestone_audit"),
    (
        "git history (no GSD file)",
        "commit (written by the git collector, never by an agent); a JUnit report is read by the `gsd-junit` collector, which records tests, not entities",
    ),
    ("`WINDOWS.md`", "broken_window"),
    ("`NN-REFACTOR.md`", "refactor_proposal"),
    ("`quick/*`", "plan, summary (a quick plan has no phase)"),
    ("`workstreams/*`", "workstream (a scope)"),
];

/// How an agent spawns a sub-agent on a host.
fn spawning(host: &str) -> &'static str {
    match host {
        "codex" | "cursor" => {
            "as the adapter header above says (the agent is the installed `gsd-<name>`)"
        }
        _ => "`Task(subagent_type=\"gsd-<name>\", ...)` (the agent is the installed `gsd-<name>`)",
    }
}

/// The preamble for `host`, with `settings` (a JSON object, `settings::block()`) as the current configuration.
pub fn text(host: &str, settings: &str) -> String {
    let mut table = String::from("| GSD file | Kinds |\n|---|---|\n");
    for (file, kinds) in FILES {
        table.push_str(&format!("| {file} | {kinds} |\n"));
    }
    let spawn = spawning(host);
    format!(
        "{MARKER}
You are running a GSD workflow inside SpecForge (the extension `@gsd/core`). The workflow text below is GSD's own. The
files it talks about are entities of the project's spec here, and the tools it calls are SpecForge's. This section says
where each thing went; where it and the workflow disagree, this section wins.

## Where GSD's files went

{table}
Read an entity with `specforge query <id> [--kinds <kind>] [--depth <n>] [--format json]` (MCP tool `specforge.query`).
Check the spec with `specforge check`. Create and edit with `specforge new <kind> <id> ...`, `specforge set`,
`specforge unset` and `specforge link`, or the MCP tool `specforge.edit`. An artifact is created through
`specforge.edit` (the command line's `--path` option is its own).

## How to work

- **Computed is never written.** A status, a progress, a position and a count are computed from the graph. Where the
  workflow says to update one, there is nothing to do.
- **Harness fields have one writer**: the command of `@gsd/core` that owns them (`specforge gsd <command>`; \
`specforge gsd help` lists them), never `specforge set`.
- **A command step's line has no arguments.** The step names the command; this text gives its arguments. Every
  `--by` names who answers: `--by \"<user>\"` for a person, the agent's name for an agent.
- **Judges record their own verdicts** with `specforge gsd record-verdict` (the agent's body says how). An
  orchestrator never records a verdict on a judge's behalf.
- **A refused call** (`gate_failed`, `human_only`, `stale_revision`, `invalid_input`) is reported to the user with its
  message. Never edit the spec to get around it.
- **An agent that `@gsd/core` does not install** (`gsd-debugger`, `gsd-ui-researcher`, `gsd-ui-checker`,
  `gsd-security-auditor`, `gsd-code-reviewer`, `gsd-codebase-mapper`, ...) is skipped, and the skip is said.
- **Spawning an agent** on this host: {spawn}. The model is the installed agent's own: skip any model resolution.
- **References** the workflow names are read with `specforge gsd reference <path>`.
- **Ids in the shell blocks.** `$PHASE_ID` and `$PLAN_ID` are the entity the run is about (the end of this text names
  it), `$MILESTONE` is the current milestone and `$PROJECT` the project (the end of this text names them too).

## Settings

Where the workflow reads `config-get <key>`, read the key here. To change one: `specforge settings set @gsd/core
<name> <value>`.

```json
{settings}
```
{END}

"
    )
}
