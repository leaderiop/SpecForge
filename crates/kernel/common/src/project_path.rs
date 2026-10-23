//! How a surface document names a file an operation opened or wrote (ADR 0102).

use serde::Serialize;
use std::fmt;
use std::path::{Component, Path, PathBuf};

/// A file of a project as a document names it: relative to the project root with `/` between components, or, for a
/// file outside the root (only the CLI can name one, ADR 0092 D6), or a source outside it, absolute and canonical as far as it exists.
/// It serializes as that string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, crate::shape::Shape)]
pub struct ProjectPath(String);

impl ProjectPath {
    /// `path` named from `root`. A path and a root that name one place through a symbolic link (`/var` and
    /// `/private/var`) are compared as their canonical forms, a removed file's through its nearest existing ancestor,
    /// and a linked file keeps its own name (as a source does), not its target's.
    /// A relative `path` is read against the process's working directory, as the CLI's arguments are. A `..` is applied
    /// where it stands: a path through a missing directory and back is named from the root.
    pub fn of(root: &Path, path: &Path) -> ProjectPath {
        inside(root, path).unwrap_or_else(|| ProjectPath::absolute(path))
    }

    /// The key of the source at `path` in the project at `root` (ADR 0242): inside the root, what [`Self::of`] names;
    /// outside it, an absolute path with its directory canonical and its own name kept (a source under a spec root
    /// outside the project, ADR 0080 D2; a detached buffer); a relative name that is no path (an editor's
    /// `untitled:` document) as given. An empty `root` (no project) names every absolute path that way.
    pub fn key(root: &Path, path: &Path) -> ProjectPath {
        if !path.is_absolute() {
            return within(path, root)
                .unwrap_or_else(|| ProjectPath(path.to_string_lossy().into_owned()));
        }
        inside(root, path)
            .unwrap_or_else(|| ProjectPath(in_canonical_dir(path).to_string_lossy().into_owned()))
    }

    /// `path` named with no root: absolute and canonical as far as it exists.
    pub fn absolute(path: &Path) -> ProjectPath {
        ProjectPath(canonical(path).display().to_string())
    }

    /// The name as the document prints it.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The name, owned.
    pub fn into_string(self) -> String {
        self.0
    }

    /// The file this names, joined onto `root` (an absolute name is its own path): what a consumer that opens the
    /// named file again reads.
    pub fn under(&self, root: &Path) -> PathBuf {
        root.join(&self.0)
    }
}

impl AsRef<str> for ProjectPath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<&str> for ProjectPath {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<str> for ProjectPath {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl fmt::Display for ProjectPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `path` named from `root`: lexically, else by its directory's canonical form and its own name under the canonical
/// root. `None` when it is not under the root. An empty `root` is no project.
fn inside(root: &Path, path: &Path) -> Option<ProjectPath> {
    if root.as_os_str().is_empty() {
        return None;
    }
    within(path, root).or_else(|| within(&in_canonical_dir(path), &canonical(root)))
}

/// `path` with its directory canonical and its own name kept: a symbolic link stays itself. A path that ends in no
/// name (`..`, a root) is canonical as a whole.
fn in_canonical_dir(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    match (absolute.parent(), absolute.file_name()) {
        (Some(parent), Some(name))
            if absolute.components().next_back() == Some(Component::Normal(name)) =>
        {
            canonical(parent).join(name)
        }
        _ => canonical(path),
    }
}

/// `path` relative to `root` as spelled, `/`-joined; `None` when it is not under it, or reaches out and back
/// through a `..`.
fn within(path: &Path, root: &Path) -> Option<ProjectPath> {
    let relative = path.strip_prefix(root).ok()?;
    let mut names = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(name) => names.push(name.to_string_lossy()),
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(ProjectPath(if names.is_empty() {
        ".".to_string()
    } else {
        names.join("/")
    }))
}

/// `path` absolute and canonical as far as it exists: the canonical form of its nearest existing ancestor, joined
/// with the rest.
fn canonical(path: &Path) -> PathBuf {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    // `..` leaves the real directory reached so far; past a name that does not exist (which is no link) it leaves
    // that name. What remains has no `..`, so the walk up below can name every component.
    let mut walked = PathBuf::new();
    for component in path.components() {
        if component == Component::ParentDir {
            walked = std::fs::canonicalize(&walked).unwrap_or(walked);
            walked.pop();
        } else {
            walked.push(component);
        }
    }
    let path = walked;
    let mut existing = path.clone();
    let mut rest = Vec::new();
    while !existing.exists() {
        let Some(name) = existing.file_name().map(|n| n.to_os_string()) else {
            break;
        };
        rest.push(name);
        if !existing.pop() {
            break;
        }
    }
    let mut canonical = std::fs::canonicalize(&existing).unwrap_or(existing);
    canonical.extend(rest.into_iter().rev());
    canonical
}

#[cfg(test)]
mod tests;
