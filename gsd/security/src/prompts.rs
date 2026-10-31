//! `/gsd-secure-phase` as the prompt `secure_phase`, ported from upstream's command and workflow (plan 16 T22). The
//! pipeline is `gsd_kit::prompt`; the text is `assets/`. The workflow's `SECURITY.md` is the phase's `security_audit`
//! entity: the tail of the prompt says so and ends with upstream's template, whose columns are the kinds' fields.

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

/// The closing section: the ids the shell blocks use, what `SECURITY.md` is here, the commands that write it, and the
/// template whose columns are the kinds' fields.
fn tail(phase: &str) -> String {
    let template = asset(TEMPLATE)
        .map(render::without_notice)
        .unwrap_or_default();
    format!(
        "In the shell blocks above, `$PHASE_ID` is `{phase}`.

## The security audit here (@gsd/security)

`SECURITY.md` is the phase's `security_audit` entity (`<phase>/NN-SECURITY`): its `threat` and `trust_boundary` \
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

The template, whose columns are fields of those entities:

<file path=\"{TEMPLATE}\">
{}
</file>",
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
                    "No phase is complete: it was not executed. Run /gsd:execute-phase first.",
                ));
            };
            let number = id.rsplit('/').next().unwrap_or_default();
            arguments = format!("{number} {arguments}").trim().to_string();
            picked = Some(id.clone());
            id
        }
    };
    let text = render::render(&render::Input {
        command: "secure-phase",
        host: call.host().unwrap_or("claude-code"),
        arguments: &arguments,
        settings: &block(),
        tail: &tail(&phase),
        asset,
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
            .reads_graph()
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
            .attribution(gsd_kit::agent::attribution("commands/gsd/secure-phase.md"));
        // shortcut: upstream's `audit_review` cluster would hide it (`specforge.surface.audit_review`), but a slash's
        // `when` must be a setting of its own extension (W021) and the cluster is core's; upgrade with a kernel round.
    });
}
