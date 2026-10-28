//! What `specforge doctor` says of the host files (ADR 0292): a file install wrote that a hand edit has changed, and an
//! install that is stale against the project. Only a host with a file install marked is judged: a directory a person
//! wrote by hand is none of install's business.

use super::drift::{self, RefusalCode};
use super::install::ContainedHostDisk;
use super::port::{HostRenderer, HostTree};
use super::render::rendered;
use crate::Caller;
use crate::view::ProjectView;

/// The finding code of a marked file changed since install wrote it.
pub const FILE_EDITED: &str = "host_file_edited";
/// The finding code of an install that would write something else now.
pub const INSTALL_STALE: &str = "host_install_stale";

/// One thing the doctor reports about a host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostFinding {
    pub code: &'static str,
    /// The file (project-relative); none for a whole install.
    pub path: Option<String>,
    pub message: String,
    pub remediation: String,
}

/// The findings about the hosts `renderers` serve, over the project `view` reads.
pub fn findings(
    view: &ProjectView<'_>,
    renderers: &[&dyn HostRenderer],
    caller: Caller<'_>,
) -> Vec<HostFinding> {
    if view.project_root().is_err() {
        return Vec::new();
    }
    let sources = view.env().sources().clone();
    let disk = ContainedHostDisk {
        caller,
        sources: &sources,
    };
    let mut out = Vec::new();
    for renderer in renderers {
        let host = renderer.host();
        // What uninstall would do names what install marked: removed if intact, refused if edited.
        let marked = drift::plan(*renderer, &HostTree::default(), &disk, false);
        let edited: Vec<_> = marked
            .refused
            .iter()
            .filter(|r| r.code == RefusalCode::EditedFile)
            .collect();
        // An install that wrote only config entries (the MCP server and the spec guard) is an install: anything
        // taking its marks away, a file or an entry, says so.
        let installed = !edited.is_empty() || marked.changes().next().is_some();
        if !installed {
            continue;
        }
        let reinstall =
            specforge_common::remedy::command("`specforge install {} --force`", &[host.as_str()]);
        for refusal in edited {
            out.push(HostFinding {
                code: FILE_EDITED,
                path: Some(refusal.path.clone()),
                message: refusal.message.clone(),
                remediation: format!("keep the edit out of it, or run {reinstall} to overwrite it"),
            });
        }
        let Ok(contributions) = rendered(view, host) else {
            continue;
        };
        let wanted = drift::plan(*renderer, &renderer.render(&contributions), &disk, false);
        let differ = wanted.changes().count();
        if differ > 0 {
            out.push(HostFinding {
                code: INSTALL_STALE,
                path: None,
                message: format!(
                    "{differ} {} file(s) differ from what the project's extensions contribute now",
                    host.as_str()
                ),
                remediation: format!(
                    "run {}",
                    specforge_common::remedy::command("`specforge install {}`", &[host.as_str()])
                ),
            });
        }
    }
    out
}
