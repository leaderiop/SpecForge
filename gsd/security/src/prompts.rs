//! `/gsd-secure-phase` as the prompt `secure_phase`, ported from upstream's command and workflow. The pipeline is
//! `gsd_kit::prompt`; the text is `assets/`. The phase's security audit is its `security_audit` entity: the tail of
//! the prompt names its ids and commands, the judged rules this prompt records and the verdicts of the phase that are
//! not fresh, and ends with the template, which names the kinds.

use gsd_kit::prompt::args::{self, Arg};
use gsd_kit::prompt::{render, repoint};
use specforge_extension_sdk::prelude::*;

use crate::settings;

macro_rules! asset {
    ($path:literal) => {
        (
            $path,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path)),
        )
    };
}

/// `(path, text)`, sorted by path: the files `gsd/security/assets` holds under upstream's paths.
pub static ASSETS: &[(&str, &str)] = &[
    asset!("commands/gsd/secure-phase.md"),
    asset!("gsd-core/templates/SECURITY.md"),
    asset!("gsd-core/workflows/secure-phase.md"),
];

/// The text of an embedded asset, attribution line included.
pub fn asset(path: &str) -> Option<&'static str> {
    ASSETS.iter().find(|(p, _)| *p == path).map(|(_, t)| *t)
}

pub const ID: &str = "secure_phase";
/// The judged rule this prompt records on each closed threat.
const MITIGATION: &str = "gsd.security/W302";
/// The judged rule this prompt records on the phase's security audit.
const FLAGS: &str = "gsd.security/W303";
const TEMPLATE: &str = "gsd-core/templates/SECURITY.md";
const DESCRIPTION: &str = "Retroactively verify threat mitigations for a completed phase";

const ARGS: &[Arg] = &[Arg::subject("phase", "phase")];

/// The keys the workflow reads in place of `config-get`.
const READS: &[&str] = &[
    "response_language",
    "workflow.text_mode",
    "workflow.security_enforcement",
    "workflow.security_asvs_level",
    "workflow.security_block_on",
];

/// The settings block of the preamble: the keys above that have a value.
fn block() -> String {
    let current = Settings::current();
    let rows: Vec<String> = READS
        .iter()
        .filter_map(|key| Some(format!("  \"{key}\": {}", current.get(key)?)))
        .collect();
    format!("{{\n{}\n}}", rows.join(",\n"))
}

/// The preamble's table of where GSD's files went: the security files (`spec/README.md`), and the plan row whose
/// entities carry the threat model.
const FILES: &[(&str, &str)] = &[
    (
        "`NN-MM-PLAN.md`",
        "plan and its tasks (core's), `trust_boundary`, `threat`",
    ),
    (
        "`NN-SECURITY.md`",
        "security_audit, accepted_risk, audit_run",
    ),
];

/// The phase number as the shell blocks name the end of an id, and how phases are ordered: `02.1` after `02`.
fn order(number: &str) -> Vec<u64> {
    number.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// The latest `complete` phase: the one upstream's "defaults to last completed phase" means (ADR 0314).
// shortcut: milestones compare as strings (`v1.0` < `v1.1` < `v2.0`), upgrade when a milestone id breaks that.
fn latest_complete(call: &PromptCall<'_>) -> Option<String> {
    let nodes: &[_] = call.graph().map_or(&[], |g| &g.nodes);
    nodes
        .iter()
        .filter(|n| n.kind == "phase" && n.text("status") == Some("complete"))
        .max_by_key(|n| {
            let (milestone, number) = n.id.rsplit_once('/').unwrap_or(("", &n.id));
            (milestone.to_string(), order(number))
        })
        .map(|n| n.id.clone())
}

/// The entities of `phase` this prompt judges whose judgement is not fresh (missing, stale, unrecorded or unreadable),
/// as `(rule, id)` sorted: each closed threat of its plans (by the threat's `plan` edge and the plan's `phase`) for
/// W302, its security audit for W303. What the judge step records, so a re-run judges only those.
fn not_fresh(call: &PromptCall<'_>, phase: &str) -> Vec<(&'static str, String)> {
    let nodes: &[_] = call.graph().map_or(&[], |g| &g.nodes);
    let in_phase = |plan: &str| {
        nodes
            .iter()
            .any(|n| n.kind == "plan" && n.id == plan && n.text("phase") == Some(phase))
    };
    let owed = |n: &GraphNode| match n.kind.as_str() {
        "threat" if n.text("status") == Some("closed") && n.text("plan").is_some_and(in_phase) => {
            Some(MITIGATION)
        }
        "security_audit" if n.text("phase") == Some(phase) => Some(FLAGS),
        _ => None,
    };
    let mut left: Vec<(&'static str, String)> = nodes
        .iter()
        .filter_map(|n| {
            let rule = owed(n)?;
            let fresh = n
                .judgements
                .iter()
                .any(|j| j.rule == rule && j.state == "fresh");
            (!fresh).then(|| (rule, n.id.clone()))
        })
        .collect();
    left.sort();
    left
}

/// The judged rules this prompt records, how, and the verdicts of the phase it has left to record.
fn judging(call: &PromptCall<'_>, phase: &str) -> String {
    let left = not_fresh(call, phase);
    let list = if left.is_empty() {
        "Every verdict this prompt judges is fresh: none is left to judge.".to_string()
    } else {
        let lines: Vec<String> = left
            .iter()
            .map(|(rule, id)| format!("- `{rule}` on `{id}`"))
            .collect();
        format!(
            "The verdicts that are not fresh, the ones to judge:\n\n{}",
            lines.join("\n")
        )
    };
    format!(
        "## The judged verdicts here (@gsd/security)

This prompt judges two rules. The auditor returns its findings and records nothing; after the audit is recorded, you \
record each verdict:

- `{MITIGATION}` on each closed threat of the phase: its disposition holds (`mitigate`, the mitigation is in its \
files; `accept`, an accepted risk records it; `transfer`, the transfer is documented). Outcome `holds` or `broken`.
- `{FLAGS}` on the phase's security audit: every threat flag the phase's summaries raise maps to a threat of its \
register (`mapped`), or one does not (`unregistered`, the auditor's unregistered flag: name each in `--findings`).

For each, read its inputs with `specforge query <entity> --inputs <rule> --format json`, judge them, then record the \
outcome over the `basis` it printed: `specforge gsd record-security-verdict <entity> <W302|W303> <outcome> <model> \
'<basis>'` (`--cited '[{{\"file\": \"<path>\"}}]'` names what shows it).

A verdict stays fresh until one of its inputs changes: an edit to a file a threat names stales its W302, a new or \
edited summary or threat of the phase stales the audit's W303, and only the stale ones are judged again.

{list}"
    )
}

/// The closing section: the ids the shell blocks use, the security audit's entities, the commands that write them,
/// the judged verdicts left to record, and the template that names the kinds.
fn tail(call: &PromptCall<'_>, phase: &str) -> String {
    let template = asset(TEMPLATE)
        .map(render::without_notice)
        .unwrap_or_default();
    format!(
        "In the shell blocks above, `$PHASE_ID` is `{phase}`.

## The security audit here (@gsd/security)

The phase's security audit is its `security_audit` entity (`<phase>/NN-SECURITY`): its `threat` and `trust_boundary` \
entities are in the phase's plans, its accepted risks log is the `accepted_risk` entities and its audit trail the \
`audit_run` entities. `threats_open` and `status` are computed from the threats: never written. Nothing is written \
by hand; two commands write it:

- `specforge gsd record-security-audit $PHASE_ID --verdict '<json>' [--run-by <who>]` creates the audit on the first \
call, writes each threat's `status` and `evidence`, and appends an `audit_run` only when the counts changed. The \
verdict is `{{\"threats\": [{{\"id\": \"T-01-01\", \"status\": \"open|closed\", \"evidence\": \"<where>\"}}]}}`; a \
threat of a phase planned without a register is recorded with `\"retroactive\": true`, its `plan` and its `category`, \
`component`, `severity`, `disposition` and `mitigation`.
- Step 4's option 2 records what a person says: ask the person to run `specforge gsd accept-risk <threat> <rationale> \
<who>` in their terminal (an agent's call is refused). A threat with an accepted risk no longer counts in \
`threats_open`.

`workflow.security_*` are `@gsd/security`'s settings: `specforge settings set @gsd/security <name> <value>`.

{}

The template of those entities:

<file path=\"{TEMPLATE}\">
{}
</file>",
        judging(call, phase),
        template.trim_end()
    )
}

fn handle(call: &PromptCall<'_>) -> Result<PromptOutput, String> {
    let mut arguments = args::text(call, ARGS);
    let mut picked = None;
    let phase = match call.str("phase") {
        Some(id) => id.to_string(),
        None => {
            let Some(id) = latest_complete(call) else {
                // upstream's state C
                return Ok(PromptOutput::user(
                    "No phase is complete: it was not executed. Run /gsd-execute-phase first.",
                ));
            };
            let number = id.rsplit('/').next().unwrap_or_default();
            arguments = format!("{number} {arguments}").trim().to_string();
            picked = Some(id.clone());
            id
        }
    };
    let values = args::values(call, ARGS, Some(("phase", &phase)))?;
    let text = render::render(&render::Input {
        command: "secure-phase",
        host: call.host().unwrap_or("claude-code"),
        arguments: &arguments,
        values: &values,
        settings: &block(),
        extension: "@gsd/security",
        files: FILES,
        own: false,
        tail: &tail(call, &phase),
        asset,
        // shortcut: every gsd-core path security's texts name is a core asset; ask core's list when one is not
        repoint: |text, host| repoint::repoint(text, host, &|_| true),
    })?;
    let out = PromptOutput::user(text);
    Ok(match picked {
        Some(id) => out.with_subject(id),
        None => out,
    })
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.prompt(ID, |p| {
        p.title("Verify a phase's threat mitigations")
            .input(Input::Graph)
            .description(DESCRIPTION)
            .attribution(gsd_kit::agent::attribution("commands/gsd/secure-phase.md"));
        args::declare(p, ARGS);
        p.subject("phase");
        p.step("audit", |s| {
            s.title("Verify the threat mitigations")
                .agent(settings::AGENT);
        });
        p.step("record", |s| {
            s.title("Record the audit (specforge gsd record-security-audit <phase> --verdict <json>)")
                .command("record_security_audit");
        });
        p.step("judge", |s| {
            s.title("Judge each closed threat's mitigation (W302) and the phase's threat flags (W303) and record them (specforge gsd record-security-verdict <entity> <W302|W303> <outcome> <model> '<basis>')")
                .command("record_security_verdict");
        });
        p.step("accept", |s| {
            s.title("Ask the person to run `specforge gsd accept-risk <threat> <rationale> <who>` in their terminal (an agent's call is refused: it records what a person says)")
                .command("accept_risk");
        });
        p.handler(handle);
    });
    c.slash_prompt(ID, |s| {
        s.argument_hint("[phase number]")
            // upstream's Read, Write, Edit, Bash, Glob, Grep, Agent, AskUserQuestion
            .tools(&[
                "read", "write", "edit", "shell", "search", "agent", "ask_user",
            ])
            .attribution(gsd_kit::agent::attribution("commands/gsd/secure-phase.md"))
            // upstream's `audit_review` cluster hides it: an own setting that follows core's (`settings.rs`)
            .when(settings::SURFACE_SETTING);
    });
}
