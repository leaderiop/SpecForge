//! The bound of an MCP request (ADR 0092, ADR 0104).
//!
//! The rule is `specforge_ops::Containment`: it resolves a name the way the file system would and
//! says whether it lands inside a directory, and `From<Outside> for OpError` words the refusal. This
//! module builds the bound of a request.

use std::path::{Path, PathBuf};

use crate::state::McpState;
use specforge_ops::Containment;

/// The bound of a request: the served project's root; while none is served, the server's root;
/// unbounded when the server has neither (a server made with no root names nothing to stay inside).
pub(crate) fn bound(state: &McpState) -> Containment {
    match state.project_root().or(state.root.as_deref()) {
        Some(root) => Containment::within(root),
        None => Containment::unbounded(),
    }
}

/// Where `path` really is: absolute, every symlink followed, a missing tail joined on.
pub(crate) fn landed(path: &Path) -> PathBuf {
    Containment::unbounded()
        .resolve(path)
        .map_or_else(|_| path.to_path_buf(), |held| held.real)
}
