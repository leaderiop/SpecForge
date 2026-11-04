//! The verbs a person or an agent types (ADR 0339): `specforge files`, `run`, `http` and `git`. Each is the user's own
//! act, under the user grant: every port and request, with only the project's effects ceiling narrowing it. A typed
//! path is read from the current directory, then held to the one root of the project this machine may reach that holds
//! it: `project`, `spec`, a named root of `specforge.json` (one outside the project once approved), or the verb-only
//! `temp`.

pub mod files;
pub mod run;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use specforge_common::{ProjectRoot, codes};
use specforge_effects::contained::TEMP;
use specforge_effects::grant::one_line;
use specforge_effects::{
    Consent, Denial, EffectRequest, FsOp, FsRequest, Grant, Ports, RealFs, Root, Roots,
};
use specforge_project::Environment;
use specforge_protocol_types::Operation;

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
    /// Each reachable root and where it lands, the deepest first: the root a typed path is read in is the first that
    /// holds it.
    landed: Vec<(Root, PathBuf)>,
    /// Where the `temp` root lies, made or not: `<project>/.specforge/tmp`, under the home directory for a loose root.
    temp: Option<PathBuf>,
}

/// What a verb does at a path: an fs request's op, or a write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    Fs(FsOp),
    Write,
}

impl Verbs {
    /// The verbs of the project at `root`, typed in `cwd`, performed through `ports`. A root outside the project is
    /// reachable once `consent` approved it.
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
        let mut landed: Vec<(Root, PathBuf)> = vec![(Root::Project, RealFs::landing(root.dir()))];
        if roots.get(&Root::Spec).is_some() {
            landed.push((Root::Spec, RealFs::landing(env.spec_root())));
        }
        for (name, path) in roots.named() {
            if let Some(named) = Root::parse(name) {
                landed.push((named, RealFs::landing(path)));
            }
        }
        let temp = temp_of(root);
        if let Some(at) = temp.as_deref().and_then(held_temp) {
            roots = roots.with_temp(at.clone());
            landed.push((Root::Named(TEMP.to_string()), at));
        }
        landed.sort_by_key(|(_, at)| std::cmp::Reverse(at.components().count()));
        Verbs {
            root: root.clone(),
            cwd: RealFs::landing(cwd),
            ports: ports.clone(),
            grant,
            roots,
            landed,
            temp,
        }
    }

    /// These verbs with the `temp` root made (owner only) and reachable: `Err` when it cannot be made, or lands
    /// outside the directory it is made in.
    pub fn with_temp(mut self) -> Result<Self, OpError> {
        if self.roots.get(&Root::Named(TEMP.to_string())).is_some() {
            return Ok(self);
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
        let made = std::fs::create_dir_all(&temp).and_then(|()| owner_only(&temp));
        let at = made.ok().and_then(|()| held_temp(&temp)).ok_or_else(|| {
            refused(format!(
                "the temp root {} cannot be made, or leads out of its directory",
                temp.display()
            ))
        })?;
        self.roots = self.roots.clone().with_temp(at.clone());
        self.landed.push((Root::Named(TEMP.to_string()), at));
        self.landed
            .sort_by_key(|(_, at)| std::cmp::Reverse(at.components().count()));
        Ok(self)
    }

    /// Where the `temp` root lies, links resolved, once made.
    pub fn temp_root(&self) -> Option<&Path> {
        self.roots.get(&Root::Named(TEMP.to_string()))
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

    /// The root that holds `typed` (read from the current directory) and the request for `op` there: the deepest
    /// root that holds it and whose `op` the ceiling permits. The root itself is the empty path. A path no reachable
    /// root holds, or one whose links lead out of every root, is refused, and so is one the ceiling leaves no root for
    /// (E090).
    pub fn held(&self, typed: &Path, op: FsOp) -> Result<(Located, FsRequest), OpError> {
        let (located, op) = self.located(typed, Access::Fs(op))?;
        let Access::Fs(op) = op else {
            unreachable!("an fs access is held as one")
        };
        let request = FsRequest {
            root: located.root.as_str().to_string(),
            path: located.rel.clone(),
            op,
        };
        Ok((located, request))
    }

    /// [`Self::held`] for any `access`: a write is permitted by the root's `write` op.
    pub fn located(&self, typed: &Path, access: Access) -> Result<(Located, Access), OpError> {
        let written = self.cwd.join(typed);
        // A write changes the name itself: a link is moved or removed, never what it points at.
        let at = match (&access, written.file_name(), written.parent()) {
            (Access::Write, Some(name), Some(parent)) => RealFs::landing(parent).join(name),
            _ => RealFs::landing(&written),
        };
        let mut refused = None;
        for (root, base) in &self.landed {
            let Ok(rel) = at.strip_prefix(base) else {
                continue;
            };
            let rel = slashed(rel);
            let permitted = match &access {
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
                    return Ok((located, access));
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
                 project is reachable once `specforge grants approve --root <name>` approved it",
            )
        }))
    }

    /// Whether the user grant permits `request`: `Err` is E090 with the ceiling's words.
    pub fn permit(&self, request: &EffectRequest) -> Result<(), OpError> {
        self.grant
            .permits(USER, request)
            .map_err(|denial| outside_ceiling(&one_line(request), &denial))
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

/// Where the `temp` root of the project at `root` lies: in the project's own state, or the home directory's for a
/// loose root (as its effects journal is); none for a loose root with no home.
fn temp_of(root: &ProjectRoot) -> Option<PathBuf> {
    specforge_effects::journal::state_base(root).map(|base| {
        base.join(specforge_effects::contained::HOST_STATE)
            .join("tmp")
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
        .join("tmp");
    (real.is_dir() && real == expected).then_some(real)
}

/// `dir` readable by its owner alone (`0700`).
fn owner_only(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
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
                "{shown} is not reachable ({other}): the host's own state and credential files are withheld"
            ),
        ),
    }
}
