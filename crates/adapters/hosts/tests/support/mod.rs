//! The fixture the goldens render: the `@t/core` and `@t/ui` contributions of `GsdHosts` (settings: `ui_phase` on,
//! everything else default), as the operations hand them to an adapter.

use specforge_ops::host::engine::{EngineLine, Resolved, ServedCommand, resolve};
use specforge_ops::host::sections::tools_section;
use specforge_ops::host::{
    Host, ModelChoice, Rendered, RenderedAgent, RenderedHook, RenderedSkill, RenderedSlash,
    SlashCall, Source,
};
use specforge_protocol_types::{Attribution, HookEvent, OnFailure};

fn source(extension: &str, version: &str, kind: &'static str, id: &str) -> Source {
    Source {
        extension: extension.into(),
        version: version.into(),
        kind,
        id: id.into(),
    }
}

fn body(id: &str, host: Host) -> String {
    format!("# {id}\n\nHost: {}\n\nDo the {id} work.\n", host.as_str())
}

/// The commands the fixture serves an agent: `@t/core`'s `stats` (its `approve` is human-attested).
fn served() -> Vec<ServedCommand> {
    vec![ServedCommand {
        extension: "@t/core".into(),
        line: EngineLine {
            mcp: "specforge.t.stats".into(),
            cli: vec!["specforge t stats".into()],
            use_when: "Show the project's statistics.".into(),
        },
    }]
}

/// What the operations resolve for a contribution of `extension` declaring `tools` (and a slash command's `target`).
fn resolved(extension: &str, tools: &[&str], target: Option<&str>) -> Resolved {
    let declared: Vec<String> = tools.iter().map(|t| (*t).to_string()).collect();
    resolve(&declared, extension, &served(), target)
}

fn attribution() -> Attribution {
    Attribution {
        upstream: "gsd-core".into(),
        path: "agents/gsd-planner.md".into(),
        version: "1.18.0".into(),
        license: "MIT".into(),
        copyright: "Copyright (c) 2026 Open GSD".into(),
    }
}

fn hook(id: &str, event: HookEvent, tools: &[&str], paths: &[&str]) -> RenderedHook {
    // The CLI fixture's deny-policy hooks (`gsd_hosts.rs`).
    let on_failure = if matches!(id, "write_guard" | "validate_commit") {
        OnFailure::Deny
    } else {
        OnFailure::Allow
    };
    RenderedHook {
        route: format!("t.{id}"),
        description: id.into(),
        event,
        tools: tools.iter().map(|t| (*t).to_string()).collect(),
        paths: paths.iter().map(|t| (*t).to_string()).collect(),
        timeout_ms: 5_000,
        on_failure,
    }
}

pub fn fixture(host: Host) -> Rendered {
    let agent = |ext: &str, version: &str, id: &str, description: &str, tools: &[&str]| {
        let r = resolved(ext, tools, None);
        RenderedAgent {
            source: source(ext, version, "agent", id),
            id: id.into(),
            description: description.into(),
            tools: r.classes,
            model: ModelChoice::Inherit,
            effort: None,
            color: None,
            body: body(id, host) + &tools_section(&r.lines),
            attribution: None,
        }
    };
    let skill = resolved("@t/core", &["read", "skill"], None);
    let planner = RenderedAgent {
        model: ModelChoice::Tier("standard"),
        effort: Some("high".into()),
        color: Some("green".into()),
        attribution: Some(attribution()),
        ..agent(
            "@t/core",
            "0.4.0",
            "t-planner",
            "Plans a phase",
            &["read", "write", "shell", "search", "mcp:context7/*"],
        )
    };
    Rendered {
        host,
        agents: vec![
            planner,
            agent(
                "@t/core",
                "0.4.0",
                "t-plan-checker",
                "Checks a plan",
                &[
                    "read",
                    "search",
                    "specforge:trace",
                    "specforge:navigation",
                    "specforge:t.stats",
                ],
            ),
            agent("@t/ui", "1.0.0", "t-ui-checker", "Checks a UI", &["read"]),
        ],
        skills: vec![RenderedSkill {
            source: source("@t/core", "0.4.0", "skill", "t-ns-context"),
            id: "t-ns-context".into(),
            description: "Routes a request about the codebase".into(),
            tools: skill.classes,
            argument_hint: None,
            body: body("t-ns-context", host) + &tools_section(&skill.lines),
            attribution: None,
        }],
        slashes: vec![
            RenderedSlash {
                source: source("@t/core", "0.4.0", "slash", "plan_phase"),
                name: "t-plan-phase".into(),
                description: "Plan the phase, then check the plans".into(),
                argument_hint: Some("<phase>".into()),
                tools: resolved("@t/core", &["read", "agent"], Some("specforge:prompt")).classes,
                effort: Some("max".into()),
                call: SlashCall::Prompt {
                    mcp_name: "specforge.t.plan_phase".into(),
                    cli: "specforge prompt t.plan_phase".into(),
                },
                attribution: Some(attribution()),
            },
            RenderedSlash {
                source: source("@t/core", "0.4.0", "slash", "stats"),
                name: "t-stats".into(),
                description: "Show the project's statistics".into(),
                argument_hint: None,
                tools: resolved("@t/core", &["read"], Some("specforge:t.stats")).classes,
                effort: None,
                call: SlashCall::Command {
                    mcp_tool: "specforge.t.stats".into(),
                    cli: "specforge t stats".into(),
                },
                attribution: None,
            },
        ],
        hooks: vec![
            hook("write_guard", HookEvent::PreTool, &["write"], &[]),
            hook("injection_scan", HookEvent::PostTool, &["read"], &[]),
            hook("isolation", HookEvent::PreTool, &["agent"], &[]),
            hook("validate_commit", HookEvent::PreCommit, &[], &[]),
            hook("statusline", HookEvent::Statusline, &[], &[]),
            hook("context_monitor", HookEvent::PostTool, &[], &[]),
            hook("session_cursor", HookEvent::SessionStart, &[], &[]),
            hook("record_hook", HookEvent::PostTool, &["write"], &["**"]),
            hook(
                "settings_reload",
                HookEvent::FileChanged,
                &[],
                &["specforge.json"],
            ),
            // The host's own hook, wired by every install (ADR 0292); the spec root is the project root here.
            RenderedHook {
                route: "core.spec_guard".into(),
                description:
                    "Deny a write of a computed, harness or human-only value in a spec file".into(),
                event: HookEvent::PreTool,
                tools: vec!["write".into(), "edit".into()],
                paths: vec!["**/*.spec".into()],
                timeout_ms: 10_000,
                on_failure: OnFailure::Deny,
            },
            // The host's second own hook, wired while a command is human-attested (ADR 0301).
            RenderedHook {
                route: "core.human_guard".into(),
                description: "Deny a shell call that runs a human-attested command".into(),
                event: HookEvent::PreTool,
                tools: vec!["shell".into()],
                paths: Vec::new(),
                timeout_ms: 10_000,
                on_failure: OnFailure::Deny,
            },
        ]
        .into_iter()
        // The isolation guard names Claude Code only (ADR 0322).
        .filter(|h| matches!(host, Host::ClaudeCode) || h.route != "t.isolation")
        .collect(),
    }
}
