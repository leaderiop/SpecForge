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
    /// The call's lifetime, for the facts later hooks borrow (the graph).
    pub call: std::marker::PhantomData<&'a ()>,
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
                    in_spec_root: p.in_spec_root,
                    exists: p.exists,
                    bytes: p.bytes,
                    new_bytes: p.new_bytes,
                })
                .collect(),
            response: f.response.clone(),
            call: std::marker::PhantomData,
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
