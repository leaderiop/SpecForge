//! Whether a host event is one a hook fires on (ADR 0292): the event, the tool's class and the paths its input
//! names. `specforge hook` asks it from its flags before loading anything, and again from the hook's declaration once
//! the project is loaded.

use std::path::Path;

use specforge_protocol_types::HookEvent;

use super::facts;
use crate::host::{HostHookEvent, glob};

#[derive(Debug, Clone)]
pub struct Filter {
    pub event: HookEvent,
    /// Tool classes; empty: any tool.
    pub tools: Vec<String>,
    /// Project-relative globs; empty: any path.
    pub paths: Vec<String>,
}

impl Filter {
    /// Whether `event`, read in `cwd`, is one this fires on, for the project at `root`.
    pub fn matches(&self, event: &HostHookEvent, root: &Path, cwd: &Path) -> bool {
        // A commit is a shell tool's pre event: the host has no event of its own for it.
        let wanted = event.event == self.event
            || (self.event == HookEvent::PreCommit && event.event == HookEvent::PreTool);
        if !wanted {
            return false;
        }
        if self.event == HookEvent::FileChanged {
            return event.file.as_deref().is_some_and(|file| {
                self.paths.is_empty()
                    || facts::relative_to(root, cwd, file)
                        .is_some_and(|rel| self.paths.iter().any(|g| glob::matches(g, &rel)))
            });
        }
        if !self.event.takes_tools() {
            return true;
        }
        let Some(tool) = &event.tool else {
            return false;
        };
        if self.event == HookEvent::PreCommit {
            return tool.class == "shell" && facts::commit(&tool.input).is_some();
        }
        if !self.tools.is_empty() && !self.tools.contains(&tool.class) {
            return false;
        }
        if self.paths.is_empty() {
            return true;
        }
        facts::path_names(&tool.input).iter().any(|name| {
            facts::relative_to(root, cwd, name)
                .is_some_and(|rel| self.paths.iter().any(|g| glob::matches(g, &rel)))
        })
    }
}
