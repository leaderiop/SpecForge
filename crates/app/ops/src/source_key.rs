//! The source path rule (ADR 0113 D1): the key of a source-code file, root-relative and `/`-separated, that the
//! inference manifest records and a request's path is read into (ADR 0253). One reader, [`SourceKey::read`], reads
//! a path from the project root, whoever wrote it: a request or a manifest (ADR 0271).

use std::path::{Component, Path};

use specforge_common::ProjectPath;

/// Why a path read from the project root is no source key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unkeyed {
    /// Its key lies outside the root: a `..` past it, an absolute path elsewhere, a drive path.
    Outside,
    /// It names nothing: `""`, `.`, `./`.
    Nothing,
}

/// `named`, read from `base`, as the key [`ProjectPath::key`] mints for it: a `.`, a doubled separator and a `..`
/// applied where they stand, an absolute path under `base` its relative key, a link keeping its own name
/// (ADR 0246).
pub(crate) fn minted(base: &Path, named: &str) -> String {
    ProjectPath::key(base, &base.join(named)).into_string()
}

/// A path outside the project root, or naming nothing: the source path rule refuses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OutsideRoot;

/// A source file's key: root-relative, `/`-separated, no empty or `.` component, never absolute,
/// a drive path or `..`. What the manifest records and what every read opens (joined onto the
/// project root).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SourceKey(String);

impl SourceKey {
    /// The source path rule: `given` as the root-relative, `/`-separated path the manifest
    /// records. Separators `/` and `\` are both accepted; empty and `.` components are dropped.
    /// A path that is absolute, has a drive prefix, has a `..` component, or names nothing is
    /// outside the root.
    fn of(given: &str) -> Result<SourceKey, OutsideRoot> {
        let parts = components(given)?;
        if parts.is_empty() {
            return Err(OutsideRoot);
        }
        Ok(SourceKey(parts.join("/")))
    }

    /// [`Self::of`] for a source root: the project root itself (`.`) is a root, naming nothing
    /// at all is not.
    fn root(given: &str) -> Result<SourceKey, OutsideRoot> {
        let parts = components(given)?;
        if parts.is_empty() {
            return if given.is_empty() {
                Err(OutsideRoot)
            } else {
                Ok(SourceKey(".".to_string()))
            };
        }
        Ok(SourceKey(parts.join("/")))
    }

    /// `named` read from the project `root` (ADR 0253 D1, ADR 0271 D1): minted as a named spec file is, then the
    /// source path rule. Whoever wrote it, every spelling of one file under the root is that file. Opens no file:
    /// where a link lands is judged where the file is opened.
    pub(crate) fn read(root: &Path, named: &str) -> Result<SourceKey, Unkeyed> {
        Self::keyed(root, named, Self::of)
    }

    /// [`Self::read`] for a directory: `.` and the root's own path are the root, `""` names nothing.
    pub(crate) fn read_root(root: &Path, named: &str) -> Result<SourceKey, Unkeyed> {
        if named.is_empty() {
            return Err(Unkeyed::Nothing);
        }
        Self::keyed(root, named, Self::root)
    }

    fn keyed(
        root: &Path,
        named: &str,
        rule: impl FnOnce(&str) -> Result<SourceKey, OutsideRoot>,
    ) -> Result<SourceKey, Unkeyed> {
        let key = minted(root, named);
        let climbs = Path::new(&key).components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        });
        if !climbs && let Ok(key) = rule(&key) {
            return Ok(key);
        }
        // What stays refuses only when the rule finds nothing in it: a drive path or a leading `\` lies outside.
        let nothing = !climbs
            && key
                .split(['/', '\\'])
                .all(|part| part.is_empty() || part == ".");
        Err(if nothing {
            Unkeyed::Nothing
        } else {
            Unkeyed::Outside
        })
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// The components of `query` under the source path rule's separators (`/` and `\`), without the
/// empty and `.` ones. A path that is absolute, has a drive prefix or has a `..` component is
/// outside the root.
fn components(query: &str) -> Result<Vec<&str>, OutsideRoot> {
    let bytes = query.as_bytes();
    let absolute = query.starts_with(['/', '\\']);
    let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if absolute || drive {
        return Err(OutsideRoot);
    }
    let parts: Vec<&str> = query
        .split(['/', '\\'])
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    if parts.contains(&"..") {
        return Err(OutsideRoot);
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_read_from_its_root() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        let canonical = std::fs::canonicalize(root).unwrap();
        let at = |base: &Path, tail: &str| format!("{}/{tail}", base.display());
        for named in [
            "src/a.rs",
            "./src//a.rs",
            "src\\a.rs",
            "src/./a.rs",
            "src/../src/a.rs",
            &at(root, "src/a.rs"),
            &at(&canonical, "src/a.rs"),
        ] {
            assert_eq!(
                SourceKey::read(root, named).as_ref().map(SourceKey::as_str),
                Ok("src/a.rs"),
                "{named}"
            );
        }
        for named in ["../x", "/x", "\\x", "C:\\x", &at(root, "../x")] {
            assert_eq!(
                SourceKey::read(root, named),
                Err(Unkeyed::Outside),
                "{named}"
            );
        }
        for named in ["", ".", "./"] {
            assert_eq!(
                SourceKey::read(root, named),
                Err(Unkeyed::Nothing),
                "{named:?}"
            );
        }
        for named in [".", root.to_str().unwrap()] {
            assert_eq!(
                SourceKey::read_root(root, named)
                    .as_ref()
                    .map(SourceKey::as_str),
                Ok("."),
                "{named}"
            );
        }
        assert_eq!(SourceKey::read_root(root, ""), Err(Unkeyed::Nothing));
    }

    #[test]
    fn the_source_path_rule_over_a_root() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        for (given, recorded) in [
            ("a.rs", "a.rs"),
            ("./a.rs", "a.rs"),
            ("src\\a.rs", "src/a.rs"),
            ("src//a.rs", "src/a.rs"),
            ("src/./a.rs", "src/a.rs"),
            ("src/", "src"),
            ("a/../b", "b"),
        ] {
            assert_eq!(
                SourceKey::read(root, given).map(|k| k.as_str().to_string()),
                Ok(recorded.to_string()),
                "{given}"
            );
        }
        for outside in ["/x", "\\x", "C:\\x", "c:/x", "../x", "a/../../x"] {
            assert_eq!(
                SourceKey::read(root, outside),
                Err(Unkeyed::Outside),
                "{outside:?}"
            );
        }
        for nothing in ["", ".", "./"] {
            assert_eq!(
                SourceKey::read(root, nothing),
                Err(Unkeyed::Nothing),
                "{nothing:?}"
            );
        }
        // A source root may be the project root itself.
        assert_eq!(SourceKey::read_root(root, ".").unwrap().as_str(), ".");
        assert_eq!(SourceKey::read_root(root, "./src").unwrap().as_str(), "src");
        assert_eq!(SourceKey::read_root(root, ""), Err(Unkeyed::Nothing));
        for outside in ["/x", "../x"] {
            assert_eq!(
                SourceKey::read_root(root, outside),
                Err(Unkeyed::Outside),
                "{outside:?}"
            );
        }
    }
}
