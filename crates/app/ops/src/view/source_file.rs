//! A spec file a request names (a typed create's `file`, outline's `file`, the fixes' `file_path`): read into its
//! source key, the path from the project root that the compile and the editor name it by, or refused once
//! (ADR 0195). Every spelling of one file is that file; the key is what the ledger, the LSP and a span's `file` say.
//! A refusal names the file as the request spelled it.

use std::path::{Component, Path, PathBuf};

use specforge_common::SourceRoot;

use super::ProjectView;
use crate::{OpError, OpErrorKind};

/// A create's file is not a project source.
pub const NOT_A_SOURCE: &str = "not_a_source";

pub(crate) const FILE_NOT_FOUND: &str = "file_not_found";

/// `named`'s source key in `sources`, with `.` and doubled separators gone; `None` when it leaves the project root
/// (a `..`, or an absolute path elsewhere). A project with no spec root (one assembled in memory) keys its sources
/// by whatever path they were given, so an absolute path is a key there.
fn key_of(sources: &SourceRoot, named: &str) -> Option<String> {
    // A `..` anywhere leaves the project root's reading of the name, whatever it lands on.
    if Path::new(named)
        .components()
        .any(|c| c == Component::ParentDir)
    {
        return None;
    }
    let key = sources.key(&sources.base().join(named)).into_string();
    let mut rebuilt = PathBuf::new();
    for component in Path::new(&key).components() {
        match component {
            Component::Normal(name) => rebuilt.push(name),
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) if sources.base().as_os_str().is_empty() => {
                rebuilt.push(component.as_os_str());
            }
            _ => return None,
        }
    }
    Some(rebuilt.to_string_lossy().into_owned())
}

fn outside(named: &str) -> OpError {
    OpError::new(
        OpErrorKind::InvalidInput,
        NOT_A_SOURCE,
        format!("'{named}' leaves the project root: a spec file is named from the project root"),
    )
    .with_file(named)
}

impl ProjectView<'_> {
    /// The source key of the compiled source `named` names. A spelling that leaves the spec root is `not_a_source`
    /// (kind `InvalidInput`); any other that is none of the compile's sources (missing, a directory, not a `.spec`
    /// file, excluded) is `file_not_found`. Reads no file.
    pub fn source(&self, named: &str) -> Result<String, OpError> {
        let key = key_of(self.env().sources(), named).ok_or_else(|| outside(named))?;
        if self.project().source_keys().contains(key.as_str()) {
            return Ok(key);
        }
        Err(OpError::new(
            OpErrorKind::FileNotFound,
            FILE_NOT_FOUND,
            format!("'{named}' is none of this project's sources"),
        )
        .with_file(named))
    }

    /// The key of a file a typed edit may create or change: a compiled source, or a new file discovery would keep.
    /// `not_a_source` (kind `InvalidInput`) otherwise.
    pub fn source_place(&self, named: &str) -> Result<String, OpError> {
        let key = key_of(self.env().sources(), named).ok_or_else(|| outside(named))?;
        if self.project().source_keys().contains(key.as_str()) {
            return Ok(key);
        }
        if self.env().sources().holds_key(&key) {
            return Ok(key);
        }
        Err(OpError::new(
            OpErrorKind::InvalidInput,
            NOT_A_SOURCE,
            format!(
                "'{named}' is not a source of this project (a .spec file inside the spec root, not excluded)"
            ),
        )
        .with_file(named))
    }
}
