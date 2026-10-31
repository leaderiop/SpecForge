//! The agent host a prompt is rendered for (ADR 0291): the one list of hosts, read from what MCP's `initialize`
//! names (`clientInfo.name`) or from a `--host` flag, and sent to a prompt export as its `host` (and, later, to
//! the files a hook installs for it).

pub mod diagnose;
pub mod drift;
pub mod glob;
pub mod install;
pub mod marker;
pub mod owned;
pub mod port;
pub mod record;
pub mod render;
pub mod shell;
pub mod state;

pub use owned::{config_path, layout_dir, link_in, owned_path};
pub use port::{
    HOOK_MARK, HookAnswer, HookReadError, HostHookEvent, HostRenderer, HostReply, HostTree,
    MISSING_GUARD, Merged, MergedEntry, MergedFile, ModelChoice, NotRendered, OwnedFile, Rendered,
    RenderedAgent, RenderedHook, RenderedSkill, RenderedSlash, SlashCall, Source, ToolUse,
};

/// A host an extension renders for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Host {
    ClaudeCode,
    Codex,
    Cursor,
}

impl Host {
    /// Every host, in the order help lists them.
    pub const ALL: [Host; 3] = [Host::ClaudeCode, Host::Codex, Host::Cursor];

    /// The host an MCP client names itself as (`clientInfo.name`); `None` for a client this does not know.
    pub fn from_client(name: &str) -> Option<Host> {
        let name = name.to_ascii_lowercase();
        if name.starts_with("claude-code") {
            Some(Host::ClaudeCode)
        } else if name.starts_with("codex") {
            Some(Host::Codex)
        } else if name.starts_with("cursor") {
            Some(Host::Cursor)
        } else {
            None
        }
    }

    /// The host a `--host` flag or a `host` argument spells, exactly as [`Self::as_str`] does.
    pub fn from_flag(flag: &str) -> Option<Host> {
        Host::ALL.into_iter().find(|h| h.as_str() == flag)
    }

    /// The name a prompt export is sent and a flag spells.
    pub fn as_str(self) -> &'static str {
        match self {
            Host::ClaudeCode => "claude-code",
            Host::Codex => "codex",
            Host::Cursor => "cursor",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Host;

    #[test]
    fn a_client_is_read_to_its_host_and_a_flag_spells_it() {
        assert_eq!(Host::from_client("claude-code"), Some(Host::ClaudeCode));
        assert_eq!(Host::from_client("codex-mcp-client"), Some(Host::Codex));
        assert_eq!(Host::from_client("Cursor"), Some(Host::Cursor));
        assert_eq!(Host::from_client("vscode"), None);
        for host in Host::ALL {
            assert_eq!(Host::from_flag(host.as_str()), Some(host));
        }
        assert_eq!(Host::from_flag("emacs"), None);
    }
}
