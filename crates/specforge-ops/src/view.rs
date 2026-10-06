//! The project view: what every read operation over a compiled project
//! reads (CONTEXT.md "Project view", ADR 0015).
//!
//! A surface builds one from the project it holds, however it holds it (a
//! [`CompiledProject`] in the CLI, a [`ProjectSession`] in MCP and the LSP)
//! and hands it to an operation; the operation returns a typed outcome the
//! surface only renders. The view owns the project's recorded test report
//! and the coverage computed from it, both read at the root the project was
//! compiled from and never in an ancestor directory.

use std::path::Path;
use std::sync::Arc;

use specforge_emitter::{GraphProtocolSchema, generate_schema};
use specforge_graph::Graph;
use specforge_project::coverage::{ProjectCoverage, RecordedCoverage, ReportError, TestReport};
use specforge_project::snapshot::EntitySnapshot;
use specforge_project::{CompiledProject, ProjectSession};
use specforge_registry::RegistryBuild;

use crate::schema_cache::SchemaCache;

/// The read-only slice of a compiled project every operation reads,
/// borrowed. `root` is the root the project was compiled from: its recorded
/// test report and its schema cache are there, never in an ancestor.
/// Without a root (a graph built in memory) there is no report, no schema
/// cache, and no extension pass runs.
#[derive(Clone, Copy)]
pub struct ProjectView<'a> {
    pub graph: &'a Graph,
    /// Kinds, fields, edges, rules, the extension declarations and their
    /// ordered passes.
    pub registries: &'a RegistryBuild,
    pub root: Option<&'a Path>,
    /// The memo of the graph's entity snapshot, the recorded report and the
    /// coverage, owned by whoever owns `graph`.
    recorded: &'a RecordedCoverage,
}

impl<'a> ProjectView<'a> {
    /// The view of `graph`, built with `registries`, rooted at `root`.
    /// `recorded` must belong to the owner of `graph` (a fresh
    /// `RecordedCoverage::default()` for a graph assembled in a test).
    pub fn new(
        graph: &'a Graph,
        registries: &'a RegistryBuild,
        root: Option<&'a Path>,
        recorded: &'a RecordedCoverage,
    ) -> Self {
        ProjectView {
            graph,
            registries,
            root,
            recorded,
        }
    }

    /// The view of a compiled project, rooted where it was compiled.
    pub fn of(project: &'a CompiledProject) -> Self {
        Self::new(
            &project.graph,
            &project.env.registries,
            Some(&project.env.root),
            project.recorded(),
        )
    }

    /// The view of a session's graph and environment, rooted at `root`:
    /// the caller's (MCP: its call target; the LSP: the session's root).
    pub fn of_session(session: &'a ProjectSession, root: Option<&'a Path>) -> Self {
        Self::new(
            session.graph(),
            &session.environment().registries,
            root,
            session.recorded(),
        )
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
        self.recorded
            .at(self.root, self.graph, self.registries)
            .map(|recorded| recorded.coverage)
    }

    /// The graph's entity snapshot (ADR 0019): every entity with what it
    /// writes and its standing, the one the project's checks read. A view
    /// whose owner seeded none (a graph assembled in a test) takes one on
    /// first use, its spec root the view's root; nothing a view reads
    /// resolves a path.
    pub fn entities(&self) -> &'a EntitySnapshot {
        self.recorded.entities(
            self.graph,
            self.registries,
            self.root.unwrap_or(Path::new("")),
        )
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

        // A graph assembled without a compile: the view takes one, once.
        let (graph, registries) = (Graph::new(), RegistryBuild::default());
        let recorded = RecordedCoverage::default();
        let view = ProjectView::new(&graph, &registries, None, &recorded);
        assert!(view.entities().is_empty());
        assert!(std::ptr::eq(view.entities(), view.entities()));
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
        let (graph, registries) = (Graph::new(), RegistryBuild::default());

        let recorded = RecordedCoverage::default();
        let at_sub = ProjectView::new(&graph, &registries, Some(&sub), &recorded);
        assert!(at_sub.test_report().unwrap().is_none());
        assert!(at_sub.coverage().unwrap().summary.test_results.is_none());

        let recorded = RecordedCoverage::default();
        let at_root = ProjectView::new(&graph, &registries, Some(project), &recorded);
        let error = at_root.test_report().unwrap_err();
        assert_eq!(error.diagnostic().code, "E045");
        assert!(at_root.coverage().is_err());

        let recorded = RecordedCoverage::default();
        let rootless = ProjectView::new(&graph, &registries, None, &recorded);
        assert!(rootless.test_report().unwrap().is_none());
    }
}
