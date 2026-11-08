//! Where a written path may lead (ADR 0335 D1): the one path containment. A path is contained in the root it
//! names (`spec`, `project`, or a named root through a `<name>/` prefix), checked lexically, then for real through
//! its links, then name by name against the directory listings, so the answer is the same on every operating system.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use specforge_common::find_close_match;
use specforge_protocol_types::{PrefixProblem, Root, RootedPath};

/// The name of the verbs' scratch root (ADR 0339).
pub const TEMP: &str = "temp";

/// The verbs' scratch root, [`TEMP`], as a path names it.
pub fn temp_root() -> Root {
    Root::Named(TEMP.to_string())
}

/// A project's roots: the project root, the spec root and the roots its configuration names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    project: PathBuf,
    /// None once [`Self::without_spec`] dropped it: a path naming it is refused like an unknown root.
    spec: Option<PathBuf>,
    named: BTreeMap<String, PathBuf>,
    /// The verbs' scratch root `temp` (ADR 0339), set by [`Self::with_temp`]: it lies in the host's own state, and only
    /// what a person types reaches it.
    temp: Option<PathBuf>,
}

impl Roots {
    /// The project root and the spec root, as the environment resolved them.
    pub fn new(project: impl Into<PathBuf>, spec: impl Into<PathBuf>) -> Self {
        Roots {
            project: project.into(),
            spec: Some(spec.into()),
            named: BTreeMap::new(),
            temp: None,
        }
    }

    /// These roots and `name` at `path`: the named roots of `specforge.json` `roots` are built with it.
    pub fn with_named(mut self, name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        self.named.insert(name.into(), path.into());
        self
    }

    /// These roots and the verbs' scratch root `temp` at `path` (ADR 0339): a project names no root `temp` (E109), so
    /// no guest's roots ever hold it.
    pub fn with_temp(mut self, path: impl Into<PathBuf>) -> Self {
        self.temp = Some(path.into());
        self
    }

    /// Whether `root` is the verbs' scratch root, which lies in the host's own state by design.
    pub fn is_temp(&self, root: &Root) -> bool {
        self.temp.is_some() && matches!(root, Root::Named(name) if name == TEMP)
    }

    /// The named roots (`specforge.json` `roots`), by name, and where each lies.
    pub fn named(&self) -> impl Iterator<Item = (&str, &Path)> {
        self.named
            .iter()
            .map(|(name, path)| (name.as_str(), path.as_path()))
    }

    /// Where `root` lies; `None` for a root no one named.
    pub fn get(&self, root: &Root) -> Option<&Path> {
        match root {
            Root::Spec => self.spec.as_deref(),
            Root::Project => Some(&self.project),
            Root::Named(name) if name == TEMP => self.temp.as_deref(),
            Root::Named(name) => self.named.get(name).map(PathBuf::as_path),
        }
    }

    pub fn project(&self) -> &Path {
        &self.project
    }

    /// These roots with no spec root: what a reader with no approval gets when it lies outside the project.
    pub fn without_spec(mut self) -> Self {
        self.spec = None;
        self
    }

    /// The spec root lands outside the project root (ADR 0145 D4), its `..` and its links followed: a project's roots
    /// then name it as an outside root, under the approval a named one needs.
    pub fn spec_outside(&self) -> bool {
        self.spec
            .as_ref()
            .is_some_and(|spec| !RealFs::landing(spec).starts_with(RealFs::landing(&self.project)))
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

/// Where `path` lands: made absolute when it has no root (a rooted path keeps the drive it names or lacks, so a
/// Windows `\p` is not given the current drive and stays comparable with a root spelled the same way; a link target
/// written `\x` with no drive then lands outside a root that names one: a refusal, never an escape), then walked
/// component by component. `..` leaves the real directory reached so far (after its links); every existing component
/// is replaced by its canonical form; a link whose target does not exist is replaced by its target (read against the
/// link's directory) and the walk goes on, at most [`MAX_LINKS`] times (then the name lands at itself). A missing
/// tail is joined on.
pub fn landing(lookup: &dyn Lookup, path: &Path) -> PathBuf {
    let path = if path.has_root() {
        path.to_path_buf()
    } else {
        std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
    };
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
    /// A bare path that names nothing (`""`, `.`, `a/..`); a written `<root>` names the root.
    Empty,
}

/// A written path its root holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub root: Root,
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
    /// The name, or where it lands, is on the credential deny-list or in the host's own state (ADR 0293 D3a).
    pub withheld: bool,
    /// It, or where it lands, is in the host's own state ([`HOST_STATE`]): no `Fs` request reaches it.
    pub host_state: bool,
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

    /// `written`, in the root its `<name>/` prefix names, else in `default`: a `\\` is read as `/` and told (W175), and
    /// a prefix [`RootedPath::parse`] refuses names no root (E095).
    pub fn resolve(&self, written: &str, default: Root) -> Result<Resolved, Refused> {
        let text = written.replace('\\', "/");
        let rooted = RootedPath::parse(&text, default).map_err(|problem| match problem {
            PrefixProblem::BadRootName { at } => Refused::UnknownRoot(text[at].to_string()),
            PrefixProblem::UnclosedRoot | PrefixProblem::Backslash { .. } => {
                Refused::UnknownRoot(text.clone())
            }
        })?;
        let mut resolved = self.resolve_rooted(&rooted)?;
        resolved.backslashes = written.contains('\\');
        Ok(resolved)
    }

    /// `rooted`, in the root it names.
    pub fn resolve_rooted(&self, rooted: &RootedPath) -> Result<Resolved, Refused> {
        let root = rooted.root.clone();
        let rest = rooted.path.as_str();
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
        // A bare path that comes to nothing names nothing; `<root>` names the root.
        if parts.is_empty() && rooted.written_root.is_none() {
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
        // Where it lands is judged from its root and the project: what lies above them may be named anything.
        let host_state = in_host_state(&rel)
            || real.as_ref().is_some_and(|real| {
                real_root
                    .as_deref()
                    .is_some_and(|root| under_host_state(real, root))
                    || (!self.roots.is_temp(&root)
                        && landed_in_host_state(&self.roots, self.lookup, real))
            });
        let withheld = host_state
            || credential(&rel)
            || real
                .as_ref()
                .is_some_and(|real| credential(&real.to_string_lossy().replace('\\', "/")));
        Ok(Resolved {
            root,
            rel,
            at,
            real,
            exists,
            case,
            backslashes: false,
            withheld,
            host_state,
        })
    }

    /// A close name to `resolved`'s in its parent, as written with that name, never a withheld one: the parent is one
    /// `resolve` held, so no directory outside a root is ever listed.
    pub fn sibling(&self, resolved: &Resolved) -> Option<String> {
        let (dir, name) = match resolved.rel.rsplit_once('/') {
            Some((dir, name)) => (Some(dir), name),
            None => (None, resolved.rel.as_str()),
        };
        let as_written = |name: &str| match dir {
            Some(dir) => format!("{dir}/{name}"),
            None => name.to_string(),
        };
        let names = self.names(resolved.at.parent()?)?;
        // A name withheld from a read is withheld from a suggestion: it would say the file is there.
        let shown = names.iter().map(String::as_str);
        let close = find_close_match(name, shown.filter(|name| !withheld(&as_written(name))))?;
        Some(as_written(close))
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

/// A root (`/`), a drive (`C:`) or a UNC prefix, on any operating system.
fn absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/')
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
        || Path::new(path).is_absolute()
}

/// The directory of the host's own state, wherever a root holds it: its effects journal keeps other extensions'
/// answers, which may have been authenticated, and a project read needs no approval.
pub const HOST_STATE: &str = ".specforge";

/// Whether `path` (`/` separated, from a root) has a [`HOST_STATE`] component, in any case.
pub fn in_host_state(path: &str) -> bool {
    path.split('/')
        .any(|part| part.eq_ignore_ascii_case(HOST_STATE))
}

/// Whether `real`, under `root`, is in the host's own state seen from `root`.
fn under_host_state(real: &Path, root: &Path) -> bool {
    real.strip_prefix(root)
        .is_ok_and(|rel| in_host_state(&rel.to_string_lossy().replace('\\', "/")))
}

/// Whether `real` lands in the project's own state: a named root can be a directory inside it.
pub(crate) fn landed_in_host_state(roots: &Roots, lookup: &dyn Lookup, real: &Path) -> bool {
    lookup
        .canonical(roots.project())
        .is_some_and(|project| under_host_state(real, &project))
}

/// Whether `path` (`/` separated, from a root) is not handed to a guest as text (ADR 0293 D3a): the host's own state,
/// or what looks like a credential.
pub fn withheld(path: &str) -> bool {
    in_host_state(path) || credential(path)
}

/// Files that hold a credential whatever their directory.
const CREDENTIAL_NAMES: [&str; 8] = [
    ".env",
    ".envrc",
    ".npmrc",
    ".netrc",
    ".pgpass",
    ".pypirc",
    ".dockercfg",
    ".git-credentials",
];

/// Files that hold a credential in this directory only: `config` and `config.json` alone are ordinary names.
const CREDENTIAL_PATHS: [&str; 3] = [".git/config", ".docker/config.json", ".kube/config"];

/// Whether `path` looks like it holds a credential: [`CREDENTIAL_NAMES`], [`CREDENTIAL_PATHS`], `.env.*`, `*.pem`,
/// `*.key`, `id_rsa*`, `id_ed25519*`, `id_ecdsa*`, `id_dsa*`, `credentials*`, and everything under `.ssh/` but `*.pub`,
/// `known_hosts` and `config`. shortcut: names only, and no project can opt a file in; upgrade when a project needs to.
pub fn credential(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    let name = path.rsplit('/').next().unwrap_or_default();
    let under =
        |dir: &str| path.starts_with(&format!("{dir}/")) || path.contains(&format!("/{dir}/"));
    CREDENTIAL_NAMES.contains(&name)
        || name.starts_with(".env.")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || ["id_rsa", "id_ed25519", "id_ecdsa", "id_dsa"]
            .iter()
            .any(|key| name.starts_with(key))
        || name.starts_with("credentials")
        || CREDENTIAL_PATHS
            .iter()
            .any(|at| path == *at || path.ends_with(&format!("/{at}")))
        || (under(".ssh") && !(name.ends_with(".pub") || name == "known_hosts" || name == "config"))
}
