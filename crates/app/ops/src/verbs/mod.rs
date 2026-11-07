//! The verbs a person or an agent types (ADR 0339): `specforge files`, `run`, `http`, `git` and `wait` (ADR 0342). Each
//! is the user's own act, under the user grant: every port and request, with only the project's effects ceiling
//! narrowing it. A typed path is read from the current directory, then held to the one root of the project this machine
//! may reach that holds it: `project`, `spec`, a named root of `specforge.json` (one outside the project once approved),
//! a root named on this machine (`grants approve --root <name> --at <dir>`), or the verb-only `temp`. The last two no
//! extension's grant holds.

pub mod files;
pub mod git;
pub mod http;
pub mod run;
pub mod wait;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use specforge_common::{ProjectRoot, codes};
use specforge_effects::contained::temp_root;
use specforge_effects::grant::one_line;
use specforge_effects::{
    Consent, Denial, EffectRequest, FsOp, FsRequest, Grant, Ports, RealFs, RecordDigest, Root,
    Roots, UnixMillis,
};
use specforge_project::Environment;
use specforge_protocol_types::Operation;

use crate::consent::machine_roots_here;
use crate::{OUTSIDE_ROOT, OpError, OpErrorKind};

/// The call kind a user grant is asked with: it applies no gate, so any kind would do.
const USER: Operation = Operation::Command;

/// What a verb works with: the project, the user grant under its ceiling, the roots this machine may reach, and the
/// ports.
pub struct Verbs {
    root: ProjectRoot,
    /// The current directory, links resolved: a typed path is read from it, and an answer's path printed from it.
    cwd: PathBuf,
    ports: Ports,
    grant: Arc<Grant>,
    roots: Roots,
    /// Where the project's spec root lands, reachable or not: what `git diff` calls a spec file is the project's word.
    spec_root: PathBuf,
    /// The names of the roots this machine named (`grants approve --root <name> --at <dir>`): `files` and `git` reach them,
    /// no other verb does.
    machine_roots: Vec<String>,
    /// Each reachable root and where it lands, the deepest first: the root a typed path is read in is the first that
    /// holds it.
    landed: Vec<(Root, PathBuf)>,
    /// Where the `temp` root lies, made or not: `<project>/.specforge/tmp`, under the home directory for a loose root.
    temp: Option<PathBuf>,
}

/// How a verb's answer was had (ADR 0339 D4): the request's record digest, when it was observed, and whether the
/// effects journal answered it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Effect {
    /// The key of the request's record in the effects journal; none for a local read or write (`files`, `git`), which
    /// no record holds (ADR 0337 D3).
    pub digest: Option<RecordDigest>,
    /// When the answer was observed: its record's time when the journal answered, else now.
    pub observed_at: UnixMillis,
    /// The answer is the journal's record, not the port's now.
    pub from_journal: bool,
}

impl Effect {
    /// A local read or write, performed now in every mode (ADR 0337 D4).
    pub fn performed() -> Self {
        Effect {
            digest: None,
            observed_at: UnixMillis::now_or_end(),
            from_journal: false,
        }
    }
}

/// A verb's answer and how it was had: `answer`'s fields, then `effect`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Answered<T> {
    #[serde(flatten)]
    pub answer: T,
    pub effect: Effect,
}

impl<T> Answered<T> {
    /// `answer` of a local read or write ([`Effect::performed`]).
    pub fn performed(answer: T) -> Self {
        Answered {
            answer,
            effect: Effect::performed(),
        }
    }
}

/// What a verb does at a path: an fs request's op, or a write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    Fs(FsOp),
    Write,
}

impl Verbs {
    /// The verbs of the project at `root`, typed in `cwd`, performed through `ports`. A root outside the project is
    /// reachable once `consent` approved it, and so is each machine root `consent` holds.
    pub fn open(root: &ProjectRoot, cwd: &Path, ports: &Ports, consent: &dyn Consent) -> Self {
        let env = Environment::load(root, None);
        let grant = Arc::new(Grant::user(&env.effects_allowed().ceiling));
        let reachable = specforge_project::effects::reachable_roots(&env, root.dir(), consent);
        let mut roots = Roots::new(root.dir(), env.spec_root());
        if !reachable.iter().any(|(name, _)| name == "spec") {
            roots = roots.without_spec();
        }
        for (name, path) in reachable.iter().filter(|(name, _)| name != "spec") {
            roots = roots.with_named(name.clone(), path.clone());
        }
        // A store that cannot be used names none. A project's own root keeps its name.
        let held = consent
            .list()
            .map_or_else(|_| Vec::new(), |held| held.machine_roots);
        let mut machine_roots = Vec::new();
        for machine in machine_roots_here(consent, &held, &reachable).reached {
            machine_roots.push(machine.root.clone());
            roots = roots.with_named(machine.root, machine.target);
        }
        let spec_root = RealFs::landing(env.spec_root());
        let mut landed: Vec<(Root, PathBuf)> = vec![(Root::Project, RealFs::landing(root.dir()))];
        if roots.get(&Root::Spec).is_some() {
            landed.push((Root::Spec, spec_root.clone()));
        }
        for (name, path) in roots.named() {
            if let Some(named) = Root::parse(name) {
                landed.push((named, RealFs::landing(path)));
            }
        }
        let temp = temp_of(root);
        if let Some(at) = temp.as_deref().and_then(held_temp) {
            roots = roots.with_temp(at.clone());
            landed.push((temp_root(), at));
        }
        landed.sort_by_key(|(_, at)| std::cmp::Reverse(at.components().count()));
        Verbs {
            root: root.clone(),
            cwd: RealFs::landing(cwd),
            ports: ports.clone(),
            grant,
            roots,
            spec_root,
            machine_roots,
            landed,
            temp,
        }
    }

    /// These verbs with the `temp` root made (owner only) and reachable, and where it lies, links resolved: `Err` when
    /// it cannot be made, or lands outside the directory it is made in.
    pub fn make_temp_root(mut self) -> Result<(Self, PathBuf), OpError> {
        if let Some(at) = self.temp_root() {
            let at = at.to_path_buf();
            return Ok((self, at));
        }
        let refused = |why: String| {
            OpError::new(
                OpErrorKind::PermissionDenied,
                OpErrorKind::PermissionDenied.as_str(),
                why,
            )
        };
        let Some(temp) = self.temp.clone() else {
            return Err(refused(
                "a directory with no specforge.json has its temp root under the home directory, and there is none"
                    .to_string(),
            ));
        };
        let made = make_temp(&temp);
        let at = made.ok().and_then(|()| held_temp(&temp)).ok_or_else(|| {
            refused(format!(
                "the temp root {} cannot be made, or leads out of its directory",
                temp.display()
            ))
        })?;
        self.roots = self.roots.clone().with_temp(at.clone());
        self.landed.push((temp_root(), at.clone()));
        self.landed
            .sort_by_key(|(_, at)| std::cmp::Reverse(at.components().count()));
        Ok((self, at))
    }

    /// Where the `temp` root lies, links resolved, once made.
    pub fn temp_root(&self) -> Option<&Path> {
        self.roots.get(&temp_root())
    }

    pub fn ports(&self) -> &Ports {
        &self.ports
    }

    pub fn root(&self) -> &ProjectRoot {
        &self.root
    }

    /// The current directory, links resolved: where a typed path is read from and a program runs.
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn roots(&self) -> &Roots {
        &self.roots
    }

    /// Where the project's spec root lands, links resolved.
    pub fn spec_root(&self) -> &Path {
        &self.spec_root
    }

    /// The root that holds `typed` (read from the current directory) and the request for `op` there: the deepest
    /// root that holds it and whose `op` the ceiling permits. The root itself is the empty path. A path no reachable
    /// root holds, or one whose links lead out of every root, is refused, and so is one the ceiling leaves no root for
    /// (E090).
    pub fn held(&self, typed: &Path, op: FsOp) -> Result<(Located, FsRequest), OpError> {
        let located = self.located(typed, &Access::Fs(op.clone()))?;
        let request = FsRequest {
            root: located.root.as_str().to_string(),
            path: located.rel.clone(),
            op,
        };
        Ok((located, request))
    }

    /// [`Self::held`] for any `access`: a write is permitted by the root's `write` op.
    pub fn located(&self, typed: &Path, access: &Access) -> Result<Located, OpError> {
        let written = self.cwd.join(typed);
        // A write changes the name itself: a link is moved or removed, never what it points at.
        let at = match (access, written.file_name(), written.parent()) {
            (Access::Write, Some(name), Some(parent)) => RealFs::landing(parent).join(name),
            _ => RealFs::landing(&written),
        };
        if *access == Access::Write {
            refuse_git_directory(&at)?;
        }
        let mut refused = None;
        for (root, base) in &self.landed {
            let Ok(rel) = at.strip_prefix(base) else {
                continue;
            };
            let rel = slashed(rel);
            let permitted = match access {
                Access::Fs(op) => self.permit_fs(&FsRequest {
                    root: root.as_str().to_string(),
                    path: rel.clone(),
                    op: op.clone(),
                }),
                Access::Write => self.permit_write(root, &rel),
            };
            match permitted {
                Ok(()) => {
                    let located = Located {
                        root: root.clone(),
                        rel,
                        at,
                    };
                    return Ok(located);
                }
                Err(error) => {
                    refused.get_or_insert(error);
                }
            }
        }
        Err(refused.unwrap_or_else(|| {
            OpError::new(
                OpErrorKind::PermissionDenied,
                OUTSIDE_ROOT,
                format!(
                    "{} is in no root of the project at {}",
                    typed.display(),
                    self.root.dir().display()
                ),
            )
            .with_suggestion(
                "name a path inside the project, or in a named root of specforge.json `roots`; one outside the \
                 project is reachable once `specforge grants approve --root <name>` approved it, and `specforge \
                 grants approve --root <name> --at <dir>` names a root for every project on this machine",
            )
        }))
    }

    /// Refused when only a machine root holds `located`: `verb` is no `files` or `git`, the two a machine root is for.
    pub(crate) fn beyond_machine_roots(
        &self,
        located: &Located,
        verb: &str,
    ) -> Result<(), OpError> {
        let held = self.landed.iter().any(|(root, base)| {
            !self.machine_roots.iter().any(|name| name == root.as_str())
                && located.at.starts_with(base)
        });
        if held {
            return Ok(());
        }
        Err(OpError::new(
            OpErrorKind::PermissionDenied,
            OUTSIDE_ROOT,
            format!(
                "{} is in the machine root `{}`, which `specforge files` and `specforge git` reach, and `{verb}` does not",
                self.shown(&located.at),
                located.root.as_str()
            ),
        )
        .with_suggestion(format!(
            "`{verb}` from the directory itself, or name it in specforge.json `roots` for this project"
        )))
    }

    /// Whether the user grant permits `request`: `Err` is E090 with the ceiling's words.
    pub fn permit(&self, request: &EffectRequest) -> Result<(), OpError> {
        self.grant
            .permits(USER, request)
            .map_err(|denial| outside_ceiling(&one_line(request), &denial))
    }

    /// Whether the user grant permits a request to `url` through the proxy on `host` (the environment's or the
    /// system's): the proxy sees what the request carries, so its host is one the grant must reach, as the URL's is.
    /// `Err` is E090 with the remedy.
    pub fn permit_proxy(&self, host: &str, url: &str) -> Result<(), OpError> {
        let proxy = EffectRequest::Resolve(specforge_protocol_types::ResolveRequest {
            host: host.to_string(),
            allow_private: false,
            timeout_ms: 0,
        });
        self.grant.permits(USER, &proxy).map_err(|denial| {
            OpError::coded(
                OpErrorKind::PermissionDenied,
                codes::E090,
                format!(
                    "{} goes through the proxy on {host} (HTTP_PROXY, HTTPS_PROXY, ALL_PROXY or the system's \
                     settings), which is outside what the project allows: {}. Add {host} to `effects.http.hosts` \
                     in specforge.json, or name the URL's host in NO_PROXY",
                    specforge_common::remedy::printable(url),
                    denial.detail
                ),
            )
        })
    }

    /// Whether the user grant permits writing `rel` in `root`: `Err` is E090 with the ceiling's words.
    pub fn permit_write(&self, root: &Root, rel: &str) -> Result<(), OpError> {
        self.grant
            .permits_write(USER, root.as_str(), rel)
            .map_err(|denial| outside_ceiling(&denial.request, &denial))
    }

    /// [`Self::permit`] for a file request.
    pub fn permit_fs(&self, request: &FsRequest) -> Result<(), OpError> {
        self.permit(&EffectRequest::Fs(request.clone()))
    }

    /// `path` (absolute) as an answer prints it: from the current directory when under it, else absolute; `/`
    /// separated.
    pub fn shown(&self, path: &Path) -> String {
        match path.strip_prefix(&self.cwd) {
            Ok(rel) if rel.as_os_str().is_empty() => ".".to_string(),
            Ok(rel) => slashed(rel),
            Err(_) => slashed(path),
        }
    }

    /// Where `rel` in `root` lies, as [`Self::shown`] prints it.
    pub fn shown_in(&self, root: &Root, rel: &str) -> String {
        self.shown_named(root.as_str(), rel)
    }

    /// [`Self::shown_in`] for the root named `root`, as a port's answer names it.
    pub fn shown_named(&self, root: &str, rel: &str) -> String {
        let base = self
            .landed
            .iter()
            .find(|(r, _)| r.as_str() == root)
            .map(|(_, at)| at.clone())
            .unwrap_or_default();
        let at = if rel.is_empty() { base } else { base.join(rel) };
        self.shown(&at)
    }
}

/// Refused when `at`, a place a verb is to write, has a `.git` component: a hook put there runs on the next commit, which
/// would be code execution for a caller limited to the verbs. The git verbs' own writes are the port's, not through here.
pub(crate) fn refuse_git_directory(at: &Path) -> Result<(), OpError> {
    let in_git = at
        .components()
        .any(|part| part.as_os_str().eq_ignore_ascii_case(".git"));
    if !in_git {
        return Ok(());
    }
    Err(OpError::new(
        OpErrorKind::PermissionDenied,
        OpErrorKind::PermissionDenied.as_str(),
        format!(
            "{} is in a .git directory, which no verb writes: a hook put there would run on the next commit",
            at.display()
        ),
    )
    .with_suggestion(
        "change a repository through specforge git; install a hook or edit its config with the git program",
    ))
}

/// E090: `what` is outside what the project's effects ceiling allows.
fn outside_ceiling(what: &str, denial: &Denial) -> OpError {
    OpError::coded(
        OpErrorKind::PermissionDenied,
        codes::E090,
        format!(
            "`{what}` is outside what the project allows: {}",
            denial.detail
        ),
    )
}

/// The `temp` root's directory in the host's state (`.specforge/tmp`).
const TEMP_DIR: &str = "tmp";

/// Where the `temp` root of the project at `root` lies: in the project's own state, or the home directory's for a
/// loose root (as its effects journal is); none for a loose root with no home.
fn temp_of(root: &ProjectRoot) -> Option<PathBuf> {
    specforge_effects::journal::state_base(root).map(|base| {
        base.join(specforge_effects::contained::HOST_STATE)
            .join(TEMP_DIR)
    })
}

/// `temp` with its links resolved, when it is a directory that lands where it is written: a `.specforge` that is a
/// link out of its directory is never written through.
fn held_temp(temp: &Path) -> Option<PathBuf> {
    let base = temp.parent()?.parent()?;
    let real = std::fs::canonicalize(temp).ok()?;
    let expected = std::fs::canonicalize(base)
        .ok()?
        .join(specforge_effects::contained::HOST_STATE)
        .join(TEMP_DIR);
    (real.is_dir() && real == expected).then_some(real)
}

/// `temp` (`<base>/.specforge/tmp`) made, owner only, with no link on the way: `.specforge` and `tmp` are each looked
/// at before anything is made or its mode set, a link or another kind refused, and the mode is set through a handle
/// that refuses a link. A committed `.specforge/tmp -> ~/public_html` is never made or chmod-ed.
fn make_temp(temp: &Path) -> std::io::Result<()> {
    for dir in [temp.parent(), Some(temp)].into_iter().flatten() {
        match std::fs::symlink_metadata(dir) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => {
                return Err(std::io::Error::other(format!(
                    "{} is a link or not a directory",
                    dir.display()
                )));
            }
            // `mkdir` never follows a link at the name: one made since the look above fails it.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => std::fs::create_dir(dir)?,
            Err(error) => return Err(error),
        }
    }
    specforge_common::fresh::owner_only_dir(temp)
}

/// A typed path, located.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    pub root: Root,
    /// Relative to the root, `/` separated.
    pub rel: String,
    /// Where it lands, links resolved.
    pub at: PathBuf,
}

/// `path` with `/` between its components.
fn slashed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// A refusal of an fs answer of `class` for the path `shown`.
fn refused(class: specforge_effects::OutcomeClass, shown: &str) -> OpError {
    use specforge_effects::OutcomeClass;
    match class {
        OutcomeClass::Outside => OpError::new(
            OpErrorKind::PermissionDenied,
            OUTSIDE_ROOT,
            format!("{shown} leads out of its root"),
        ),
        OutcomeClass::Missing => OpError::new(
            OpErrorKind::FileNotFound,
            OpErrorKind::FileNotFound.as_str(),
            format!("{shown} does not exist"),
        ),
        other => OpError::new(
            OpErrorKind::PermissionDenied,
            OpErrorKind::PermissionDenied.as_str(),
            format!(
                "{shown} is not reachable ({other}): the host's own state and credential files are withheld, a file with more than one name (a hard link) is not read, so copy it in as a plain file, and a file the system denies stays unread"
            ),
        ),
    }
}
