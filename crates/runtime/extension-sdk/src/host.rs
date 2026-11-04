//! Calling the `specforge:host` ports (ADR 0335 D4): one client per port, in the protocol's types, over the one WIT
//! package this crate ships (`wit/host.wit`). A command, an MCP tool or resource and a collector may call them,
//! inside what their extension's grant permits; any other call kind is answered [`Denial::CallKind`].
//!
//! A guest imports only the interfaces it calls: an extension that calls none imports none and needs no capability
//! (ADR 0335 D5), so a port is used by calling it and declared with [`crate::Capability`]. In a component each
//! function is the import; in process (an extension's own tests) it is the host [`in_process::serve`] puts on the
//! thread, and with none it answers what a pure call gets.

mod convert;
pub mod in_process;

pub use specforge_protocol_types::{
    Denial, EnvVar, Failure, FileSize, FsContent, FsEntry, FsEntryKind, FsListing, GitBlob,
    GitBlobs, GitChange, GitEntry, GitEntryKind, GitRepo, GitStatus, GitTreeEntry, Header,
    HostError, HttpRequest, HttpResponse, ProcessOutput, ProcessRequest,
};

/// Whether a call reaches the import (a component) or the thread's in-process host.
const IN_COMPONENT: bool = cfg!(target_arch = "wasm32");

/// What wit-bindgen generates from `wit/host.wit`, on every target so the conversions compile and are tested natively.
/// Outside `wasm32` an import is never called: [`IN_COMPONENT`] sends the call to the in-process host.
#[allow(
    unsafe_code,
    reason = "the canonical ABI of the imports wit-bindgen generates"
)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/host.wit",
        world: "ports",
    });
}

use bindings::specforge::host as wit;

/// The HTTP port.
pub mod http {
    use super::*;

    /// One request, its redirects followed by the host. An answer of any status is `Ok`.
    pub fn send(request: &HttpRequest) -> Result<HttpResponse, HostError> {
        if !IN_COMPONENT {
            return in_process::with(|host| host.http_send(request));
        }
        wit::http::send(&request.into())
            .map(Into::into)
            .map_err(Into::into)
    }
}

/// The file port: reads under a named root.
pub mod fs {
    use super::*;

    /// A path's kind, size and modification time; none when it does not exist.
    pub fn stat(root: &str, path: &str) -> Result<Option<FsEntry>, HostError> {
        if !IN_COMPONENT {
            return in_process::with(|host| host.fs_stat(root, path));
        }
        wit::fs::stat(root, path)
            .map(|entry| entry.map(Into::into))
            .map_err(Into::into)
    }

    /// At most `max_bytes` of a file (0: as many as the call may have).
    pub fn read(root: &str, path: &str, max_bytes: u64) -> Result<FsContent, HostError> {
        if !IN_COMPONENT {
            return in_process::with(|host| host.fs_read(root, path, max_bytes));
        }
        wit::fs::read(root, path, max_bytes)
            .map(Into::into)
            .map_err(Into::into)
    }

    /// The files under a path matching `glob` (every file when `None`), at most `max_entries`.
    pub fn walk(
        root: &str,
        path: &str,
        glob: Option<&str>,
        max_entries: u32,
    ) -> Result<FsListing, HostError> {
        if !IN_COMPONENT {
            return in_process::with(|host| host.fs_walk(root, path, glob, max_entries));
        }
        wit::fs::walk(root, path, glob, max_entries)
            .map(Into::into)
            .map_err(Into::into)
    }
}

/// The process port: a program a `process` capability declares, in the project's directory.
pub mod process {
    use super::*;

    /// One run. A run that ended is `Ok`, whatever its exit code.
    pub fn run(request: &ProcessRequest) -> Result<ProcessOutput, HostError> {
        if !IN_COMPONENT {
            return in_process::with(|host| host.process_run(request));
        }
        wit::process::run(&request.into())
            .map(Into::into)
            .map_err(Into::into)
    }
}

/// The git port: reads of the project's repository.
pub mod git {
    use super::*;

    /// Where the project's repository is; none when it is in no repository.
    pub fn locate() -> Result<Option<GitRepo>, HostError> {
        if !IN_COMPONENT {
            return in_process::with(|host| host.git_locate());
        }
        wit::git::locate()
            .map(|repo| repo.map(Into::into))
            .map_err(Into::into)
    }

    /// The commit `revision` names; none when it names none.
    pub fn commit(revision: &str) -> Result<Option<String>, HostError> {
        if !IN_COMPONENT {
            return in_process::with(|host| host.git_commit(revision));
        }
        wit::git::commit(revision).map_err(Into::into)
    }

    /// The nearest common ancestor of two commits.
    pub fn merge_base(a: &str, b: &str) -> Result<Option<String>, HostError> {
        if !IN_COMPONENT {
            return in_process::with(|host| host.git_merge_base(a, b));
        }
        wit::git::merge_base(a, b).map_err(Into::into)
    }

    /// Every entry under `dir` (from the repository's top) at `commit`.
    pub fn tree(commit: &str, dir: &str) -> Result<Vec<GitTreeEntry>, HostError> {
        if !IN_COMPONENT {
            return in_process::with(|host| host.git_tree(commit, dir));
        }
        wit::git::tree(commit, dir)
            .map(|entries| entries.into_iter().map(Into::into).collect())
            .map_err(Into::into)
    }

    /// The sizes of the named files at `commit`, in the order asked.
    pub fn sizes(commit: &str, paths: &[String]) -> Result<Vec<FileSize>, HostError> {
        if !IN_COMPONENT {
            return in_process::with(|host| host.git_sizes(commit, paths));
        }
        wit::git::sizes(commit, paths)
            .map(|sized| sized.into_iter().map(Into::into).collect())
            .map_err(Into::into)
    }

    /// The named files at `commit`, in the order asked.
    pub fn blobs(commit: &str, paths: &[String]) -> Result<GitBlobs, HostError> {
        if !IN_COMPONENT {
            return in_process::with(|host| host.git_blobs(commit, paths));
        }
        wit::git::blobs(commit, paths)
            .map(Into::into)
            .map_err(Into::into)
    }

    /// The branch `HEAD` is on: empty when it is detached.
    pub fn branch() -> Result<String, HostError> {
        if !IN_COMPONENT {
            return in_process::with(|host| host.git_branch());
        }
        wit::git::branch().map_err(Into::into)
    }

    /// The worktree's typed status.
    pub fn status() -> Result<GitStatus, HostError> {
        if !IN_COMPONENT {
            return in_process::with(|host| host.git_status());
        }
        wit::git::status().map(Into::into).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests;
