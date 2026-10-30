//! Test support: the calls an upstream suite made to its hook, recorded by `gsd/parity/record-hook-cases.cjs`, and the
//! host-neutral facts of a Claude Code payload.

use serde_json::Value;

use super::facts::Facts;

/// One recorded call: the case it belongs to, the payload upstream's hook was given, and what it answered.
pub struct Call {
    pub path: String,
    /// The payload, when it was recorded and is JSON.
    pub input: Option<Value>,
    pub status: i64,
    pub out: Value,
}

pub fn calls(recording: &str) -> Vec<Call> {
    let all: Vec<Value> = serde_json::from_str(recording).expect("a recording is JSON");
    all.into_iter()
        .map(|c| {
            let mut path: Vec<String> = c["describe"]
                .as_array()
                .map(|d| {
                    d.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            path.push(
                c["test"]
                    .as_str()
                    .expect("a call is made inside a test")
                    .to_string(),
            );
            Call {
                path: path.join(" > "),
                input: c["input"]
                    .as_str()
                    .and_then(|s| serde_json::from_str(s).ok()),
                status: c["status"].as_i64().unwrap_or(-1),
                out: c["stdout"].clone(),
            }
        })
        .collect()
}

/// The host-neutral class of a tool Claude Code names.
pub fn claude_class(name: &str) -> &'static str {
    match name {
        "Read" => "read",
        "Write" => "write",
        "Edit" | "MultiEdit" | "NotebookEdit" => "edit",
        "Bash" => "shell",
        "Glob" | "Grep" => "search",
        "Agent" | "Task" => "agent",
        "WebFetch" => "web_fetch",
        "WebSearch" => "web_search",
        _ => "",
    }
}

/// The facts of a Claude Code tool-use payload. `None` for a tool no class covers (another runtime's vocabulary).
pub fn claude_facts(payload: &Value) -> Option<Facts<'static>> {
    let name = payload["tool_name"].as_str()?;
    let class = claude_class(name);
    (!class.is_empty()).then(|| Facts {
        host: "claude-code".into(),
        class: class.into(),
        tool: name.into(),
        input: payload["tool_input"].clone(),
        ..Facts::default()
    })
}

/// Replays a recording against its suite: `check` judges one call (`None`: it cannot be replayed, `Some(Err)`: our hook
/// answers differently). The result is the static cases every one of whose calls was replayed and agreed; a
/// disagreement panics with the first twenty.
pub fn replay(
    suite: &str,
    recording: &str,
    check: impl Fn(&Call) -> Option<Result<(), String>>,
) -> Vec<String> {
    let all = super::accounting::cases(suite);
    let mut seen: std::collections::BTreeMap<String, (usize, usize)> =
        std::collections::BTreeMap::new();
    let mut wrong = Vec::new();
    for call in calls(recording) {
        let case = super::accounting::case_of(&all, &call.path);
        let entry = seen.entry(case.path.clone()).or_default();
        match check(&call) {
            None => entry.1 += 1,
            Some(Ok(())) => entry.0 += 1,
            Some(Err(why)) => wrong.push(format!("{}: {why}", call.path)),
        }
    }
    assert!(
        wrong.is_empty(),
        "{} calls differ:\n{}",
        wrong.len(),
        wrong
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    seen.into_iter()
        .filter(|(_, (ok, skipped))| *ok > 0 && *skipped == 0)
        .map(|(path, _)| path)
        .collect()
}
