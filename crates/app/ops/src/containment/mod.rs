//! What a request may reach on disk (ADR 0104): every path, or the paths that land inside one
//! directory. An operation applies it where it opens or writes the file, so the name it checks is
//! the name it opens. The CLI is unbounded (ADR 0092 D6); MCP bounds a request (ADR 0092 D2).
//!
//! A name is resolved the way the file system would: a relative one is read against the root
//! (bounded) or the process's directory (unbounded), `.` and `..` are applied to the real directory
//! reached so far, and every symlink on the way is followed, a dangling one by its target text.
//! What does not exist yet (a file about to be created) is the rest of the name, joined on.

mod disk;
mod file;

pub use disk::ContainedDisk;
pub use file::{ReadFailure, declared};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use specforge_protocol_types::{Root, RootedPath};

// RealFs only looks paths up (where a link lands): a Lookup, not an Fs effect, so it reads no file past the Fs port.
use specforge_effects::RealFs;

use crate::{OpError, OpErrorKind};

/// The code of every refusal [`Outside`] becomes through `From` (an `OpError` code string, not a
/// catalogue diagnostic: ADR 0092 D5).
pub const OUTSIDE_ROOT: &str = "outside_root";

/// What a request may reach. Built by the surface, carried in the operation's request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Containment {
    /// The directory, canonical as far as it existed when built; `None`: unbounded.
    root: Option<PathBuf>,
    /// The other roots a `<name>/…` path may name (ADR 0336 D6), each where it lands: the spec root and the named roots
    /// this machine may reach. Only a bounded containment reads the prefix: the CLI's user types a path as it is.
    roots: BTreeMap<String, PathBuf>,
}

/// A path a containment holds (ADR 0104, ADR 0112, ADR 0272): one path, three names, one use each.
/// The caller opens (reads, rewrites, creates) `real`, except an owned name, which it opens at `at`;
/// it quotes `named` in its messages and writes [`Held::shown`] in its documents and records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    /// As the caller wrote it. May be relative even when bounded: never joined onto anything.
    pub named: PathBuf,
    /// `named` read against its base (a bounded containment's root, the process's directory when
    /// unbounded, or the base given to [`Containment::resolve_from`]), its links not followed: an
    /// owned name is created and unlinked here (ADR 0171), a runner is handed it, and `shown` names it.
    pub at: PathBuf,
    /// Where it lands: canonical as far as it exists, a missing tail joined on.
    pub real: PathBuf,
}

/// A name that lands outside its containment's root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outside {
    named: PathBuf,
    landed: PathBuf,
    root: PathBuf,
}

impl Containment {
    /// Every path: [`resolve`](Self::resolve) hands a name back as given and refuses nothing.
    pub const fn unbounded() -> Self {
        Containment {
            root: None,
            roots: BTreeMap::new(),
        }
    }

    /// The paths that land inside `root`, as it is on disk now.
    pub fn within(root: &Path) -> Self {
        Containment {
            root: Some(RealFs::landing(root)),
            roots: BTreeMap::new(),
        }
    }

    /// The same bound, and a `<name>/…` path may also name each of `roots`.
    pub fn with_roots(mut self, roots: impl IntoIterator<Item = (String, PathBuf)>) -> Self {
        self.roots = roots
            .into_iter()
            .map(|(name, at)| (name, RealFs::landing(&at)))
            .collect();
        self
    }

    /// This containment as the `Disk` port: what a session reads its sources through when the surface
    /// bounds them (ADR 0145).
    pub fn disk(&self) -> ContainedDisk {
        ContainedDisk(self.clone())
    }

    /// The same kind of bound for a project inside this one: `within(dir)` when bounded,
    /// unbounded when unbounded. A change bounds its writes to its own project (ADR 0092 D8).
    pub fn narrowed(&self, dir: &Path) -> Self {
        match self.root {
            Some(_) => Containment::within(dir),
            None => Containment::unbounded(),
        }
    }

    /// The directory, canonical; `None` when unbounded.
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// [`resolve_from`](Self::resolve_from) against the root when bounded, the process's directory
    /// when not. Bounded: an absolute `named` stands; `Outside` unless it lands inside. Unbounded:
    /// a relative name is the process's, as the CLI's user typed it, and nothing is refused. Either
    /// way the real path is computed component by component, so what the caller opens is what was
    /// checked.
    pub fn resolve(&self, named: &Path) -> Result<Held, Outside> {
        self.resolve_from(self.root.as_deref().unwrap_or(Path::new("")), named)
    }

    /// `named` read against `base` (an absolute `named` stands); `Outside` (quoting `named`) unless
    /// it lands inside the root.
    pub fn resolve_from(&self, base: &Path, named: &Path) -> Result<Held, Outside> {
        if let Some(root) = &self.root
            && let Some(text) = named.to_str().filter(|text| text.starts_with('<'))
        {
            return self.hold_rooted(root, named, text);
        }
        self.hold(named.to_path_buf(), base.join(named))
    }

    /// The name `text`, whose `<root>/` prefix names one of the roots: it lands inside that root, or `Outside`. A root
    /// not reached from here (unknown, or outside the project and not approved) is `Outside` too.
    fn hold_rooted(&self, bound: &Path, named: &Path, text: &str) -> Result<Held, Outside> {
        let outside = |landed: PathBuf, root: &Path| Outside::new(named, landed, root);
        let rooted = RootedPath::parse(text, Root::Project)
            .map_err(|_| outside(named.to_path_buf(), bound))?;
        let root = match &rooted.root {
            Root::Project => bound,
            other => self
                .roots
                .get(other.as_str())
                .ok_or_else(|| outside(named.to_path_buf(), bound))?,
        };
        let at = root.join(&rooted.path);
        let real = RealFs::landing(&at);
        if !real.starts_with(root) {
            return Err(outside(real, root));
        }
        Ok(Held {
            named: named.to_path_buf(),
            at,
            real,
        })
    }

    /// The name `named`, read at `at`, held: its landing checked against the root when bounded.
    fn hold(&self, named: PathBuf, at: PathBuf) -> Result<Held, Outside> {
        let real = RealFs::landing(&at);
        match &self.root {
            Some(root) if !real.starts_with(root) => Err(Outside {
                named,
                landed: real,
                root: root.clone(),
            }),
            _ => Ok(Held { named, at, real }),
        }
    }
}

impl Outside {
    /// The refusal for `named` (a spelling of the caller's), which landed at `landed`, outside
    /// `root`.
    pub fn new(named: &Path, landed: PathBuf, root: &Path) -> Self {
        Outside {
            named: named.to_path_buf(),
            landed,
            root: root.to_path_buf(),
        }
    }

    /// As the caller spelled it.
    pub fn named(&self) -> &Path {
        &self.named
    }

    /// Where it really is.
    pub fn landed(&self) -> &Path {
        &self.landed
    }

    /// What bounds it.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// `invalid_input`, [`OUTSIDE_ROOT`]: "'<named>' resolves to <landed>, outside <root>, which bounds
/// the request". An operation that keeps its own refusal (migrate, infer, format's per-file
/// failure) builds it from the accessors instead.
impl From<Outside> for OpError {
    fn from(outside: Outside) -> Self {
        OpError::new(
            OpErrorKind::InvalidInput,
            OUTSIDE_ROOT,
            format!(
                "'{}' resolves to {}, outside {}, which bounds the request",
                outside.named.display(),
                outside.landed.display(),
                outside.root.display()
            ),
        )
    }
}

#[cfg(test)]
mod disk_tests;
#[cfg(test)]
mod named_tests;
#[cfg(test)]
mod regular_tests;
#[cfg(test)]
mod replace_tests;
#[cfg(test)]
mod tests;
