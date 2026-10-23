//! How a source is keyed and opened: the one mint of a source key and the one opener of it (ADR 0242). A key is the
//! source's [`ProjectPath`] under [`SourceRoot::base`]; every layer that holds, stamps, parses or prints a source
//! names it by that text.

use super::SourceRoot;
use crate::ProjectPath;
use std::path::{Path, PathBuf};

impl SourceRoot {
    /// The directory every key of this project is relative to.
    pub fn base(&self) -> &Path {
        &self.dir
    }

    /// The key of the source at `path` ([`ProjectPath::key`] under [`Self::base`]): what the ledger stamps, the
    /// session holds, the parser is handed, a span says and every document prints. A path under the base is
    /// compared lexically first; the disk is read (`canonicalize`) only when that fails, so a link keeps its own
    /// name and a deleted file is keyed as it was.
    pub fn key(&self, path: &Path) -> ProjectPath {
        ProjectPath::key(self.base(), path)
    }

    /// The file `key` names: [`Self::base`] joined with it (an absolute key is its own path). The inverse of
    /// [`Self::key`] on every key it returns.
    pub fn path(&self, key: &str) -> PathBuf {
        self.base().join(key)
    }

    /// The spec root as a key prefix: [`Self::dir`] relative to [`Self::base`], `""` when they are one directory,
    /// absolute when the spec root lies outside it. Where an import path is read from, and the region "every
    /// source".
    pub fn prefix(&self) -> &Path {
        &self.prefix
    }

    /// Whether `key` is a source once its file exists: under [`Self::prefix`], and [`Self::holds`] of the rest.
    pub fn holds_key(&self, key: &str) -> bool {
        Path::new(key)
            .strip_prefix(&self.prefix)
            .is_ok_and(|rest| self.holds(&rest.to_string_lossy()))
    }

    /// [`Self::dir`] as a prefix of the keys under [`Self::base`].
    pub(super) fn prefix_of(base: &Path, dir: &Path) -> PathBuf {
        match ProjectPath::key(base, dir).as_str() {
            "." => PathBuf::new(),
            named => PathBuf::from(named),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Exclude;

    #[cfg(unix)]
    #[test]
    fn a_source_key_keeps_a_symlinks_own_name() {
        let dir = tempfile::TempDir::new().unwrap();
        let dir = std::fs::canonicalize(dir.path()).unwrap();
        let real = dir.join("real");
        std::fs::create_dir_all(real.join("shared")).unwrap();
        std::fs::write(real.join("shared/r.spec"), "").unwrap();
        // The root spelled two ways: through `link` and as `real`.
        std::os::unix::fs::symlink(&real, dir.join("link")).unwrap();
        std::os::unix::fs::symlink("shared/r.spec", real.join("alias.spec")).unwrap();

        let link = dir.join("link");
        let sources = SourceRoot::new(link.clone(), link, Exclude::default());
        assert_eq!(sources.key(&real.join("alias.spec")), "alias.spec");
    }

    #[cfg(unix)]
    #[test]
    fn a_walked_path_and_its_spellings_have_one_key() {
        let dir = tempfile::TempDir::new().unwrap();
        let dir = std::fs::canonicalize(dir.path()).unwrap();
        let real = dir.join("elsewhere");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(real.join("a.spec"), "").unwrap();
        // The spec root reached through a symlinked alias.
        let alias = dir.join("alias");
        std::os::unix::fs::symlink(&real, &alias).unwrap();

        let sources = SourceRoot::new(alias.clone(), alias.clone(), Exclude::default());
        assert_eq!(sources.key(&alias.join("a.spec")), "a.spec");
        assert_eq!(sources.key(&real.join("a.spec")), "a.spec");
        assert_eq!(sources.path("a.spec"), alias.join("a.spec"));
    }

    #[test]
    fn the_prefix_and_the_keys_under_it() {
        let dir = tempfile::TempDir::new().unwrap();
        let sources = SourceRoot::new(
            dir.path().to_path_buf(),
            dir.path().to_path_buf(),
            Exclude::default(),
        );
        assert_eq!(sources.prefix(), Path::new(""));
        assert!(sources.holds_key("sub/a.spec"));
        assert!(!sources.holds_key("sub/a.txt"));
        assert!(!sources.holds_key("/abs/a.spec"));
    }
}
