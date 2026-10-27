//! A plan names the revision of every file it changes (ADR 0289): the host compares it with the project it planned
//! on, and, under the write lock, with the disk, so a plan made over a text that has since changed is refused
//! (`stale_revision`) and writes nothing.

use super::STALE_REVISION;
use crate::revision::{ABSENT, of_bytes, of_file};
use crate::{Caller, OpError, OpErrorKind};
use specforge_common::SourceRoot;
use specforge_project::CompiledProject;
use std::collections::BTreeMap;

/// A plan made over `planned` of `file`, which is `current`.
pub(super) fn stale(file: &str, planned: &str, current: &str) -> OpError {
    OpError::new(
        OpErrorKind::Conflict,
        STALE_REVISION,
        format!(
            "the plan was made over {file} at {planned}, which is now {current}; nothing was written"
        ),
    )
    .with_suggestion("run the command again over the project as it is now")
    .with_data(serde_json::json!({"file": file, "planned": planned, "current": current}))
}

/// Every file of `files` is, in the compile `project` holds, at the revision the plan names.
pub(super) fn against_compile(
    project: &CompiledProject,
    files: &BTreeMap<String, String>,
) -> Result<(), OpError> {
    for (file, planned) in files {
        let current = of_file(project, file);
        if &current != planned {
            return Err(stale(file, planned, &current));
        }
    }
    Ok(())
}

/// Every file of `files` is, on disk now, at the revision the plan names: read through the caller's containment,
/// under the write lock.
pub(super) fn against_disk(
    sources: &SourceRoot,
    caller: Caller<'_>,
    files: &BTreeMap<String, String>,
) -> Result<(), OpError> {
    for (file, planned) in files {
        let held = caller.containment().resolve(&sources.path(file))?;
        let current = match held.read_if_there() {
            Ok(Some(bytes)) => of_bytes(&bytes),
            Ok(None) => ABSENT.to_string(),
            Err(e) => {
                return Err(OpError::new(
                    OpErrorKind::of_io(&e),
                    crate::FILE_UNREADABLE,
                    format!("cannot read {file} to check its revision: {e}"),
                ));
            }
        };
        if &current != planned {
            return Err(stale(file, planned, &current));
        }
    }
    Ok(())
}
