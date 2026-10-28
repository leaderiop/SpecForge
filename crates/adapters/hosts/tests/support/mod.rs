//! The fixture the goldens render: the `@t/core` and `@t/ui` contributions of `GsdHosts` (settings: `ui_phase` on,
//! everything else default), as the operations hand them to an adapter.

use specforge_ops::host::{
    Host, ModelChoice, Rendered, RenderedAgent, RenderedHook, RenderedSkill, RenderedSlash,
    SlashCall, Source,
};
use specforge_protocol_types::{Attribution, HookEvent};

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
    RenderedHook {
        route: format!("t.{id}"),
        description: id.into(),
        event,
        tools: tools.iter().map(|t| (*t).to_string()).collect(),
        paths: paths.iter().map(|t| (*t).to_string()).collect(),
        timeout_ms: 5_000,
    }
}

pub fn fixture(host: Host) -> Rendered {
    let classes = |c: &[&str]| c.iter().map(|t| (*t).to_string()).collect::<Vec<_>>();
    let agent =
        |ext: &str, version: &str, id: &str, description: &str, tools: &[&str]| RenderedAgent {
            source: source(ext, version, "agent", id),
            id: id.into(),
            description: description.into(),
            tools: classes(tools),
            model: ModelChoice::Inherit,
            effort: None,
            color: None,
            body: body(id, host),
            attribution: None,
        };
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
                &["read", "search"],
            ),
            agent("@t/ui", "1.0.0", "t-ui-checker", "Checks a UI", &["read"]),
        ],
        skills: vec![RenderedSkill {
            source: source("@t/core", "0.4.0", "skill", "t-ns-context"),
            id: "t-ns-context".into(),
            description: "Routes a request about the codebase".into(),
            tools: classes(&["read", "skill"]),
            argument_hint: None,
            body: body("t-ns-context", host),
            attribution: None,
        }],
        slashes: vec![
            RenderedSlash {
                source: source("@t/core", "0.4.0", "slash", "plan_phase"),
                name: "t-plan-phase".into(),
                description: "Plan the phase, then check the plans".into(),
                argument_hint: Some("<phase>".into()),
                tools: classes(&["read", "agent"]),
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
                tools: classes(&["read"]),
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
        ],
    }
}
