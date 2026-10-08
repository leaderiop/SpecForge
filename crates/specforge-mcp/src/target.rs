//! The project one MCP call acts on (CONTEXT.md "Call target", ADR 0014).
//!
//! A call's optional `path` and its entry's [`TargetSpec`] (reach and
//! freshness) are resolved into one [`CallTarget`] before the handler runs:
//! the served session, brought up to date with disk unless the call asks
//! for the last compile; another project, compiled for this call only; or
//! the directory `init` creates. Handlers read their project as a
//! [`ProjectRef`] and cannot tell which adapter answered it; they never
//! resolve a root, pick a freshness rule or reload anything themselves.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use specforge_common::{Diagnostic, project_root_of};
use specforge_graph::Graph;
use specforge_ops::view::ProjectView;
use specforge_project::{CompiledProject, SharedRuntime};

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

impl Reach {
    /// Whether an entry of this reach names its project by a `path`
    /// argument: the one predicate for the input schema and for what a
    /// no-project refusal tells the client to pass.
    pub(crate) fn takes_path(self) -> bool {
        matches!(
            self,
            Reach::AnyProject | Reach::WritesAnyProject | Reach::NewProject
        )
    }
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

    /// Whether the call names its project by a `path` argument.
    fn takes_path(self) -> bool {
        self.reach.takes_path()
    }

    /// Whether the call may ask for the last compile with `use_cached`.
    fn takes_use_cached(self) -> bool {
        self.freshness == Freshness::FreshUnlessCached
    }

    /// The input-schema properties the target reads from a call: `path` for
    /// a reach that names a project by it (`AnyProject`, `WritesAnyProject`:
    /// "Project root path (uses initialized root if omitted)"; `NewProject`:
    /// "Directory for the new project, outside the current one"), and
    /// `use_cached` for `FreshUnlessCached`. A tool's listed schema is its
    /// own properties plus these ([`ToolSpec::input_schema`]); its handler
    /// never reads them.
    pub fn properties(self) -> serde_json::Map<String, Value> {
        let mut properties = serde_json::Map::new();
        if self.takes_path() {
            let description = match self.reach {
                Reach::NewProject => "Directory for the new project, outside the current one",
                _ => "Project root path (uses initialized root if omitted)",
            };
            properties.insert(
                "path".into(),
                serde_json::json!({ "type": "string", "description": description }),
            );
        }
        if self.takes_use_cached() {
            properties.insert(
                "use_cached".into(),
                serde_json::json!({
                    "type": "boolean",
                    "description": "Use the last compile instead of bringing the project up to date with disk; with no project served, the path is compiled anyway",
                    "default": false,
                }),
            );
        }
        properties
    }

    /// The arguments a call cannot be made without: `["path"]` for
    /// `NewProject`, refused by [`resolve`] before the handler runs.
    pub fn required(self) -> &'static [&'static str] {
        match self.reach {
            Reach::NewProject => &["path"],
            _ => &[],
        }
    }

    /// Every name the target reads from a call, listed or not: `path` for
    /// every reach but `Unscoped` (a `Served` entry accepts its own
    /// project's root and refuses another's,
    /// [`TargetError::OtherProjectRefused`]), and `use_cached` for
    /// `FreshUnlessCached`. [`Self::fields`] stays the listed ones.
    pub fn accepted(self) -> &'static [&'static str] {
        match (self.reach != Reach::Unscoped, self.takes_use_cached()) {
            (true, true) => &["path", "use_cached"],
            (true, false) => &["path"],
            (false, true) => &["use_cached"],
            (false, false) => &[],
        }
    }

    /// The names [`Self::properties`] declares, for the schema drift test.
    pub fn fields(self) -> &'static [&'static str] {
        match (self.takes_path(), self.takes_use_cached()) {
            (true, true) => &["path", "use_cached"],
            (true, false) => &["path"],
            (false, true) => &["use_cached"],
            (false, false) => &[],
        }
    }
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
    /// needs one refuses ([`Call::project`]). The reach of the entry it was
    /// resolved for says what would fix it ([`no_project`]).
    NoProject(Reach),
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
            None => (own_runtime(), true),
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
            self.runtime = own_runtime();
        }
        self.project = CompiledProject::compile(&self.root, Some(self.runtime.as_ref()));
    }
}

fn own_runtime() -> SharedRuntime {
    Arc::new(specforge_component::ComponentRuntime::with_user_cache())
}

/// What every handler reads, from either adapter: the served session or a
/// project compiled for the call.
pub struct ProjectRef<'a> {
    /// The project root (where `specforge.json` lives).
    pub root: &'a Path,
    /// The runtime its extensions run in: every project a call reaches has
    /// one (the served session's, the host's or the project's own; the
    /// one-shot compile's for another project), so an extension call never
    /// finds none (ADR 0017).
    pub runtime: &'a SharedRuntime,
    /// The project view, built once for the call: rooted at the project
    /// root, reporting what the server reports for the project.
    view: ProjectView<'a>,
}

impl<'a> ProjectRef<'a> {
    /// The project's graph, the view's.
    pub fn graph(&self) -> &'a Graph {
        self.view.graph()
    }

    /// Where its `.spec` files are keyed from: spans are relative to it.
    pub fn spec_root(&self) -> &'a Path {
        &self.view.env().spec_root
    }

    /// What every operation over this project reads, rooted at the project
    /// root: the one way MCP builds a project view. Its recorded test
    /// report and coverage are memoized by its owner (the served session,
    /// or the project compiled for this call); it reports what `specforge
    /// check` reports, then, for the served project, the contributions of
    /// its extensions MCP does not serve under their names (I017).
    pub fn view(&self) -> ProjectView<'a> {
        self.view
    }

    /// Everything the server reports for this project
    /// ([`ProjectView::reported`]).
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.view.reported()
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
    /// `init` was called without the `path` it creates: `invalid_input`
    /// "Missing required parameter: path" on argument `path`.
    PathRequired,
    /// `path` or `use_cached` is not of its type (a string, a boolean):
    /// `invalid_input` on that argument, as every argument of the wrong
    /// type is refused.
    InvalidArgument {
        argument: &'static str,
        message: String,
    },
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
            TargetError::PathRequired => {
                McpError::new(ErrorCode::InvalidInput, "Missing required parameter: path")
                    .with_argument("path")
            }
            TargetError::InvalidArgument { argument, message } => {
                McpError::new(ErrorCode::InvalidInput, message).with_argument(argument)
            }
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

/// What a client can do about a missing project, ending every no-project
/// refusal: an entry that takes a `path` can pass one; one that serves only
/// the served project (a `Served` tool, a resource, a prompt) refuses a
/// `path`, so it says to start the server in a project.
fn serve_a_project(reach: Reach) -> &'static str {
    if reach.takes_path() {
        "pass {\"path\": ...} or start the server in a project"
    } else {
        "start the server in a project (`specforge mcp <root>`), or call a tool that takes a `path` first (ADR 0014 D5)"
    }
}

/// The refusal of a handler that needs a project when none is served, for
/// an entry of this `reach`. It names `path` only where the entry takes one:
/// the client can fix it, so a surface without `isError` answers it -32602;
/// for any other entry no params fix it, and it is a -32603 (ADR 0024 D5).
pub fn no_project(reach: Reach) -> McpError {
    let error = McpError::new(
        ErrorCode::PreconditionFailed,
        format!("no project is served: {}", serve_a_project(reach)),
    );
    if reach.takes_path() {
        error.with_argument("path")
    } else {
        error
    }
}

/// A refusal that names something of the project (a file, an entity), as
/// the call's target makes it: with nothing served ([`CallTarget::NoProject`])
/// a file or an entity is not "not found", there is no project to look in,
/// so the refusal is [`no_project`] (`precondition_failed`), its message
/// leading with what was asked, `entity_id` kept and `argument` dropped. Any
/// other refusal, and every refusal of a call that has a project, is
/// returned as it is. The one place that rule is applied: the dispatchers of
/// tools, prompts and resources call it on what their handler refused with
/// (ADR 0025).
pub(crate) fn without_project(target: &CallTarget, error: McpError) -> McpError {
    let CallTarget::NoProject(reach) = *target else {
        return error;
    };
    let asked = match error.code {
        ErrorCode::FileNotFound => match &error.file {
            Some(file) => format!("'{file}' is no project's file"),
            None => error.message.clone(),
        },
        ErrorCode::EntityNotFound => {
            let entity = error.entity_id.as_deref().unwrap_or("that entity");
            format!("entity '{entity}' is in no project")
        }
        _ => return error,
    };
    let mut refused = McpError::new(
        ErrorCode::PreconditionFailed,
        format!(
            "no project is served, so {asked}: {}",
            serve_a_project(reach)
        ),
    );
    if reach.takes_path() {
        refused.argument = Some("path".to_string());
    }
    refused.entity_id = error.entity_id;
    refused.tool = error.tool;
    refused.prompt = error.prompt;
    refused.uri = error.uri;
    refused.reported = error.reported;
    refused
}

/// [`without_project`], in place.
pub(crate) fn refuse_without_project(target: &CallTarget, error: &mut McpError) {
    let refused = std::mem::replace(
        error,
        McpError::new(ErrorCode::InternalError, String::new()),
    );
    *error = without_project(target, refused);
}

/// One call: the state and its target.
pub struct Call<'s> {
    pub state: &'s mut McpState,
    target: CallTarget,
}

impl<'s> Call<'s> {
    /// A call on `target`, resolved by [`resolve`].
    pub fn new(state: &'s mut McpState, target: CallTarget) -> Self {
        Call { state, target }
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
                // `McpState::serve` opens every served project from disk, with
                // a runtime: a session without either is no project (it is
                // unreachable, and a server answers rather than panics).
                let (Some(root), Some(runtime)) = (session.root(), session.runtime()) else {
                    return Err(no_project(Reach::Served));
                };
                Ok(ProjectRef {
                    root,
                    runtime,
                    view: ProjectView::of_session(session, Some(root))
                        .also_reporting(self.state.surfaces().diagnostics()),
                })
            }
            CallTarget::Other(other) => Ok(ProjectRef {
                root: &other.root,
                runtime: &other.runtime,
                // Rooted where it was compiled: `other.root`.
                view: ProjectView::of(&other.project),
            }),
            CallTarget::NoProject(reach) => Err(no_project(*reach)),
            CallTarget::New(_) => Err(no_project(Reach::NewProject)),
            CallTarget::Unscoped => Err(no_project(Reach::Unscoped)),
        }
    }

    pub fn target(&self) -> &CallTarget {
        &self.target
    }

    /// The project view of what the call reads: its project's
    /// ([`ProjectRef::view`]), else, with no project, the empty session's
    /// graph without a root: no recorded report, no schema cache; it
    /// reports what the server reports for the served session
    /// ([`McpState::diagnostics`]). For a read view that answers without a
    /// project.
    pub fn view(&self) -> ProjectView<'_> {
        match self.project() {
            Ok(project) => project.view(),
            Err(_) => ProjectView::of_session(self.state.session(), None)
                .also_reporting(self.state.surfaces().diagnostics()),
        }
    }

    /// The root of the project the call reads, when it has one: the served
    /// project's, or the one its path names. `None` with no project served
    /// (a tool that answers without a project reads the empty session).
    pub fn root(&self) -> Option<&Path> {
        match &self.target {
            CallTarget::Served => self.state.session().root(),
            CallTarget::Other(other) => Some(&other.root),
            CallTarget::New(_) | CallTarget::Unscoped | CallTarget::NoProject(_) => None,
        }
    }

    /// Where the `.spec` files of the project the call reads are keyed
    /// from, when it has a root ([`Self::root`]).
    pub fn spec_root(&self) -> Option<&Path> {
        match &self.target {
            CallTarget::Served => self.state.spec_root(),
            CallTarget::Other(other) => Some(&other.project.env.spec_root),
            CallTarget::New(_) | CallTarget::Unscoped | CallTarget::NoProject(_) => None,
        }
    }

    /// The directory `init` creates its project in.
    pub fn new_project_dir(&self) -> Option<&Path> {
        match &self.target {
            CallTarget::New(dir) => Some(dir),
            _ => None,
        }
    }

    /// The call wrote its target's files: bring the target up to date now
    /// and return what `specforge check` reports for it. Called only by
    /// [`crate::mutation::refresh`] (ADR 0022). The served project is
    /// brought up to date with disk; another project is compiled again (the
    /// server keeps serving its own); the directory `init` created is
    /// served when nothing is (ADR 0014 D5).
    pub(crate) fn bring_up_to_date(&mut self) -> Vec<Diagnostic> {
        match &mut self.target {
            CallTarget::Served => {
                self.state.ensure_fresh();
                self.state.diagnostics()
            }
            CallTarget::Other(other) => {
                other.recompile();
                other.project.diagnostics()
            }
            CallTarget::New(dir) => {
                if self.state.project_root().is_none() {
                    let dir = dir.clone();
                    self.state.serve(&dir);
                }
                Vec::new()
            }
            CallTarget::Unscoped | CallTarget::NoProject(_) => Vec::new(),
        }
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
    // The target's own arguments, read by their type as every argument
    // is (ADR 0033 D4): a `path` that is not a string and a `use_cached`
    // that is not a boolean are refused.
    let path = match spec.reach {
        Reach::Unscoped => None,
        _ => argument::<String>(arguments, "path")?,
    };
    let path = path.as_deref();
    let cached =
        spec.takes_use_cached() && argument::<bool>(arguments, "use_cached")?.unwrap_or(false);
    let served = |state: &mut McpState| {
        if state.project_root().is_none() {
            return CallTarget::NoProject(spec.reach);
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
                return Err(TargetError::PathRequired);
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

/// The target argument `name` of the call, read as `T`: none when absent
/// or `null`.
fn argument<T: crate::args::Arg>(
    arguments: &Value,
    name: &'static str,
) -> Result<Option<T>, TargetError> {
    match arguments.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            T::read(name, value)
                .map(Some)
                .map_err(|message| TargetError::InvalidArgument {
                    argument: name,
                    message,
                })
        }
    }
}

/// The project `path` names: canonical, then the nearest enclosing project,
/// else the directory itself.
fn project_at(path: &Path) -> Result<PathBuf, TargetError> {
    let canonical =
        std::fs::canonicalize(path).map_err(|_| TargetError::PathNotFound(path.to_path_buf()))?;
    Ok(project_root_of(&canonical))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[specforge_test_macros::test(
        behavior = "list_mcp_tools",
        verify = "a tool's path and use_cached are declared once, by its target"
    )]
    fn target_arguments_follow_reach_and_freshness() {
        let names =
            |spec: TargetSpec| -> Vec<String> { spec.properties().keys().cloned().collect() };
        for reach in [
            Reach::Unscoped,
            Reach::Served,
            Reach::AnyProject,
            Reach::WritesAnyProject,
            Reach::NewProject,
        ] {
            for freshness in [Freshness::Fresh, Freshness::FreshUnlessCached] {
                let spec = TargetSpec::new(reach, freshness);
                let takes_path = matches!(
                    reach,
                    Reach::AnyProject | Reach::WritesAnyProject | Reach::NewProject
                );
                let takes_cached = freshness == Freshness::FreshUnlessCached;
                let mut declared = names(spec);
                declared.sort();
                let mut fields: Vec<&str> = spec.fields().to_vec();
                fields.sort();
                assert_eq!(declared, fields, "{reach:?} {freshness:?}");
                assert_eq!(declared.iter().any(|n| n == "path"), takes_path);
                assert_eq!(declared.iter().any(|n| n == "use_cached"), takes_cached);
                assert_eq!(
                    spec.required(),
                    if reach == Reach::NewProject {
                        &["path"][..]
                    } else {
                        &[][..]
                    }
                );
            }
        }
        // `path` reads differently where the call creates its project.
        let described = |reach| {
            TargetSpec::new(reach, Freshness::Fresh).properties()["path"]["description"]
                .as_str()
                .map(str::to_string)
        };
        assert_eq!(
            described(Reach::NewProject).as_deref(),
            Some("Directory for the new project, outside the current one")
        );
        assert_eq!(
            described(Reach::AnyProject).as_deref(),
            Some("Project root path (uses initialized root if omitted)")
        );
    }

    #[specforge_test_macros::test(
        invariant = "mcp_structured_error_responses",
        verify = "a no-project refusal names path only for an entry that takes one"
    )]
    fn no_project_names_path_only_where_the_entry_takes_one() {
        for reach in [
            Reach::AnyProject,
            Reach::WritesAnyProject,
            Reach::NewProject,
        ] {
            let error = no_project(reach);
            assert_eq!(error.code, ErrorCode::PreconditionFailed);
            assert_eq!(error.argument.as_deref(), Some("path"), "{reach:?}");
            assert!(error.message.contains("pass {\"path\": ...}"), "{reach:?}");
            // The client can fix it: -32602 where there is no `isError`.
            assert_eq!(error.into_rpc_error().code, -32602);
        }
        for reach in [Reach::Served, Reach::Unscoped] {
            let error = no_project(reach);
            assert_eq!(error.code, ErrorCode::PreconditionFailed);
            assert_eq!(error.argument, None, "{reach:?}");
            assert!(!error.message.contains("pass {"), "{}", error.message);
            assert!(error.message.contains("specforge mcp <root>"), "{reach:?}");
            // No params fix it: a server-side -32603.
            assert_eq!(error.into_rpc_error().code, -32603);
        }
        // A read that named something refuses the same way, its hint the
        // entry's own.
        let target = CallTarget::NoProject(Reach::Served);
        let refused = without_project(
            &target,
            crate::tool::entity_not_found(&specforge_graph::Graph::new(), "alpha"),
        );
        assert_eq!(refused.code, ErrorCode::PreconditionFailed);
        assert_eq!(refused.argument, None);
        assert_eq!(refused.entity_id.as_deref(), Some("alpha"));
        assert!(!refused.message.contains("pass {"), "{}", refused.message);
        let target = CallTarget::NoProject(Reach::AnyProject);
        let refused = without_project(
            &target,
            crate::tool::entity_not_found(&specforge_graph::Graph::new(), "alpha"),
        );
        assert_eq!(refused.argument.as_deref(), Some("path"));
    }
    #[test]
    fn without_project_names_the_file_from_the_refusal() {
        let target = CallTarget::NoProject(Reach::Served);

        // The file is what the refusal says it is about, not what its
        // message happens to start with.
        let mut refusal = crate::tool::file_not_found("spec/a.spec");
        refusal.message = "the project holds no such file".to_string();
        let refused = without_project(&target, refusal);
        assert_eq!(refused.code, ErrorCode::PreconditionFailed);
        assert!(
            refused
                .message
                .contains("'spec/a.spec' is no project's file"),
            "{}",
            refused.message
        );

        // A refusal that names no file keeps its own message.
        let unnamed = McpError::new(ErrorCode::FileNotFound, "gone for another reason");
        let refused = without_project(&target, unnamed);
        assert!(
            refused.message.contains("gone for another reason"),
            "{}",
            refused.message
        );
    }
}
