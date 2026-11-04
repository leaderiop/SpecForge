//! The verbs a person or an agent types (ADR 0339): `specforge files`, `run`, `http` and `git`. Each is the user's own
//! act, under the user grant: every port and request, with only the project's effects ceiling narrowing it. A typed
//! path is read from the current directory, then held to the one root of the project this machine may reach that holds
//! it: `project`, `spec`, a named root of `specforge.json` (one outside the project once approved), or the verb-only
//! `temp`.

pub mod files;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use specforge_common::{ProjectRoot, codes};
use specforge_effects::grant::one_line;
use specforge_effects::{
    Consent, EffectRequest, FsOp, FsRequest, Grant, Ports, RealFs, Root, Roots,
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
        landed.sort_by_key(|(_, at)| std::cmp::Reverse(at.components().count()));
        Verbs {
            root: root.clone(),
            cwd: RealFs::landing(cwd),
            ports: ports.clone(),
            grant,
            roots,
            landed,
        }
    }

    pub fn ports(&self) -> &Ports {
        &self.ports
    }

    pub fn roots(&self) -> &Roots {
        &self.roots
    }

    /// The root that holds `typed` (read from the current directory) and the request for `op` there: the deepest
    /// root that holds it and whose `op` the ceiling permits. The root itself is the empty path. A path no reachable
    /// root holds, or one whose links lead out of every root, is refused, and so is one the ceiling leaves no root for
    /// (E090).
    pub fn held(&self, typed: &Path, op: FsOp) -> Result<(Located, FsRequest), OpError> {
        let at = RealFs::landing(&self.cwd.join(typed));
        let mut refused = None;
        for (root, base) in &self.landed {
            let Ok(rel) = at.strip_prefix(base) else {
                continue;
            };
            let request = FsRequest {
                root: root.as_str().to_string(),
                path: slashed(rel),
                op: op.clone(),
            };
            match self.permit_fs(&request) {
                Ok(()) => {
                    let located = Located {
                        root: root.clone(),
                        rel: request.path.clone(),
                        at,
                    };
                    return Ok((located, request));
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
        self.grant.permits(USER, request).map_err(|denial| {
            OpError::coded(
                OpErrorKind::PermissionDenied,
                codes::E090,
                format!(
                    "`{}` is outside what the project allows: {}",
                    one_line(request),
                    denial.detail
                ),
            )
        })
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
        let base = self
            .landed
            .iter()
            .find(|(r, _)| r == root)
            .map(|(_, at)| at.clone())
            .unwrap_or_default();
        let at = if rel.is_empty() { base } else { base.join(rel) };
        self.shown(&at)
    }
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
