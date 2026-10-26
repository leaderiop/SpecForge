//! A spec file a request names (a typed create's `file`, outline's `file`, the fixes' `file_path`): read into its
//! source key, the path from the project root that the compile and the editor name it by, or refused once
//! (ADR 0195). Every spelling of one file is that file; the key is what the ledger, the LSP and a span's `file` say.
//! A source under a spec root outside the project is named by its absolute key or any spelling of it from the
//! project root (ADR 0246). A refusal names the file as the request spelled it. A source-code path a request names
//! (infer_session's `source_file`, find_spec_for_source's `file_path`) is read the same way, beside it (ADR 0253).

use std::path::{Component, Path};

use specforge_common::SourceRoot;

use super::ProjectView;
use crate::source_key::{SourceKey, Unkeyed};
use crate::{OpError, OpErrorKind};

/// A create's file is not a project source.
pub const NOT_A_SOURCE: &str = "not_a_source";

/// A source-code path a request names whose key lies outside the project root, or that names nothing (kind
/// `InvalidInput`).
pub const NOT_A_SOURCE_PATH: &str = "not_a_source_path";

pub(crate) const FILE_NOT_FOUND: &str = "file_not_found";

/// The one sentence of a named path whose key lies outside the roots its reader reads against (ADR 0253).
fn lies_outside(named: &str, roots: &str, what: &str) -> String {
    format!("'{named}' lies outside {roots}: {what} is named from the project root")
}

/// The key `named` mints, or `None` when it lies outside both the project root and the spec root. A project with no
/// root (one assembled in memory) keys its sources by the absolute path they were given.
fn key_of(sources: &SourceRoot, named: &str) -> Option<String> {
    let key = crate::source_key::minted(sources.base(), named);
    let path = Path::new(&key);
    let inside = if path.is_absolute() {
        sources.base().as_os_str().is_empty()
            || (sources.prefix().is_absolute() && path.starts_with(sources.prefix()))
    } else {
        !path.components().any(|c| c == Component::ParentDir)
    };
    inside.then_some(key)
}

fn outside(named: &str) -> OpError {
    OpError::new(
        OpErrorKind::InvalidInput,
        NOT_A_SOURCE,
        lies_outside(named, "the project root and the spec root", "a spec file"),
    )
    .with_file(named)
}

impl ProjectView<'_> {
    /// The source key of the compiled source `named` names. A spelling whose key lies outside the project root and the spec root is `not_a_source`
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
    /// `not_a_source` (kind `InvalidInput`) otherwise, and for a name equal to a compiled source ignoring case, naming
    /// that source: two files differing only in case cannot be checked out together on every OS.
    pub fn source_place(&self, named: &str) -> Result<String, OpError> {
        let key = key_of(self.env().sources(), named).ok_or_else(|| outside(named))?;
        if self.project().source_keys().contains(key.as_str()) {
            return Ok(key);
        }
        let folded = key.to_lowercase();
        if let Some(source) = self
            .project()
            .source_keys()
            .into_iter()
            .find(|k| k.to_lowercase() == folded)
        {
            return Err(OpError::new(
                OpErrorKind::InvalidInput,
                NOT_A_SOURCE,
                format!(
                    "'{named}' differs from the source '{source}' only in case: name that file as '{source}', or give the new file a name no source has"
                ),
            )
            .with_file(named));
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

    /// The key of the source-code file a request names (infer_session's `source_file`), read from the project root as a
    /// spec file is: any spelling of one file is that file, its absolute path under the root included. A key outside
    /// the project root, or one that names nothing, is `not_a_source_path` (kind `InvalidInput`, no file). Reads no
    /// file: where a link lands is judged where the file is opened.
    pub(crate) fn source_path(&self, named: &str) -> Result<SourceKey, OpError> {
        self.project_root()?;
        SourceKey::read(self.env().sources().base(), named).map_err(|why| unkeyed(why, named))
    }

    /// [`Self::source_path`] for a directory (a source root, find_spec_for_source's `file_path`): `.` and the
    /// project root's own path are the project root.
    pub(crate) fn source_root(&self, named: &str) -> Result<SourceKey, OpError> {
        self.project_root()?;
        SourceKey::read_root(self.env().sources().base(), named).map_err(|why| unkeyed(why, named))
    }
}

/// The refusal of a named source-code path that is no key.
fn unkeyed(why: Unkeyed, named: &str) -> OpError {
    match why {
        Unkeyed::Nothing => names_no_file(named),
        Unkeyed::Outside => OpError::new(
            OpErrorKind::InvalidInput,
            NOT_A_SOURCE_PATH,
            lies_outside(named, "the project root", "a source file"),
        ),
    }
}

fn names_no_file(named: &str) -> OpError {
    OpError::new(
        OpErrorKind::InvalidInput,
        NOT_A_SOURCE_PATH,
        format!("'{named}' names no file: a source file is named from the project root"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Viewed;
    use specforge_project::testing::Assembled;

    fn keyed(project: &Assembled, named: &str) -> Result<String, OpError> {
        project
            .view()
            .source_path(named)
            .map(|key| key.as_str().to_string())
    }

    #[specforge_test_macros::test(
        behavior = "read_a_named_source_path",
        verify = "every spelling of a source-code path under the project root reads as its key"
    )]
    fn every_spelling_under_the_root_is_its_key() {
        let project = Assembled::new();
        let root = project.root().to_path_buf();
        let canonical = std::fs::canonicalize(&root).unwrap();
        let absolute = root.join("src/a.rs").display().to_string();
        let behind = canonical.join("src/a.rs").display().to_string();
        for named in [
            "src/a.rs",
            "./src//a.rs",
            "src\\a.rs",
            "src/./a.rs",
            "src/../src/a.rs",
            absolute.as_str(),
            behind.as_str(),
        ] {
            assert_eq!(keyed(&project, named).unwrap(), "src/a.rs", "{named}");
        }
        let view = project.view();
        for named in [
            ".",
            "./",
            root.to_str().unwrap(),
            canonical.to_str().unwrap(),
        ] {
            assert_eq!(view.source_root(named).unwrap().as_str(), ".", "{named}");
        }
        assert_eq!(view.source_root("./src/").unwrap().as_str(), "src");
    }

    #[specforge_test_macros::test(
        behavior = "read_a_named_source_path",
        verify = "a source-code path whose key lies outside the project root is refused in the named spec file's sentence"
    )]
    fn a_spelling_that_leaves_is_refused_in_the_spec_files_sentence() {
        let project = Assembled::new();
        let other = tempfile::TempDir::new().unwrap();
        let elsewhere = other.path().join("a.rs").display().to_string();
        let beyond = format!("{}/../x", project.root().display());
        let view = project.view();
        for named in [
            "../x",
            "/x",
            "\\x",
            "C:\\x",
            "c:/x",
            elsewhere.as_str(),
            beyond.as_str(),
        ] {
            for error in [
                view.source_path(named).unwrap_err(),
                view.source_root(named).unwrap_err(),
            ] {
                assert_eq!(error.kind, OpErrorKind::InvalidInput, "{named}");
                assert_eq!(error.code, NOT_A_SOURCE_PATH, "{named}");
                assert_eq!(
                    error.message,
                    lies_outside(named, "the project root", "a source file"),
                    "{named}"
                );
                assert_eq!(error.file(), None, "{named}: it is no spec file");
            }
        }
        // The spec reader words the same sentence for its two roots.
        let spec = view.source("../x.spec").unwrap_err();
        assert_eq!(
            spec.message,
            lies_outside(
                "../x.spec",
                "the project root and the spec root",
                "a spec file"
            )
        );
    }

    #[specforge_test_macros::test(
        behavior = "read_a_named_source_path",
        verify = "a source-code path that names nothing is refused"
    )]
    fn a_spelling_of_nothing_is_refused() {
        let project = Assembled::new();
        for named in ["", ".", "./"] {
            let error = keyed(&project, named).unwrap_err();
            assert_eq!(error.code, NOT_A_SOURCE_PATH, "{named:?}");
            assert_eq!(
                error.message,
                format!("'{named}' names no file: a source file is named from the project root")
            );
        }
        let error = project.view().source_root("").unwrap_err();
        assert!(error.message.contains("names no file"), "{}", error.message);
    }
}
