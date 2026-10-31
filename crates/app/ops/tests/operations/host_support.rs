//! What the K9 tests of hooks, state, the spec guard and doctor share (ADR 0292): the GSD-shaped host fixture
//! ([`specforge_project::testing::GsdHosts`]) served in process, and the one place a test calls the host.
//!
//! [`run_hook`], [`run_core_hook`], [`state_of`] and [`host_findings`] are the one place a test calls the host.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_ops::authoring::hook;
use specforge_ops::host::HostHookEvent;
use specforge_ops::view::ProjectView;
use specforge_project::testing::{
    CORE, GSD_HOSTS_EXTENSIONS, GsdHosts, TestProject, UI, gsd_hosts_sources,
};
use specforge_project::{CompiledProject, SharedRuntime};
use specforge_wasm::testing::InProcessRuntime;

/// A project of the fixture's extensions with `core` settings for `@t/core` and `ui` for `@t/ui`.
pub fn hosts_project(core: Value, ui: Value) -> TestProject {
    let project = TestProject::new()
        .config(|c| {
            c["name"] = "p".into();
            c["settings"] = json!({ CORE: core, UI: ui });
        })
        .enabling(&GSD_HOSTS_EXTENSIONS)
        .installing_enabled();
    for (path, text) in gsd_hosts_sources() {
        project.write(path, text);
    }
    project
}

/// A project served in process and compiled.
pub struct Hosted {
    pub project: TestProject,
    pub runtime: Arc<InProcessRuntime>,
    pub compiled: CompiledProject,
}

pub fn hosted(project: TestProject, fixture: GsdHosts) -> Hosted {
    let runtime = Arc::new(fixture.runtime());
    let compiled = project.compile_in(Arc::clone(&runtime) as SharedRuntime);
    Hosted {
        project,
        runtime,
        compiled,
    }
}

impl Hosted {
    /// The exports called so far, in order, as `extension export`.
    pub fn exports(&self) -> Vec<String> {
        self.runtime
            .calls()
            .into_iter()
            .map(|c| format!("{} {}", c.extension, c.export))
            .collect()
    }

    /// The input the last call of `export` was sent.
    pub fn input(&self, export: &str) -> Value {
        self.runtime
            .calls()
            .into_iter()
            .rev()
            .find(|call| call.export == export)
            .unwrap_or_else(|| panic!("{export} was not called"))
            .input
    }
}

/// What running a hook came to: the neutral answer (`decision`, `reason`, `context`, `notice`, `statusline`), what
/// went to stderr, the exports the guest was asked for, and the files the run changed.
#[derive(Debug)]
pub struct HookRun {
    pub answer: Value,
    pub stderr: String,
    pub exports: Vec<String>,
    pub changed: Vec<String>,
}

/// The neutral event of a Write of `content` to `path` by the host agent `session`.
pub fn write_event(host: &str, path: &str, content: &str) -> Value {
    json!({
        "event": "pre_tool", "host": host, "session": "s1",
        "tool": {"name": "Write", "class": "write", "input": {"file_path": path, "content": content}},
    })
}

/// Run the hook `id` of `@t/core` (`<short>.<id>`) over `hosted` for the neutral `event`.
pub fn run_hook(hosted: &Hosted, id: &str, event: &Value) -> HookRun {
    let before = hosted.exports().len();
    let view = ProjectView::of(&hosted.compiled);
    let neutral: HostHookEvent = serde_json::from_value(event.clone()).expect("a neutral event");
    let outcome = hook::run(
        &view,
        &if id.starts_with("core.") {
            id.to_string()
        } else {
            format!("t.{id}")
        },
        &neutral,
        specforge_ops::testing::caller(),
    );
    HookRun {
        answer: serde_json::to_value(&outcome.answer).expect("an answer serializes"),
        stderr: outcome.stderr,
        exports: hosted.exports().split_off(before),
        changed: outcome.changed,
    }
}

/// The state document of `extension` (`project` or `sessions/<id>`), if it exists.
pub fn state_of(hosted: &Hosted, extension: &str, document: &str) -> Option<Value> {
    let root = ProjectView::of(&hosted.compiled).root()?.to_path_buf();
    let text =
        std::fs::read_to_string(root.join(specforge_ops::host::state::path(extension, document)))
            .ok()?;
    serde_json::from_str(&text).ok()
}

/// The doctor findings about hosts, as `code path`.
pub fn host_findings(hosted: &Hosted) -> Vec<String> {
    let view = ProjectView::of(&hosted.compiled);
    let adapters: Vec<_> = specforge_ops::host::Host::ALL
        .iter()
        .filter_map(|host| specforge_hosts::renderer(*host))
        .collect();
    let renderers: Vec<&dyn specforge_ops::host::HostRenderer> =
        adapters.iter().map(AsRef::as_ref).collect();
    specforge_ops::host::diagnose::findings(&view, &renderers, specforge_ops::testing::caller())
        .into_iter()
        .map(|f| match f.path {
            Some(path) => format!("{} {path}", f.code),
            None => f.code.to_string(),
        })
        .collect()
}

/// Run the core hook `core.<id>` (the spec guard), which no extension declares.
pub fn run_core_hook(hosted: &Hosted, id: &str, event: &Value) -> HookRun {
    let before = hosted.exports().len();
    let view = ProjectView::of(&hosted.compiled);
    let neutral: HostHookEvent = serde_json::from_value(event.clone()).expect("a neutral event");
    let outcome = hook::run(
        &view,
        &format!("core.{id}"),
        &neutral,
        specforge_ops::testing::caller(),
    );
    HookRun {
        answer: serde_json::to_value(&outcome.answer).expect("an answer serializes"),
        stderr: outcome.stderr,
        exports: hosted.exports().split_off(before),
        changed: outcome.changed,
    }
}
