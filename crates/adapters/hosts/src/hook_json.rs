//! The hook protocol Claude Code and Codex share: a JSON object on stdin with `hook_event_name`, `session_id`, `cwd`,
//! `tool_name`, `tool_input`, `tool_response` and `prompt`; and an answer as `hookSpecificOutput`, a top-level
//! `decision: "block"` or a `systemMessage`.

use serde_json::Value;
use specforge_ops::host::{HookAnswer, HookReadError, Host, HostHookEvent, HostReply, ToolUse};
use specforge_protocol_types::{Decision, HookEvent, ToolClass};

/// `stdin` read as `host`'s event; `class` names the class of a tool the host names.
pub fn read(
    stdin: &[u8],
    host: Host,
    class: fn(&str) -> ToolClass,
) -> Result<HostHookEvent, HookReadError> {
    let label = host.as_str();
    let v: Value = serde_json::from_slice(stdin)
        .map_err(|e| HookReadError(format!("{label}'s hook input is not JSON: {e}")))?;
    if !v.is_object() {
        return Err(HookReadError(format!(
            "{label}'s hook input is not an object"
        )));
    }
    let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    let event = match text("hook_event_name").as_deref() {
        None => HookEvent::Statusline,
        Some("PreToolUse") => HookEvent::PreTool,
        Some("PostToolUse") => HookEvent::PostTool,
        Some("UserPromptSubmit") => HookEvent::UserPrompt,
        Some("SessionStart") => HookEvent::SessionStart,
        Some("Stop") => HookEvent::Stop,
        Some("SubagentStart") => HookEvent::SubagentStart,
        Some("SubagentStop") => HookEvent::SubagentStop,
        Some("PreCompact") => HookEvent::PreCompact,
        Some(other) => {
            return Err(HookReadError(format!(
                "{label} event '{other}' is not one a hook handles"
            )));
        }
    };
    let tool = text("tool_name").map(|name| ToolUse {
        class: class(&name),
        name,
        input: v.get("tool_input").cloned().unwrap_or(Value::Null),
    });
    Ok(HostHookEvent {
        event,
        host: label.into(),
        cwd: text("cwd"),
        session: text("session_id"),
        tool,
        prompt: text("prompt"),
        response: v.get("tool_response").cloned(),
        status: (event == HookEvent::Statusline).then(|| v.clone()),
    })
}

/// The neutral `answer` to `event` in this protocol.
pub fn answer(event: &HostHookEvent, answer: &HookAnswer) -> HostReply {
    if event.event == HookEvent::Statusline {
        return HostReply {
            stdout: answer
                .statusline
                .as_ref()
                .map(|s| format!("{s}\n"))
                .unwrap_or_default(),
            ..HostReply::default()
        };
    }
    let name = match event.event {
        HookEvent::PreTool | HookEvent::PreCommit => "PreToolUse",
        HookEvent::PostTool => "PostToolUse",
        HookEvent::UserPrompt => "UserPromptSubmit",
        HookEvent::SessionStart => "SessionStart",
        HookEvent::Stop => "Stop",
        HookEvent::SubagentStart => "SubagentStart",
        HookEvent::SubagentStop => "SubagentStop",
        _ => "",
    };
    let mut out = serde_json::Map::new();
    let mut specific = serde_json::Map::new();
    match answer.decision {
        Some(Decision::Deny | Decision::Ask) if name == "PreToolUse" => {
            let word = if answer.decision == Some(Decision::Ask) {
                "ask"
            } else {
                "deny"
            };
            specific.insert("permissionDecision".into(), word.into());
            if let Some(r) = &answer.reason {
                specific.insert("permissionDecisionReason".into(), r.clone().into());
            }
        }
        Some(Decision::Deny) => {
            out.insert("decision".into(), "block".into());
            if let Some(r) = &answer.reason {
                out.insert("reason".into(), r.clone().into());
            }
        }
        _ => {}
    }
    if let Some(c) = answer
        .context
        .as_ref()
        .filter(|_| !matches!(name, "" | "Stop" | "SubagentStop"))
    {
        specific.insert("additionalContext".into(), c.clone().into());
    }
    if !specific.is_empty() {
        specific.insert("hookEventName".into(), name.into());
        out.insert("hookSpecificOutput".into(), Value::Object(specific));
    }
    if let Some(n) = &answer.notice {
        out.insert("systemMessage".into(), n.clone().into());
    }
    HostReply {
        stdout: if out.is_empty() {
            String::new()
        } else {
            format!("{}\n", Value::Object(out))
        },
        ..HostReply::default()
    }
}
