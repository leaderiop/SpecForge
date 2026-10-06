//! The project view: what every operation over a compiled project reads
//! (CONTEXT.md "Project view", ADR 0015).
//!
//! A surface builds one from the project it holds, however it holds it (a
//! [`CompiledProject`] in the CLI, a [`ProjectSession`] in MCP and the LSP)
//! and hands it to an operation; the operation returns a typed outcome the
//! surface only renders. The view borrows the environment the project was
//! compiled in and says what its surface reports for the project. It owns
//! the project's recorded test report and the coverage computed from it,
//! both read at the root the project was compiled from and never in an
//! ancestor directory.

use std::path::Path;
use std::sync::Arc;

use specforge_common::Diagnostic;
use specforge_emitter::{GraphProtocolSchema, generate_schema};
use specforge_graph::Graph;
use specforge_project::coverage::{
    ProjectCoverage, Recorded, RecordedCoverage, ReportError, TestReport,
};
use specforge_project::snapshot::EntitySnapshot;
use specforge_project::{CompiledProject, Environment, ProjectSession};
use specforge_registry::RegistryBuild;

use crate::OpError;
use crate::schema_cache::SchemaCache;

/// The compiled project as one surface sees it, borrowed: what every
/// operation over a project reads (CONTEXT.md "Project view", ADR 0015).
///
/// `root` is the root the project was compiled from: the recorded test
/// report, the schema cache, `specforge.lock`, the installed binaries and
/// the source files are read there, never in an ancestor. Without a root
/// (a graph built in memory) there is no report, no schema cache, no
/// extension pass, and an operation that reads or writes the project on
/// disk refuses with `no_project` ([`Self::project_root`]).
#[derive(Clone, Copy)]
pub struct ProjectView<'a> {
    pub graph: &'a Graph,
    /// What `specforge.json` and the loaded extensions gave the compile:
    /// the config, what each `extensions` entry enabled, the spec root,
    /// the registry build. Operations read the config here, never from
    /// disk again.
    pub env: &'a Environment,
    /// `&env.registries`: kinds, fields, edges, rules, the extension
    /// declarations and their ordered passes.
    pub registries: &'a RegistryBuild,
    pub root: Option<&'a Path>,
    /// The memo of the graph's entity snapshot, the recorded report and the
    /// coverage, owned by whoever owns `graph`.
    recorded: &'a RecordedCoverage,
    /// Where what the surface reports for the project comes from.
    reported: Reported<'a>,
    /// What the surface reports after what the compile reported (MCP's
    /// I017 notices).
    also_reported: &'a [Diagnostic],
}

/// Where a view's reported diagnostics come from: its owner, asked when
/// an operation needs them.
#[derive(Clone, Copy)]
enum Reported<'a> {
    /// A one-shot compile: what `specforge check` reports for it.
    Compiled(&'a CompiledProject),
    /// A project session: what a fresh compile reports.
    Session(&'a ProjectSession),
    /// A listed slice: a graph built in memory, a test.
    Listed(&'a [Diagnostic]),
}

impl<'a> ProjectView<'a> {
    /// The view of `graph`, compiled in `env`, rooted at `root`. `recorded`
    /// must belong to the owner of `graph` (`RecordedCoverage::default()`
    /// for a graph assembled in a test). It reports nothing until
    /// [`Self::reporting`].
    pub fn new(
        graph: &'a Graph,
        env: &'a Environment,
        root: Option<&'a Path>,
        recorded: &'a RecordedCoverage,
    ) -> Self {
        ProjectView {
            graph,
            env,
            registries: &env.registries,
            root,
            recorded,
            reported: Reported::Listed(&[]),
            also_reported: &[],
        }
    }

    /// The view of a compiled project, rooted where it was compiled; it
    /// reports what `specforge check` reports for it.
    pub fn of(project: &'a CompiledProject) -> Self {
        ProjectView {
            reported: Reported::Compiled(project),
            ..Self::new(
                &project.graph,
                &project.env,
                Some(&project.env.root),
                project.recorded(),
            )
        }
    }

    /// The view of a session's graph and environment, rooted at `root`
    /// (MCP: its call target's; the LSP: the session's); it reports the
    /// session's diagnostics.
    pub fn of_session(session: &'a ProjectSession, root: Option<&'a Path>) -> Self {
        ProjectView {
            reported: Reported::Session(session),
            ..Self::new(
                session.graph(),
                session.environment(),
                root,
                session.recorded(),
            )
        }
    }

    /// This view, reporting `diagnostics` in place of its owner's (a graph
    /// built in memory, a test).
    pub fn reporting(self, diagnostics: &'a [Diagnostic]) -> Self {
        ProjectView {
            reported: Reported::Listed(diagnostics),
            ..self
        }
    }

    /// This view, reporting `extra` after what its compile reported: MCP's
    /// surface registration notices (I017) for the served project.
    pub fn also_reporting(self, extra: &'a [Diagnostic]) -> Self {
        ProjectView {
            also_reported: extra,
            ..self
        }
    }

    /// What this surface reports for the project: what `specforge check`
    /// reports for the compile behind the view, in its order, then what
    /// the surface added ([`Self::also_reporting`]).
    pub fn reported(&self) -> Vec<Diagnostic> {
        let mut diagnostics = match self.reported {
            Reported::Compiled(project) => project.diagnostics(),
            Reported::Session(session) => session.diagnostics(),
            Reported::Listed(listed) => listed.to_vec(),
        };
        diagnostics.extend(self.also_reported.iter().cloned());
        diagnostics
    }

    /// The root, for an operation that reads or writes the project on
    /// disk: `no_project` without one.
    pub fn project_root(&self) -> Result<&'a Path, OpError> {
        self.root.ok_or_else(|| {
            OpError::new(
                "no_project",
                "this operation needs the project on disk, and this project has none",
            )
        })
    }

    /// `<root>/specforge-report.json`, what `specforge collect` last wrote:
    /// `Ok(None)` without a root or a file; an error (E045) when it is there
    /// but unusable.
    pub fn test_report(&self) -> Result<Option<Arc<TestReport>>, ReportError> {
        self.recorded.report(self.root)
    }

    /// The coverage rule over the graph's entity snapshot and the recorded
    /// report, computed once per compile and report content.
    pub fn coverage(&self) -> Result<Arc<ProjectCoverage>, ReportError> {
        self.recorded().map(|recorded| recorded.coverage)
    }

    /// The recorded report ([`Self::test_report`]) and the coverage computed
    /// from it ([`Self::coverage`]), read together: one read of the report
    /// file for a view that needs both.
    pub fn recorded(&self) -> Result<Recorded, ReportError> {
        // The snapshot first, so an unseeded memo takes it with the
        // environment's spec root.
        self.entities();
        self.recorded.at(self.root, self.graph, self.registries)
    }

    /// The graph's entity snapshot (ADR 0019): every entity with what it
    /// writes and its standing, the one the project's checks read. A view
    /// whose owner seeded none (a graph assembled in a test, the LSP's
    /// stand-in) takes one on first use, with the environment's spec root.
    pub fn entities(&self) -> &'a EntitySnapshot {
        self.recorded
            .entities(self.graph, self.registries, &self.env.spec_root)
    }

    /// The Graph Protocol schema the loaded extensions produce, unversioned
    /// (what the model diagram renders).
    pub fn schema(&self) -> GraphProtocolSchema {
        let registries = self.registries;
        generate_schema(
            &registries.kinds,
            &registries.edges,
            &registries.fields,
            &registries
                .extension_info()
                .map(|(name, version)| (name.to_string(), version.to_string()))
                .collect::<Vec<_>>(),
        )
    }

    /// [`Self::schema`], versioned against the schema cache at the root
    /// (`specforge export`'s rule); without a root, unversioned. Only reads
    /// the cache.
    pub fn versioned_schema(&self) -> GraphProtocolSchema {
        let mut schema = self.schema();
        if let Some(cache) = self.schema_cache() {
            cache.version(&mut schema);
        }
        schema
    }

    /// The schema cache at `<root>/.specforge/`; `None` without a root.
    pub fn schema_cache(&self) -> Option<SchemaCache> {
        self.root.map(SchemaCache::of_root)
    }
}

/// Projects assembled in a test, for the operations' unit tests.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use tempfile::TempDir;

    /// A project assembled in a test: a temp root, a graph, an environment
    /// and what its surface reports. Only what a test sets is on disk.
    pub(crate) struct Fixture {
        pub dir: TempDir,
        pub graph: Graph,
        pub env: Environment,
        pub recorded: RecordedCoverage,
        pub reported: Vec<Diagnostic>,
    }

    impl Fixture {
        /// An empty project rooted at a fresh temp directory: the compile
        /// found a `specforge.json` there (not written to disk) with the
        /// default config, nothing enabled, no extension, nothing reported.
        pub fn new() -> Self {
            let dir = TempDir::new().unwrap();
            let env = Environment {
                root: dir.path().to_path_buf(),
                spec_root: dir.path().to_path_buf(),
                config_found: true,
                ..Environment::empty()
            };
            Fixture {
                dir,
                graph: Graph::new(),
                env,
                recorded: RecordedCoverage::default(),
                reported: Vec::new(),
            }
        }

        /// The compile read `config` as `specforge.json` (not written to
        /// disk): its `extensions` entries, each enabling what its text
        /// names ([`EnabledExtension::of`] with no runtime).
        pub fn config_json(mut self, config: serde_json::Value) -> Self {
            let entries: Vec<String> = config["extensions"]
                .as_array()
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|e| e.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            self.env.enabled = entries
                .iter()
                .map(|e| specforge_project::EnabledExtension::of(e, None))
                .collect();
            self.env.config.extensions = entries;
            self.env.config.raw = Some(config);
            self
        }

        /// [`Self::config_json`] of a config enabling `entries`.
        pub fn config(self, entries: &[&str]) -> Self {
            self.config_json(serde_json::json!({ "extensions": entries }))
        }

        /// What the compile read each entry as enabling, in place of what
        /// the config's entries name.
        pub fn enabled(mut self, enabled: Vec<specforge_project::EnabledExtension>) -> Self {
            self.env.enabled = enabled;
            self
        }

        /// The compile loaded `declarations`, in this order.
        pub fn declarations(
            mut self,
            declarations: Vec<specforge_protocol_types::ExtensionDeclaration>,
        ) -> Self {
            self.env.registries = specforge_registry::build_registries(declarations);
            self
        }

        /// An extension `name` at `version` that contributes nothing.
        pub fn declaration(
            name: &str,
            version: &str,
        ) -> specforge_protocol_types::ExtensionDeclaration {
            specforge_protocol_types::ExtensionDeclaration {
                handshake: specforge_protocol_types::HandshakeResponse {
                    name: name.into(),
                    version: version.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }

        /// `<dir>/specforge.lock` locks each `(name, version, source)`.
        pub fn lock(self, entries: &[(&str, &str, &str)]) -> Self {
            let lock = specforge_wasm::LockFile {
                lockfile_version: 1,
                entries: entries
                    .iter()
                    .map(|(name, version, source)| specforge_wasm::LockFileEntry {
                        name: name.to_string(),
                        version: version.to_string(),
                        source: source.to_string(),
                        wasm_hash: format!("hash-{name}"),
                        key_id: None,
                        peer_dependencies: Vec::new(),
                    })
                    .collect(),
            };
            specforge_wasm::write_lock_file(&lock, &self.dir.path().join("specforge.lock"))
                .unwrap();
            self
        }

        /// The compile found no `specforge.json` at the root.
        pub fn without_config_file(mut self) -> Self {
            self.env.config_found = false;
            self
        }

        /// The compile could not use `specforge.json` as written, for
        /// `problems` (what the surface reports for them is the test's to
        /// set).
        pub fn config_problems(mut self, problems: Vec<specforge_common::ConfigProblem>) -> Self {
            self.env.config_problems = problems;
            self
        }

        /// The project's surface reports `diagnostics`.
        pub fn reporting(mut self, diagnostics: Vec<Diagnostic>) -> Self {
            self.reported = diagnostics;
            self
        }

        /// The view rooted at the temp directory, reporting what the
        /// fixture reports.
        pub fn view(&self) -> ProjectView<'_> {
            ProjectView::new(
                &self.graph,
                &self.env,
                Some(self.dir.path()),
                &self.recorded,
            )
            .reporting(&self.reported)
        }

        /// The same project without a root.
        pub fn rootless_view(&self) -> ProjectView<'_> {
            ProjectView::new(&self.graph, &self.env, None, &self.recorded).reporting(&self.reported)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;

    #[specforge_test(
        behavior = "snapshot_entities_once",
        verify = "the checks, the check passes and the coverage of one compile read one snapshot"
    )]
    fn a_view_reads_the_snapshot_its_compile_took() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("specforge.json"), r#"{"name": "v"}"#).unwrap();
        std::fs::write(dir.path().join("a.spec"), "behavior a \"A\" {\n}\n").unwrap();
        let compiled = CompiledProject::compile(dir.path(), None);
        let view = ProjectView::of(&compiled);
        assert!(std::ptr::eq(view.entities(), compiled.entities()));
        assert!(std::ptr::eq(
            view.coverage().unwrap().entities(),
            compiled.entities()
        ));
        assert_eq!(view.entities().kind_of("a"), Some("behavior"));

        // A graph assembled without a compile: the view takes one, once,
        // with the environment's spec root.
        let graph = Graph::new();
        let mut env = Environment::with_registries(RegistryBuild::default());
        env.spec_root = dir.path().join("spec");
        let recorded = RecordedCoverage::default();
        let view = ProjectView::new(&graph, &env, None, &recorded);
        assert!(view.entities().is_empty());
        assert!(std::ptr::eq(view.entities(), view.entities()));
        assert_eq!(view.entities().spec_root(), env.spec_root);
        assert!(std::ptr::eq(
            view.coverage().unwrap().entities(),
            view.entities()
        ));
    }

    #[specforge_test(
        behavior = "read_views_over_the_project_view",
        verify = "the recorded test report is read at the view's root, never an ancestor's"
    )]
    fn the_report_is_read_at_the_view_root_not_an_ancestor() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path();
        std::fs::write(project.join("specforge.json"), "{}").unwrap();
        std::fs::write(project.join("specforge-report.json"), "{not json").unwrap();
        let sub = project.join("sub");
        std::fs::create_dir(&sub).unwrap();
        let graph = Graph::new();
        let env = Environment::with_registries(RegistryBuild::default());

        let recorded = RecordedCoverage::default();
        let at_sub = ProjectView::new(&graph, &env, Some(&sub), &recorded);
        assert!(at_sub.test_report().unwrap().is_none());
        assert!(at_sub.coverage().unwrap().summary.test_results.is_none());

        let recorded = RecordedCoverage::default();
        let at_root = ProjectView::new(&graph, &env, Some(project), &recorded);
        let error = at_root.test_report().unwrap_err();
        assert_eq!(error.diagnostic().code, "E045");
        assert!(at_root.coverage().is_err());

        let recorded = RecordedCoverage::default();
        let rootless = ProjectView::new(&graph, &env, None, &recorded);
        assert!(rootless.test_report().unwrap().is_none());
    }

    fn codes(diagnostics: &[Diagnostic]) -> Vec<&str> {
        diagnostics.iter().map(|d| d.code.as_str()).collect()
    }

    #[specforge_test(
        behavior = "read_views_over_the_project_view",
        verify = "a view reports what its compile reported, then what its surface adds"
    )]
    fn a_view_reports_what_its_compile_reported_then_what_its_surface_adds() {
        // A compile reports what `specforge check` reports for it.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("specforge.json"), r#"{"extensions": []}"#).unwrap();
        std::fs::write(dir.path().join("main.spec"), "behavior b \"B\" {\n}\n").unwrap();
        let compiled = CompiledProject::compile(dir.path(), None);
        let of = ProjectView::of(&compiled);
        assert_eq!(of.reported(), compiled.diagnostics());
        assert_eq!(of.root, Some(dir.path()));
        assert!(std::ptr::eq(of.registries, &of.env.registries));

        // A view built in memory reports nothing until it is told what.
        let graph = Graph::new();
        let env = Environment::with_registries(RegistryBuild::default());
        let recorded = RecordedCoverage::default();
        let bare = ProjectView::new(&graph, &env, None, &recorded);
        assert!(bare.reported().is_empty());
        let warning = [Diagnostic::warning("W002", "unused")];
        let listed = bare.reporting(&warning);
        assert_eq!(codes(&listed.reported()), ["W002"]);

        // What the surface adds comes after, whatever the view reports.
        let i017 = [Diagnostic::info("I017", "not auto-promoted")];
        assert_eq!(
            codes(&listed.also_reporting(&i017).reported()),
            ["W002", "I017"]
        );
        let mut expected = compiled.diagnostics();
        expected.extend(i017.iter().cloned());
        assert_eq!(of.also_reporting(&i017).reported(), expected);
    }

    #[specforge_test(
        behavior = "management_operations_over_the_project_view",
        verify = "an operation that reads or writes the project on disk refuses a view without a root"
    )]
    fn a_view_without_a_root_has_no_project_root() {
        let fixture = testing::Fixture::new();

        let error = fixture.rootless_view().project_root().unwrap_err();

        assert_eq!(error.code, "no_project");
        assert_eq!(
            fixture.view().project_root().unwrap(),
            fixture.dir.path(),
            "a rooted view's project root is its root"
        );
    }
}
