//! The native seam of [`crate::host`]: what a port call answers outside a component. The host's in-process test
//! runtime and an extension's own tests [`serve`] a host on the thread for the length of a call; no host, and a port
//! nobody overrode, answers what a pure call gets in a component.

use std::cell::RefCell;
use std::rc::Rc;

use super::{
    Denial, FsContent, FsEntry, FsListing, GitBlobs, GitRepo, GitSized, GitStatus, GitTreeEntry,
    HostError, HttpRequest, HttpResponse, ProcessOutput, ProcessRequest,
};

/// What answers a port call in process. A method not given answers [`no_port`].
#[expect(unused_variables, reason = "each default answers whatever is asked")]
pub trait InProcessHost {
    fn http_send(&self, request: &HttpRequest) -> Result<HttpResponse, HostError> {
        Err(no_port())
    }
    fn fs_stat(&self, root: &str, path: &str) -> Result<Option<FsEntry>, HostError> {
        Err(no_port())
    }
    fn fs_read(&self, root: &str, path: &str, max_bytes: u64) -> Result<FsContent, HostError> {
        Err(no_port())
    }
    fn fs_walk(
        &self,
        root: &str,
        path: &str,
        glob: Option<&str>,
        max_entries: u32,
    ) -> Result<FsListing, HostError> {
        Err(no_port())
    }
    fn process_run(&self, request: &ProcessRequest) -> Result<ProcessOutput, HostError> {
        Err(no_port())
    }
    fn git_locate(&self) -> Result<Option<GitRepo>, HostError> {
        Err(no_port())
    }
    fn git_commit(&self, revision: &str) -> Result<Option<String>, HostError> {
        Err(no_port())
    }
    fn git_merge_base(&self, a: &str, b: &str) -> Result<Option<String>, HostError> {
        Err(no_port())
    }
    fn git_tree(&self, commit: &str, dir: &str) -> Result<Vec<GitTreeEntry>, HostError> {
        Err(no_port())
    }
    fn git_sizes(&self, commit: &str, paths: &[String]) -> Result<Vec<GitSized>, HostError> {
        Err(no_port())
    }
    fn git_blobs(&self, commit: &str, paths: &[String]) -> Result<GitBlobs, HostError> {
        Err(no_port())
    }
    fn git_branch(&self) -> Result<String, HostError> {
        Err(no_port())
    }
    fn git_status(&self) -> Result<GitStatus, HostError> {
        Err(no_port())
    }
}

/// The answer outside any host: a pure call's in a component.
fn no_port() -> HostError {
    HostError::Denied(Denial::CallKind(
        "no host serves specforge:host in this process".to_string(),
    ))
}

thread_local! {
    static SERVED: RefCell<Option<Rc<dyn InProcessHost>>> = const { RefCell::new(None) };
}

/// Puts back the host a nested [`serve`] replaced, a panic in the call included.
struct Restore(Option<Rc<dyn InProcessHost>>);

impl Drop for Restore {
    fn drop(&mut self) {
        let previous = self.0.take();
        SERVED.with(|served| *served.borrow_mut() = previous);
    }
}

/// Run `call` with `host` answering this thread's port calls.
pub fn serve<T>(host: Rc<dyn InProcessHost>, call: impl FnOnce() -> T) -> T {
    let _restore = Restore(SERVED.with(|served| served.borrow_mut().replace(host)));
    call()
}

/// `call` on the host this thread serves; a call into one finds the slot free, so a host may run a guest again.
pub(super) fn with<T>(
    call: impl FnOnce(&dyn InProcessHost) -> Result<T, HostError>,
) -> Result<T, HostError> {
    match SERVED.with(|served| served.borrow().clone()) {
        Some(host) => call(host.as_ref()),
        None => Err(no_port()),
    }
}
