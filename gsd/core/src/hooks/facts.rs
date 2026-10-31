//! What a hook is told, as a struct of this crate's own (D-r44-2): `HookInput` and its facts are not nameable from
//! an extension, and a struct built in one place lets the unit tests build a host's event without a host.

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

/// A path the tool's input names, with what the host computed about it.
#[derive(Debug, Clone, Default)]
pub struct PathFact {
    pub path: String,
    pub in_project: bool,
    pub in_spec_root: bool,
    pub exists: bool,
    pub bytes: Option<u64>,
    pub new_bytes: Option<u64>,
    /// The path made absolute against the hook's `cwd`, links followed as far as it exists (ADR 0320).
    pub real: String,
    /// `reads_git`, outside the project: the top directory of the repository the real path is in.
    pub toplevel: Option<String>,
    /// `reads_git`, outside the project: a component of the real path is `.git`.
    pub in_git_dir: bool,
}

/// What the host read of the repository the hook's `cwd` is in (`reads_git`, ADR 0320).
#[derive(Debug, Clone, Default)]
pub struct Git {
    pub toplevel: String,
    pub git_dir: String,
    /// Empty when `HEAD` is detached.
    pub branch: String,
}

/// The tool and the facts of a hook call.
#[derive(Debug, Clone, Default)]
pub struct Facts<'a> {
    pub host: String,
    /// The directory the host ran in.
    pub cwd: String,
    /// The tool's host-neutral class (`write`, `shell`, `agent`) and the host's name for it (`Write`, `Bash`).
    pub class: String,
    pub tool: String,
    pub input: Value,
    pub paths: Vec<PathFact>,
    /// `post_tool`: the host's tool response.
    pub response: Option<Value>,
    /// The host's clock when the hook fired (RFC 3339 UTC); empty when it passed none.
    pub now: String,
    /// `user_prompt`: what the user typed.
    pub prompt: Option<String>,
    /// `pre_commit`: the message `-m` gives; none when the commit takes it from elsewhere.
    pub commit_message: Option<String>,
    /// A hook that declares `reads_state`: the session's state (none: no session, or none sent).
    pub session_state: Option<serde_json::Map<String, Value>>,
    /// The host's session, if it names one.
    pub session: Option<String>,
    /// `statusline`: the host's status JSON.
    pub status: Option<Value>,
    /// A hook that declares `reads_state`: the project's state.
    pub project_state: serde_json::Map<String, Value>,
    /// A hook that declares `reads_graph`.
    pub graph: Option<&'a CommandGraph>,
    /// A hook that declares `reads_git`: none outside a repository.
    pub git: Option<Git>,
}

impl<'a> Facts<'a> {
    /// The facts of a hook call.
    pub fn of(call: &HookCall<'a>) -> Self {
        let f = call.facts();
        let tool = f.tool.as_ref();
        Facts {
            host: f.host.clone(),
            cwd: f.cwd.clone(),
            class: tool.map(|t| t.class.clone()).unwrap_or_default(),
            tool: tool.map(|t| t.name.clone()).unwrap_or_default(),
            input: tool.map(|t| t.input.clone()).unwrap_or_default(),
            paths: f
                .paths
                .iter()
                .map(|p| PathFact {
                    path: p.path.clone(),
                    in_project: p.in_project,
                    // The kernel calls every project path "in the spec root" when the spec root is the project (`.`), and
                    // a hook cannot tell that case from `spec/`: only a `.spec` file is one then (REVIEW-r44 M1).
                    in_spec_root: p.in_spec_root && p.path.ends_with(".spec"),
                    exists: p.exists,
                    bytes: p.bytes,
                    new_bytes: p.new_bytes,
                    real: p.real.clone(),
                    toplevel: p.toplevel.clone(),
                    in_git_dir: p.in_git_dir,
                })
                .collect(),
            response: f.response.clone(),
            now: call.now().to_string(),
            prompt: f.prompt.clone(),
            commit_message: f.commit.as_ref().and_then(|c| c.message.clone()),
            session_state: f.state.as_ref().and_then(|s| s.session.clone()),
            session: f.session.clone(),
            status: f.status.clone(),
            project_state: f
                .state
                .as_ref()
                .map(|s| s.project.clone())
                .unwrap_or_default(),
            graph: f.graph.as_ref(),
            git: f.git.as_ref().map(|g| Git {
                toplevel: g.toplevel.clone(),
                git_dir: g.git_dir.clone(),
                branch: g.branch.clone(),
            }),
        }
    }

    /// The tool input's string at `key`.
    pub fn input_str(&self, key: &str) -> Option<&str> {
        self.input.get(key)?.as_str()
    }

    /// The last path component of `path`.
    pub fn base(path: &str) -> &str {
        path.rsplit(['/', '\\']).next().unwrap_or(path)
    }
}

/// JavaScript's truthiness of a JSON value.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}
