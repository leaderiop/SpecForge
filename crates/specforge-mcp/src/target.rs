//! The project one MCP call acts on (CONTEXT.md "Call target", ADR 0014).
//!
//! A call's optional `path` and its entry's [`TargetSpec`] (reach and
//! freshness) are resolved into one [`CallTarget`] before the handler runs:
//! the served session, brought up to date with disk unless the call asks
//! for the last compile; another project, compiled for this call only; or
//! the directory `init` creates. Handlers read their project as a
//! [`ProjectRef`] and cannot tell which adapter answered it; they never
//! resolve a root, pick a freshness rule or reload anything themselves.

use std::cell::OnceCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use specforge_common::{Diagnostic, find_project_root};
use specforge_graph::Graph;
use specforge_ops::analyze::ProjectView;
use specforge_project::{CompiledProject, Environment, Origin, ProjectSession, SharedRuntime};

use crate::state::McpState;
use crate::tool::{ErrorCode, McpError};

/// Which project a tool may act on, declared on its table entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    /// Reads nothing of a project (explain).
    Unscoped,
    /// The served project only: no `path` argument.
    Served,
    /// The served project or, by `path`, another one compiled for this call.
    AnyProject,
    /// As `AnyProject`; the tool writes the target's files.
    WritesAnyProject,
    /// `path` names a directory to create a project in (init).
    NewProject,
}

/// Whether the target is brought up to date with disk before the handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness {
    /// Always (`ProjectSession::ensure_fresh`).
    Fresh,
    /// Unless the call passes `use_cached: true` (validate, analyze, doctor).
    FreshUnlessCached,
}

/// How a tool, resource or prompt reaches its project.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetSpec {
    pub reach: Reach,
    pub freshness: Freshness,
}

impl TargetSpec {
    pub const fn new(reach: Reach, freshness: Freshness) -> Self {
        TargetSpec { reach, freshness }
    }

    /// The served project, brought up to date: every read-only tool, every
    /// extension tool, resource and prompt.
    pub const SERVED: TargetSpec = TargetSpec::new(Reach::Served, Freshness::Fresh);

    /// No project at all.
    pub const UNSCOPED: TargetSpec = TargetSpec::new(Reach::Unscoped, Freshness::Fresh);
}

/// The project a call acts on, resolved before its handler runs.
pub enum CallTarget {
    /// The served session (adopted if nothing was served).
    Served,
    /// Compiled for this call; the server keeps serving its own.
    Other(Box<OtherProject>),
    /// The directory `init` creates a project in.
    New(PathBuf),
    /// The tool reads no project.
    Unscoped,
    /// Nothing is served and the call names no project: a handler that
    /// needs one refuses ([`Call::project`]).
    NoProject,
}

/// Another project, compiled for one call, with the one runtime it was
/// compiled in.
pub struct OtherProject {
    root: PathBuf,
    project: CompiledProject,
    runtime: SharedRuntime,
    /// The runtime was built for this project (not the host's), so a
    /// recompile builds a fresh one: the call may have changed its
    /// extensions.
    owns_runtime: bool,
}

impl OtherProject {
    /// Compile the project at `root`, its extensions running in `host`
    /// when the server has one, else in a runtime of the project's own.
    fn compile(root: PathBuf, host: Option<&SharedRuntime>) -> Self {
        let (runtime, owns_runtime) = match host {
            Some(host) => (Arc::clone(host), false),
            None => (project_runtime(&root), true),
        };
        let project = CompiledProject::compile(&root, Some(runtime.as_ref()));
        OtherProject {
            root,
            project,
            runtime,
            owns_runtime,
        }
    }

    /// Compile again after the call wrote files: what `specforge check`
    /// reports for the project now.
    fn recompile(&mut self) {
        if self.owns_runtime {
            self.runtime = project_runtime(&self.root);
        }
        self.project = CompiledProject::compile(&self.root, Some(self.runtime.as_ref()));
    }
}

fn project_runtime(root: &Path) -> SharedRuntime {
    Arc::new(specforge_component::project_runtime(root))
}

/// What a project's diagnostics are read from.
#[derive(Clone, Copy)]
enum Reported<'a> {
    /// The served session, then what registering its surfaces with MCP
    /// reported.
    Session(&'a ProjectSession, &'a [Diagnostic]),
    /// A one-shot compile.
    Compiled(&'a CompiledProject),
}

/// What every handler reads, from either adapter: the served session or a
/// project compiled for the call.
pub struct ProjectRef<'a> {
    /// The project root (where `specforge.json` lives).
    pub root: &'a Path,
    /// Where its `.spec` files are keyed from: spans are relative to it.
    pub spec_root: &'a Path,
    pub env: &'a Environment,
    pub graph: &'a Graph,
    /// The runtime its extensions run in.
    pub runtime: Option<&'a SharedRuntime>,
    reported: Reported<'a>,
}

impl<'a> ProjectRef<'a> {
    /// What an analysis of this project reads: the one way MCP builds a
    /// project view.
    pub fn view(&self) -> ProjectView<'a> {
        ProjectView::in_environment(self.env, self.graph, Some(self.root))
    }

    /// Everything the server reports for this project: what `specforge
    /// check` reports, then, for the served project, what registering its
    /// surfaces with MCP reported.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        match self.reported {
            Reported::Session(session, surfaces) => {
                let mut diagnostics = session.diagnostics();
                diagnostics.extend(surfaces.iter().cloned());
                diagnostics
            }
            Reported::Compiled(project) => project.diagnostics(),
        }
    }
}

/// Why a call's target could not be resolved: the call fails with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetError {
    /// `path` names nothing on disk: `file_not_found` on argument `path`.
    PathNotFound(PathBuf),
    /// A tool that serves only the served project was given another
    /// project's `path`: `invalid_input` on argument `path` (the dispatcher
    /// names the tool).
    OtherProjectRefused,
    /// `init` was asked to create a project inside the served one:
    /// `conflict`.
    InsideServed { dir: PathBuf, served: PathBuf },
}

impl From<TargetError> for McpError {
    fn from(error: TargetError) -> Self {
        match error {
            TargetError::PathNotFound(path) => McpError::new(
                ErrorCode::FileNotFound,
                format!("path not found: {}", path.display()),
            )
            .with_argument("path"),
            TargetError::OtherProjectRefused => McpError::new(
                ErrorCode::InvalidInput,
                "this tool acts on the project the server serves; path cannot name another project",
            )
            .with_argument("path"),
            TargetError::InsideServed { dir, served } => McpError::new(
                ErrorCode::Conflict,
                format!(
                    "{} is inside the current project at {}",
                    dir.display(),
                    served.display()
                ),
            )
            .with_argument("path"),
        }
    }
}

/// The refusal of a handler that needs a project when none is served.
pub fn no_project() -> McpError {
    McpError::new(
        ErrorCode::PreconditionFailed,
        "no project is served: pass {\"path\": ...} or start the server in a project",
    )
}

/// One call: the state, its target, and what the handler wrote.
pub struct Call<'s> {
    pub state: &'s mut McpState,
    target: CallTarget,
    wrote: bool,
    /// The runtime of a served project built in memory, which has none of
    /// its own: the host's, else one built for its root on first use.
    in_memory_runtime: OnceCell<SharedRuntime>,
}

impl<'s> Call<'s> {
    /// A call on `target`, resolved by [`resolve`].
    pub fn new(state: &'s mut McpState, target: CallTarget) -> Self {
        Call {
            state,
            target,
            wrote: false,
            in_memory_runtime: OnceCell::new(),
        }
    }

    /// The project the call acts on. `NoProject` (and a target that is no
    /// project) is `precondition_failed`: tools return it with `?` through
    /// `From<McpError> for ToolOutcome`, prompts as their refusal.
    #[allow(
        clippy::result_large_err,
        reason = "the refusal is the McpError tools and prompts return as is (ADR 0014)"
    )]
    pub fn project(&self) -> Result<ProjectRef<'_>, McpError> {
        match &self.target {
            CallTarget::Served => {
                let session = self.state.session();
                let root = session.root().ok_or_else(no_project)?;
                let runtime = match session.runtime() {
                    Some(runtime) => Some(runtime),
                    None => Some(self.in_memory_runtime.get_or_init(|| {
                        match &self.state.extension_runtime {
                            Some(host) => Arc::clone(host),
                            None => project_runtime(root),
                        }
                    })),
                };
                Ok(ProjectRef {
                    root,
                    spec_root: &session.environment().spec_root,
                    env: session.environment(),
                    graph: session.graph(),
                    runtime,
                    reported: Reported::Session(session, &self.state.surface_diagnostics),
                })
            }
            CallTarget::Other(other) => Ok(ProjectRef {
                root: &other.root,
                spec_root: &other.project.env.spec_root,
                env: &other.project.env,
                graph: &other.project.graph,
                runtime: Some(&other.runtime),
                reported: Reported::Compiled(&other.project),
            }),
            CallTarget::New(_) | CallTarget::Unscoped | CallTarget::NoProject => Err(no_project()),
        }
    }

    pub fn target(&self) -> &CallTarget {
        &self.target
    }

    /// The root of the project the call reads, when it has one: the served
    /// project's, or the one its path names. `None` with no project served
    /// (a tool that answers without a project reads the empty session).
    pub fn root(&self) -> Option<&Path> {
        match &self.target {
            CallTarget::Served => self.state.session().root(),
            CallTarget::Other(other) => Some(&other.root),
            CallTarget::New(_) | CallTarget::Unscoped | CallTarget::NoProject => None,
        }
    }

    /// Where the `.spec` files of the project the call reads are keyed
    /// from, when it has a root ([`Self::root`]).
    pub fn spec_root(&self) -> Option<&Path> {
        match &self.target {
            CallTarget::Served => self
                .state
                .session()
                .root()
                .map(|_| self.state.environment().spec_root.as_path()),
            CallTarget::Other(other) => Some(&other.project.env.spec_root),
            CallTarget::New(_) | CallTarget::Unscoped | CallTarget::NoProject => None,
        }
    }

    /// The directory `init` creates its project in.
    pub fn new_project_dir(&self) -> Option<&Path> {
        match &self.target {
            CallTarget::New(dir) => Some(dir),
            _ => None,
        }
    }

    /// The handler wrote its target's files: bring the target up to date
    /// now and return what `specforge check` reports for it. The served
    /// project is brought up to date with disk (a project built in memory
    /// is replaced by the project on disk at its root); another project is
    /// compiled again; the server keeps serving its own.
    pub fn wrote(&mut self) -> Vec<Diagnostic> {
        self.wrote = true;
        match &mut self.target {
            CallTarget::Served => {
                match (self.state.session().origin(), self.state.project_root()) {
                    (Origin::Disk, _) => {
                        self.state.ensure_fresh();
                    }
                    (_, Some(root)) => {
                        let root = root.to_path_buf();
                        self.state.serve(&root);
                    }
                    (_, None) => {}
                }
                self.state.diagnostics()
            }
            CallTarget::Other(other) => {
                other.recompile();
                other.project.diagnostics()
            }
            CallTarget::New(_) | CallTarget::Unscoped | CallTarget::NoProject => Vec::new(),
        }
    }

    /// Whether the handler reported writing its target ([`Self::wrote`]).
    pub fn has_written(&self) -> bool {
        self.wrote
    }
}

/// Resolve the call's `path` (from `arguments`) under `spec`, then apply
/// the freshness policy to a served target.
///
/// A `path` is canonical, then the nearest enclosing project (a directory
/// with `specforge.json` or `specforge.spec`), else the directory itself;
/// one that does not exist is `file_not_found`. A path while nothing is
/// served is adopted: the server serves it, and the call acts on it as the
/// served project. A path naming the served project (or a directory inside
/// it) is the served project; any other is compiled for this call only.
/// `init`'s path is used as given (it creates it), and may not lie inside
/// the served project.
pub fn resolve(
    state: &mut McpState,
    spec: TargetSpec,
    arguments: &Value,
) -> Result<CallTarget, TargetError> {
    let path = arguments.get("path").and_then(Value::as_str);
    let cached = spec.freshness == Freshness::FreshUnlessCached
        && arguments.get("use_cached").and_then(Value::as_bool) == Some(true);
    let served = |state: &mut McpState| {
        if state.project_root().is_none() {
            return CallTarget::NoProject;
        }
        if !cached {
            state.ensure_fresh();
        }
        CallTarget::Served
    };
    match spec.reach {
        Reach::Unscoped => Ok(CallTarget::Unscoped),
        Reach::NewProject => {
            let Some(path) = path else {
                // The handler refuses the missing argument.
                return Ok(CallTarget::Unscoped);
            };
            let dir = PathBuf::from(path);
            if let Some(root) = state.project_root() {
                let (inside, root) = (absolute(&dir), absolute(root));
                if inside.starts_with(&root) {
                    return Err(TargetError::InsideServed { dir, served: root });
                }
            }
            Ok(CallTarget::New(dir))
        }
        Reach::Served | Reach::AnyProject | Reach::WritesAnyProject => {
            let Some(path) = path else {
                return Ok(served(state));
            };
            let root = project_at(Path::new(path))?;
            let served_root = state.project_root().map(absolute);
            if served_root.is_none() && spec.reach != Reach::Served {
                // Nothing served: the call serves this project, as it is
                // on disk now.
                state.serve(&root);
                return Ok(CallTarget::Served);
            }
            if served_root.as_deref() == Some(root.as_path()) {
                return Ok(served(state));
            }
            if spec.reach == Reach::Served {
                return Err(TargetError::OtherProjectRefused);
            }
            Ok(CallTarget::Other(Box::new(OtherProject::compile(
                root,
                state.extension_runtime.as_ref(),
            ))))
        }
    }
}

/// The project `path` names: canonical, then the nearest enclosing project,
/// else the directory itself.
fn project_at(path: &Path) -> Result<PathBuf, TargetError> {
    let canonical =
        std::fs::canonicalize(path).map_err(|_| TargetError::PathNotFound(path.to_path_buf()))?;
    Ok(find_project_root(&canonical).unwrap_or(canonical))
}

/// `path` made absolute and canonical as far as it exists (a directory
/// `init` is about to create need not).
fn absolute(path: &Path) -> PathBuf {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
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
    let mut absolute = std::fs::canonicalize(&existing).unwrap_or(existing);
    absolute.extend(rest.into_iter().rev());
    absolute
}
