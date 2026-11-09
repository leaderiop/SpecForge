//! The project as cargo's reader sees it from one package's directory: the host's file listing and the manifests'
//! texts, no file system (a guest has none, ADR 0293).

use std::collections::{BTreeMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use cargo_toml::{AbstractFilesystem, Error, Manifest, Value};

use crate::joined;

/// The absolute stand-in for the project root a workspace's base path starts with: `cargo_toml` joins an inherited
/// path dependency to it, and [`crate::read_packages`] strips it again.
pub const ROOT: &str = "/";

/// One package directory's view of the project: the files under it, as the listing names them, and every manifest
/// above it, for the workspace its fields inherit from.
pub struct Listing<'a> {
    dir: String,
    files: &'a [String],
    manifests: &'a BTreeMap<&'a str, &'a str>,
}

impl<'a> Listing<'a> {
    /// The view from the project-relative `dir` over `files` (project-relative paths) and `manifests` (each
    /// `Cargo.toml`'s project-relative path and its text; their paths are listed too).
    pub fn new(dir: &str, files: &'a [String], manifests: &'a BTreeMap<&'a str, &'a str>) -> Self {
        Listing {
            dir: dir.to_string(),
            files,
            manifests,
        }
    }

    /// The workspace manifest in the project-relative directory `dir`, if that directory holds one.
    fn workspace_in(&self, dir: &str) -> Result<Option<Manifest<Value>>, Error> {
        let path = if dir.is_empty() {
            "Cargo.toml".to_string()
        } else {
            format!("{dir}/Cargo.toml")
        };
        let Some(text) = self.manifests.get(path.as_str()) else {
            return Ok(None);
        };
        let manifest = Manifest::<Value>::from_slice_with_metadata(text.as_bytes())?;
        Ok(manifest.workspace.is_some().then_some(manifest))
    }
}

impl AbstractFilesystem for Listing<'_> {
    /// The names one level under `rel_path` of the package directory: a file's, or a directory's that holds a listed
    /// file. A directory nothing is listed under is not found, as cargo's reader expects of an absent one.
    fn file_names_in(&self, rel_path: &str) -> io::Result<HashSet<Box<str>>> {
        let not_found = || io::Error::from(io::ErrorKind::NotFound);
        let base = joined(&self.dir, rel_path).ok_or_else(not_found)?;
        let prefix = if base.is_empty() {
            String::new()
        } else {
            format!("{base}/")
        };
        let names: HashSet<Box<str>> = self
            .files
            .iter()
            .map(String::as_str)
            .chain(self.manifests.keys().copied())
            .filter_map(|path| path.strip_prefix(prefix.as_str()))
            .filter_map(|rest| rest.split('/').next())
            .filter(|name| !name.is_empty())
            .map(Box::from)
            .collect();
        if names.is_empty() {
            Err(not_found())
        } else {
            Ok(names)
        }
    }

    /// The workspace `hint` (`package.workspace`, a directory relative to the package's) names, or else the nearest
    /// manifest declaring `[workspace]` at or above the package's directory; its base path is [`ROOT`] joined to its
    /// directory.
    fn parse_root_workspace(
        &self,
        hint: Option<&Path>,
    ) -> Result<(Manifest<Value>, PathBuf), Error> {
        let missing = |why: String| Error::Io(io::Error::new(io::ErrorKind::NotFound, why));
        let found = |dir: &str, manifest| (manifest, Path::new(ROOT).join(dir));
        if let Some(hint) = hint {
            let hint = hint.to_string_lossy();
            let hint = hint.strip_suffix("Cargo.toml").unwrap_or(&hint);
            let dir = joined(&self.dir, hint)
                .ok_or_else(|| missing(format!("the workspace `{hint}` is outside the project")))?;
            return match self.workspace_in(&dir)? {
                Some(manifest) => Ok(found(&dir, manifest)),
                None => Err(missing(format!(
                    "no Cargo.toml in `{dir}` declares [workspace]"
                ))),
            };
        }
        let mut dir = self.dir.clone();
        loop {
            if let Some(manifest) = self.workspace_in(&dir)? {
                return Ok(found(&dir, manifest));
            }
            if dir.is_empty() {
                return Err(missing(format!(
                    "no Cargo.toml at or above `{}` declares [workspace]",
                    self.dir
                )));
            }
            dir = dir
                .rsplit_once('/')
                .map_or("", |(parent, _)| parent)
                .to_string();
        }
    }
}
