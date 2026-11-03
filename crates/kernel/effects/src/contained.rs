//! Where a written path may lead (ADR 0335 D1): the one path containment. A path is contained in the root it
//! names (`spec`, `project`, or a named root through a `<name>/` prefix), checked lexically, then for real through
//! its links, then name by name against the directory listings, so the answer is the same on every operating system.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::ffi::OsString;
use std::fmt;
use std::path::{Component, Path, PathBuf};

use specforge_common::find_close_match;

/// A root a path is written against: `spec`, `project`, or one a project names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RootName {
    Spec,
    Project,
    Named(String),
}

impl RootName {
    /// The root `name` names: `spec`, `project`, else a named root.
    pub fn of(name: &str) -> RootName {
        match name {
            "spec" => RootName::Spec,
            "project" => RootName::Project,
            other => RootName::Named(other.to_string()),
        }
    }
}

impl fmt::Display for RootName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RootName::Spec => f.write_str("spec"),
            RootName::Project => f.write_str("project"),
            RootName::Named(name) => f.write_str(name),
        }
    }
}

/// A project's roots: the project root, the spec root and the roots its configuration names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    project: PathBuf,
    spec: PathBuf,
    named: BTreeMap<String, PathBuf>,
}

impl Roots {
    /// The project root and the spec root, as the environment resolved them.
    pub fn new(project: impl Into<PathBuf>, spec: impl Into<PathBuf>) -> Self {
        Roots {
            project: project.into(),
            spec: spec.into(),
            named: BTreeMap::new(),
        }
    }

    /// These roots and `name` at `path`: plan 05's named roots (`specforge.json` `roots`) are built with it.
    pub fn with_named(mut self, name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        self.named.insert(name.into(), path.into());
        self
    }

    /// Where `root` lies; `None` for a root no one named.
    pub fn get(&self, root: &RootName) -> Option<&Path> {
        match root {
            RootName::Spec => Some(&self.spec),
            RootName::Project => Some(&self.project),
            RootName::Named(name) => self.named.get(name).map(PathBuf::as_path),
        }
    }

    pub fn project(&self) -> &Path {
        &self.project
    }

    pub fn spec(&self) -> &Path {
        &self.spec
    }

    /// The spec root does not lie under the project root (ADR 0145 D4): it is an outside root, which a grant treats
    /// as a named root outside the project (plan 02).
    pub fn spec_outside(&self) -> bool {
        !self.spec.starts_with(&self.project)
    }
}

/// What the real checks ask the file system. A disk with no links (memory, a commit) answers `canonical` with `None`,
/// and the real walk is skipped.
pub trait Lookup {
    /// `path` with every link resolved; `None` when it does not exist, or the disk has no links.
    fn canonical(&self, path: &Path) -> Option<PathBuf>;
    /// The target of the link at `path`, read but not followed: a dangling link is followed by its target text.
    fn read_link(&self, _path: &Path) -> Option<PathBuf> {
        None
    }
    /// The names directly in `dir` (UTF-8 only); `None` when it is not a directory that can be listed.
    fn names(&self, dir: &Path) -> Option<Vec<String>>;
}

/// The real file system.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealFs;

impl RealFs {
    /// Where `path` lands on the real file system: see [`landing`].
    pub fn landing(path: &Path) -> PathBuf {
        landing(&RealFs, path)
    }
}

impl Lookup for RealFs {
    fn canonical(&self, path: &Path) -> Option<PathBuf> {
        std::fs::canonicalize(path).ok()
    }

    fn read_link(&self, path: &Path) -> Option<PathBuf> {
        std::fs::read_link(path).ok()
    }

    fn names(&self, dir: &Path) -> Option<Vec<String>> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .ok()?
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .collect();
        names.sort();
        Some(names)
    }
}

/// The most links one walk follows; past them a name lands at itself and its open fails.
pub const MAX_LINKS: usize = 40;

/// One step of a path still to walk.
enum Step {
    Root(OsString),
    Parent,
    Name(OsString),
}

fn steps(path: &Path) -> impl Iterator<Item = Step> {
    path.components().filter_map(|component| match component {
        Component::Prefix(_) | Component::RootDir => Some(Step::Root(component.as_os_str().into())),
        Component::CurDir => None,
        Component::ParentDir => Some(Step::Parent),
        Component::Normal(name) => Some(Step::Name(name.to_os_string())),
    })
}

/// Where `path` lands: made absolute and walked component by component. `..` leaves the real directory reached so
/// far (after its links); every existing component is replaced by its canonical form; a link whose target does not
/// exist is replaced by its target (read against the link's directory) and the walk goes on, at most [`MAX_LINKS`]
/// times (then the name lands at itself). A missing tail is joined on.
pub fn landing(lookup: &dyn Lookup, path: &Path) -> PathBuf {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut pending: VecDeque<Step> = steps(&path).collect();
    let mut landed = PathBuf::new();
    let mut links = 0;
    while let Some(step) = pending.pop_front() {
        match step {
            Step::Root(root) => landed.push(root),
            Step::Parent => {
                landed.pop();
            }
            Step::Name(name) => {
                landed.push(&name);
                if let Some(real) = lookup.canonical(&landed) {
                    landed = real;
                } else if links < MAX_LINKS
                    && let Some(target) = lookup.read_link(&landed)
                {
                    links += 1;
                    landed.pop();
                    for step in steps(&target).collect::<Vec<_>>().into_iter().rev() {
                        pending.push_front(step);
                    }
                    if target.is_absolute() {
                        landed = PathBuf::new();
                    }
                }
            }
        }
    }
    landed
}

/// Why a written path is not one its root holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// `<name>/…` names a root no one declared (E095).
    UnknownRoot(String),
    /// A root, a drive or a UNC prefix (E094).
    Absolute,
    /// A `..` that climbs above the root (E094).
    Climbs,
    /// A link that lands outside the root (E094).
    LinksOut { landed: PathBuf },
    /// Nothing is named.
    Empty,
}

/// A written path its root holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub root: RootName,
    /// Relative to the root, `/` joined, lexically normal.
    pub rel: String,
    /// The root joined with `rel`.
    pub at: PathBuf,
    /// Where `at` lands with its links resolved, when the disk has links.
    pub real: Option<PathBuf>,
    /// Every name exists as written, or as its one case variant (see `case`).
    pub exists: bool,
    /// `rel` as the disk spells it, when a name differs from it in case only (W174).
    pub case: Option<String>,
    /// The written path used `\`, read as `/` (W175).
    pub backslashes: bool,
    /// The name, or where it lands, is on the credential deny-list (ADR 0293 D3a).
    pub withheld: bool,
}

/// The one path containment (ADR 0335 D1): a written path is contained in the root it names, decided lexically, then
/// for real, then against the directory listings, over a [`Lookup`]. Listings are kept for the life of the value.
pub struct Contained<'a> {
    roots: Cow<'a, Roots>,
    lookup: &'a dyn Lookup,
    listings: RefCell<BTreeMap<PathBuf, Option<Vec<String>>>>,
}

impl<'a> Contained<'a> {
    pub fn new(roots: &'a Roots, lookup: &'a dyn Lookup) -> Self {
        Contained {
            roots: Cow::Borrowed(roots),
            lookup,
            listings: RefCell::new(BTreeMap::new()),
        }
    }

    /// A project's own containment: its project root and its spec root, over `lookup`. What every reader of a
    /// project's files (a rule, a judged input, a K11 read, a cited file) resolves through.
    pub fn project(project: &Path, spec: &Path, lookup: &'a dyn Lookup) -> Self {
        Contained {
            roots: Cow::Owned(Roots::new(project, spec)),
            lookup,
            listings: RefCell::new(BTreeMap::new()),
        }
    }

    /// `written`, in the root its `<name>/` prefix names, else in `default`.
    pub fn resolve(&self, written: &str, default: RootName) -> Result<Resolved, Refused> {
        let backslashes = written.contains('\\');
        let text = written.replace('\\', "/");
        let (root, rest) = match prefixed(&text) {
            Some((name, rest)) => (RootName::of(name), rest),
            None => (default, text.as_str()),
        };
        let Some(base) = self.roots.get(&root) else {
            return Err(Refused::UnknownRoot(root.to_string()));
        };
        if absolute(rest) {
            return Err(Refused::Absolute);
        }
        let mut parts: Vec<&str> = Vec::new();
        for part in rest.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop().ok_or(Refused::Climbs)?;
                }
                name => parts.push(name),
            }
        }
        if parts.is_empty() {
            return Err(Refused::Empty);
        }
        let rel = parts.join("/");
        let at = parts.iter().fold(base.to_path_buf(), |at, p| at.join(p));
        let real_root = self.lookup.canonical(base);
        let real = match &real_root {
            Some(real_root) => {
                let landed = landing(self.lookup, &at);
                if !landed.starts_with(real_root) {
                    return Err(Refused::LinksOut { landed });
                }
                Some(landed)
            }
            None => None,
        };
        let (spelled, mut exists) = self.spelled(base, &parts);
        let case = (exists && spelled != parts).then(|| spelled.join("/"));
        if exists && real_root.is_some() {
            let on_disk = spelled.iter().fold(base.to_path_buf(), |at, p| at.join(p));
            exists = self.lookup.canonical(&on_disk).is_some();
        }
        let withheld = withheld(&rel)
            || real
                .as_ref()
                .is_some_and(|real| withheld(&real.to_string_lossy().replace('\\', "/")));
        Ok(Resolved {
            root,
            rel,
            at,
            real,
            exists,
            case,
            backslashes,
            withheld,
        })
    }

    /// A close name to `resolved`'s in its parent, as written with that name: the parent is one `resolve` held, so no
    /// directory outside a root is ever listed.
    pub fn sibling(&self, resolved: &Resolved) -> Option<String> {
        let (dir, name) = match resolved.rel.rsplit_once('/') {
            Some((dir, name)) => (Some(dir), name),
            None => (None, resolved.rel.as_str()),
        };
        let names = self.names(resolved.at.parent()?)?;
        let close = find_close_match(name, names.iter().map(String::as_str))?;
        Some(match dir {
            Some(dir) => format!("{dir}/{close}"),
            None => close.to_string(),
        })
    }

    /// Each of `parts` as the listing of its parent spells it, while one exists: the written name, else its only
    /// case-insensitive match. The bool: every part was found.
    fn spelled(&self, base: &Path, parts: &[&str]) -> (Vec<String>, bool) {
        let mut dir = base.to_path_buf();
        let mut spelled = Vec::with_capacity(parts.len());
        for part in parts {
            let Some(names) = self.names(&dir) else {
                return (spelled, false);
            };
            let found = if names.iter().any(|n| n == part) {
                (*part).to_string()
            } else {
                let mut variants = names.iter().filter(|n| n.eq_ignore_ascii_case(part));
                match (variants.next(), variants.next()) {
                    (Some(only), None) => only.clone(),
                    _ => return (spelled, false),
                }
            };
            dir.push(&found);
            spelled.push(found);
        }
        (spelled, true)
    }

    fn names(&self, dir: &Path) -> Option<Vec<String>> {
        if let Some(names) = self.listings.borrow().get(dir) {
            return names.clone();
        }
        let names = self.lookup.names(dir);
        self.listings
            .borrow_mut()
            .insert(dir.to_path_buf(), names.clone());
        names
    }
}

/// `<name>/rest` (or `<name>` alone): the root's name and the rest.
fn prefixed(text: &str) -> Option<(&str, &str)> {
    let inner = text.strip_prefix('<')?;
    let (name, rest) = inner.split_once('>')?;
    if name.is_empty() || name.contains(['/', '<']) {
        return None;
    }
    match rest {
        "" => Some((name, "")),
        _ => rest.strip_prefix('/').map(|rest| (name, rest)),
    }
}

/// A root (`/`), a drive (`C:`) or a UNC prefix, on any operating system.
fn absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/')
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
        || Path::new(path).is_absolute()
}

/// Whether `path` (`/` separated) looks like it holds a credential, which a read for text does not hand to a guest
/// (ADR 0293 D3a): `.env` and `.env.*`, `*.pem`, `*.key`, `id_rsa*`, `id_ed25519*`, `.npmrc`, `.netrc`,
/// `credentials*` and `.git/config`. shortcut: names only, and no project can opt a file in; upgrade when a project
/// needs to.
pub fn withheld(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    let name = path.rsplit('/').next().unwrap_or_default();
    name == ".env"
        || name.starts_with(".env.")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.starts_with("id_rsa")
        || name.starts_with("id_ed25519")
        || name == ".npmrc"
        || name == ".netrc"
        || name.starts_with("credentials")
        || path == ".git/config"
        || path.ends_with("/.git/config")
}
