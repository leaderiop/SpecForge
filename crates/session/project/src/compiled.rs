//! The compiled project (CONTEXT.md "Compiled project", ADR 0047): what a
//! compile built and what it reports. A one-shot compile is one; a project
//! session holds one and keeps it current.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use specforge_common::{Diagnostic, ProjectRoot, codes};
use specforge_graph::{
    Applied, Computed, FileChange, Graph, GraphBuild, compute_graph_delta, moved_nodes,
};

mod disk;

use crate::coverage::{REPORT_FILE, Recorded, RecordedCoverage, ReportError, TestReport};
use crate::ledger::{Reads, Unrecorded};
use crate::sources::{Read, SourceCache};
use crate::{Environment, SharedRuntime, SourceBuild};
use specforge_registry::{Disk, EntitySnapshot, PlainDisk};

/// A compiled project: an environment, the sources read in it, their graph
/// build, what resolving their imports and running the checks reported, and
/// the memo of the graph's entity snapshot and coverage (ADR 0019).
///
/// [`Self::diagnostics`] is, by definition, what `specforge check` reports,
/// in the one report order. A one-shot compile is one ([`Self::compile`]:
/// the CLI); a [`crate::ProjectSession`] holds one and keeps it current
/// ([`crate::ProjectSession::project`]). After every update that runs the
/// checks, a session's compiled project reports exactly what
/// [`Self::compile`] of the same sources reports, in the same order (ADR
/// 0047). An editor update that skipped the checks
/// ([`crate::CheckMode::SyntaxOnlyIfParseErrors`]) reports no check
/// diagnostic until the next update that runs them.
///
/// A copy of this project as it is now: the graph is copied, everything else
/// is shared. What a reader holds while the project's session is out for an
/// update (the LSP, ADR 0100 D4, ADR 0108). A copy is never the session's:
/// updating it is the session's alone.
#[derive(Clone)]
pub struct CompiledProject {
    /// Shared, so a reader keeps the environment an update started from
    /// while the session is out for it (the LSP's copy).
    env: Arc<Environment>,
    /// The text of every source as read, the retained trees and the
    /// sources that could not be read.
    sources: SourceCache,
    /// The graph of the sources and what building it reported (ADR 0032).
    graph: GraphBuild,
    /// E025 for the unreadable sources, then the resolver's (E025, I004,
    /// W113, W027).
    import_diagnostics: Vec<Diagnostic>,
    /// What the checks on the graph reported: the registry build's checks
    /// and the extensions' rules, then the check-phase passes. Empty when
    /// the last update skipped them.
    check_diagnostics: Vec<Diagnostic>,
    /// The values the compute passes gave at the last check (ADR 0287); empty when the checks were skipped.
    computed: Arc<Computed>,
    /// The graph's entity snapshot, the recorded test report and the
    /// coverage: seeded with the snapshot the checks read; made over the
    /// graph on first use after an update that skipped them.
    recorded: OnceLock<RecordedCoverage>,
    /// What it reports ([`Self::diagnostics`]), in the one report order,
    /// made on first use after a change and shared: a session's update holds
    /// it as what the session reports now, and the next update as what it
    /// reported before (ADR 0074).
    reported: OnceLock<Arc<[Diagnostic]>>,
    /// The one answer to rooted or detached ([`Self::root`], ADR 0047 D4, ADR 0121).
    /// `false` for a detached project (no project on disk: an editor with
    /// no workspace folder, MCP while nothing is served): it has no root,
    /// and imports are not resolved, with no spec root to resolve them
    /// against (ADR 0030 D5).
    on_disk: bool,
    /// The last update skipped the checks (the typing fast path): none is
    /// reported until an update runs them (ADR 0136 D4).
    checks_skipped: bool,
    /// What reading the baseline gave at the last check (ADR 0287): held, so a typed edit or an update asks git
    /// nothing again. A new commit drops it.
    baseline: Option<Arc<crate::baseline::HeldBaseline>>,
    /// Renames applied to the editor's buffers and not yet saved (ADR 0289): the checks read the ledger as they will
    /// have written it. Empty but for a session of an editor.
    ledger_overlay: Vec<Arc<crate::harness::LedgerOverlay>>,
}

impl CompiledProject {
    /// Compile the project at `root`, its extensions loaded into `runtime`
    /// (without one no extension loads), which its environment holds, its
    /// sources read from the plain file system. It keeps no stamp (ADR 0030).
    pub fn compile(root: &ProjectRoot, runtime: Option<SharedRuntime>) -> Self {
        Self::compile_through(Environment::load(root, runtime), &PlainDisk)
    }

    /// The project of `env`, each source read through `sources` (ADR 0166): a
    /// surface that bounds its request hands its containment's disk, and a
    /// source the disk refuses is that source's E025, absent from the graph
    /// (ADR 0145 D2). The environment and the checks read the plain file
    /// system (ADR 0145 D3), as a session's do. A caller that read the
    /// environment first (a migration plans with its field forms, ADR 0186)
    /// compiles in that load.
    pub fn compile_through(env: Environment, sources: &dyn Disk) -> Self {
        Self::checked(env, &mut Unrecorded::through(sources))
    }

    /// Compile the project of an environment already loaded (the CLI routes
    /// an extension command on it first): its sources read from the plain
    /// file system, its graph built and checked in the environment's runtime.
    pub fn of(env: Environment) -> Self {
        Self::checked(env, &mut Unrecorded::plain())
    }

    /// The cold read of `env` through `reads`, then every check.
    fn checked(env: Environment, reads: &mut Unrecorded<'_>) -> Self {
        let mut project = Self::read(Arc::new(env), &BTreeMap::new(), reads);
        project.check(reads);
        project
    }

    /// The environment it was compiled in: config, what each `extensions`
    /// entry enabled, the spec root, the registry build.
    pub fn environment(&self) -> &Environment {
        &self.env
    }

    /// [`Self::environment`], shared: it stays valid after a session
    /// replaces this project. Tests build an unread copy from it.
    #[cfg(feature = "testing")]
    pub fn shared_environment(&self) -> Arc<Environment> {
        Arc::clone(&self.env)
    }

    /// This project with `texts` as the text each source was read as, and no other: the same graph,
    /// environment, report and memo (shared, as `Clone` shares them). A test's way to give navigation
    /// a text that disagrees with the graph (shifted) or none (left out); production never makes one.
    #[cfg(feature = "testing")]
    pub fn with_source_texts<K: Into<String>, T: Into<Arc<str>>>(
        &self,
        texts: impl IntoIterator<Item = (K, T)>,
    ) -> Self {
        CompiledProject {
            sources: SourceCache::holding(
                texts
                    .into_iter()
                    .map(|(key, text)| (key.into(), text.into()))
                    .collect(),
            ),
            ..self.clone()
        }
    }

    /// This project as if `texts` (by source key, relative to the project root) were the text of their sources:
    /// a key no compiled source has when the environment's [`SourceRoot::holds_key`] admits it, then
    /// every check. What `specforge check` reports once those texts are written. Reads no source: every
    /// other source is what this project holds (the text it read, or the E025 it recorded for one it could
    /// not read: over MCP, one the bound refused, ADR 0158); the checks read the disk as a compile's do.
    /// Compare it with [`Self::checked_diagnostics`], not [`Self::diagnostics`]: after an update that skipped the
    /// checks, only the former has them.
    pub fn edited(&self, texts: &BTreeMap<String, String>) -> CompiledProject {
        self.edited_with(texts, None)
    }

    /// [`Self::edited`] with the harness ledger (ADR 0287) as `ledger`, the bytes a plan will write: the checks read
    /// them for `specforge-harness`, so a rename's re-keyed lines are seen when the plan is judged.
    pub fn edited_with(
        &self,
        texts: &BTreeMap<String, String>,
        ledger: Option<&[u8]>,
    ) -> CompiledProject {
        let mut keys: BTreeSet<String> =
            self.sources.keys().into_iter().map(String::from).collect();
        let mut planned: BTreeMap<String, &str> = BTreeMap::new();
        for (key, text) in texts {
            if keys.contains(key) || self.env.sources().holds_key(key) {
                keys.insert(key.clone());
                planned.insert(key.clone(), text.as_str());
            }
        }
        let keys: Vec<String> = keys.into_iter().collect();
        let mut project = Self::built(
            Arc::clone(&self.env),
            &keys,
            &planned,
            self.on_disk,
            |key| self.sources.as_read(key),
        );
        project.baseline = self.baseline.clone();
        match ledger {
            Some(bytes) => {
                let overlay = crate::harness::Overlay::new(self.env.root(), bytes);
                project.check(&mut Unrecorded::checking(&overlay));
            }
            None => project.check(&mut Unrecorded::plain()),
        }
        project
    }

    /// The bytes of the file `name` at the project root on disk now (`specforge-harness`, `.gitattributes`); empty
    /// when there is none.
    pub fn root_file(&self, name: &str) -> Vec<u8> {
        use specforge_registry::Disk;
        specforge_registry::PlainDisk
            .read(&self.env.root().join(name))
            .unwrap_or_default()
    }

    /// Whether an extension the project enables declares a harness field (ADR 0287).
    pub fn declares_harness(&self) -> bool {
        !crate::harness::Harness::of(self.env.registries()).is_empty()
    }

    /// Whether a write of this project is serialized by the write lock (ADR 0289): the project declares a field
    /// whose ledger a write keeps true, or an extension that writes through a plan (a writing command or
    /// collector) or judges a write (a guard pass). Plain projects write without taking it.
    pub fn serializes_writes(&self) -> bool {
        self.declares_harness()
            || self.env.registries().declarations().iter().any(|d| {
                d.surfaces.commands.iter().any(|c| c.writes)
                    || d.collectors.iter().any(|c| c.writes)
                    || d.passes
                        .iter()
                        .any(|p| p.phase.as_deref() == Some(specforge_protocol_types::GUARD_PHASE))
            })
    }

    /// The harness ledger after a write that took this project to `after`, from the ledger `current` (the file's
    /// bytes, empty when it has none): [`Ledger::rewritten`](crate::harness::Ledger::rewritten) over the two graphs.
    /// With `keep_unseen`, a line of `current` for an entity
    /// neither project holds is kept (it is another writer's).
    pub fn rewritten_ledger<'a>(
        &'a self,
        after: &'a CompiledProject,
        current: &[u8],
        change: &crate::harness::LedgerChange,
        keep_unseen: bool,
    ) -> Vec<u8> {
        use crate::harness::{Harness, Ledger, View};
        let harness = Harness::of(self.env.registries());
        let forms = self.env.registries().field_forms();
        let view = |project: &'a CompiledProject, keeps_gone| View {
            graph: project.graph(),
            forms,
            harness: &harness,
            keeps_gone,
        };
        let errors = after.parsed().any(|(_, file)| !file.errors.is_empty());
        let ledger = Ledger::parse(&String::from_utf8_lossy(current)).ledger;
        let mut rewritten = ledger.rewritten(&view(self, false), &view(after, errors), change);
        if keep_unseen {
            // The ledger was rewritten by another writer since this change was planned: its lines for entities, or
            // for values, neither project holds are that writer's, and stay.
            let held = |project: &CompiledProject, l: &crate::harness::Line| {
                if l.is_field() {
                    harness
                        .value(project.graph(), forms, &l.path, &l.key)
                        .is_some()
                } else {
                    project.graph().node(&l.path).is_some()
                }
            };
            for line in ledger.lines().filter(|l| !held(self, l) && !held(after, l)) {
                rewritten.insert(line.clone());
            }
        }
        rewritten.canonical().into_bytes()
    }

    /// This project judged against the merge-base of `revision` and `HEAD` instead of `HEAD` (`check --since`):
    /// the checks run again over a copy; this project keeps its own baseline.
    pub fn judged_since(
        &self,
        revision: &str,
    ) -> Result<CompiledProject, crate::baseline::UnknownRevision> {
        let read = crate::baseline::Baseline::since(&self.env, revision)?;
        let mut project = self.clone();
        project.baseline = Some(Arc::new(crate::baseline::HeldBaseline {
            stamp: None,
            pinned: true,
            read,
        }));
        project.check(&mut Unrecorded::plain());
        Ok(project)
    }

    /// What `specforge check` reports for this project now: [`Self::diagnostics`], or, when the last
    /// update skipped the checks (the editor's typing fast path, ADR 0136 D4), that report with every
    /// check run now over a copy; this project keeps skipping them. What [`Self::edited`]'s report is
    /// compared with (ADR 0151 D3, ADR 0167). Reads no source.
    pub fn checked_diagnostics(&self) -> Vec<Diagnostic> {
        if !self.checks_skipped {
            return self.diagnostics();
        }
        // shortcut: not memoized, so each edit planned while the checks are skipped runs them once more;
        // memoize beside `reported` if an eager client's code actions make that measurable.
        let mut checked = self.clone();
        checked.check(&mut Unrecorded::plain());
        checked.diagnostics()
    }

    /// The root it was compiled from; `None` for a detached project.
    pub fn root(&self) -> Option<&Path> {
        self.on_disk.then_some(self.env.root())
    }

    /// The keys of the sources it holds (each `.spec` file's path from
    /// the project root): those in its graph and those that could not be read.
    /// What a `use` path may name (ADR 0055).
    pub fn source_keys(&self) -> BTreeSet<&str> {
        self.sources.keys()
    }

    /// The graph of its sources.
    pub fn graph(&self) -> &Graph {
        self.graph.graph()
    }

    /// Exactly what `specforge check` reports, in its order: the
    /// environment's (its own, then the registry build's), the imports',
    /// the graph build's, then the checks'. The one place this order is
    /// written. A diagnostic about entities with no span of its own is
    /// located at the first one's block (ADR 0227).
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.reported().to_vec()
    }

    /// [`Self::diagnostics`], shared: made once per state of the project.
    /// The one place the report order is written. The environment's
    /// diagnostics never change inside a compiled project (a reload builds
    /// a new one), so the memo resets where the other parts change.
    pub(crate) fn reported(&self) -> Arc<[Diagnostic]> {
        Arc::clone(self.reported.get_or_init(|| {
            let graph = self.graph();
            self.env
                .diagnostics()
                .chain(&self.import_diagnostics)
                .chain(self.graph.diagnostics())
                .chain(&self.check_diagnostics)
                .cloned()
                // Settings, once, for every source: a gated code is dropped, a raised one is raised.
                .filter_map(|d| self.env.settings().effective(d))
                .map(|d| d.placed_at_subject(|id| graph.node(id).map(|n| n.source_span.clone())))
                .collect()
        }))
    }

    /// The text the graph's spans for `path` (relative to the project root)
    /// are positions in: the file as read, or the buffer as given, at the
    /// last change of that file. `None` for a file it does not hold.
    pub fn source_text(&self, path: &str) -> Option<Arc<str>> {
        self.sources.text(path).cloned()
    }

    /// [`Self::source_text`] of every file, shared rather than copied.
    pub fn source_texts(&self) -> HashMap<String, Arc<str>> {
        self.sources.texts()
    }

    /// How many `.spec` files it holds.
    pub fn file_count(&self) -> usize {
        self.graph.files().len()
    }

    /// Every source it parsed, by key, in key order: the parses its graph was built from (the finder reads them).
    pub fn parsed(&self) -> impl ExactSizeIterator<Item = (&str, &specforge_graph::SpecFile)> {
        self.graph.files()
    }

    /// The graph's entity snapshot (ADR 0019): the one its checks read, or
    /// one taken on first use after an update that skipped them.
    pub fn entities(&self) -> &EntitySnapshot {
        self.memo().entities()
    }

    /// The values the compute passes gave at the last check (ADR 0287), beside the graph: empty when no pass gave
    /// one or the checks were skipped.
    pub fn computed(&self) -> &Computed {
        &self.computed
    }

    /// [`Self::entities`], shared: a test holds it across an update.
    #[cfg(feature = "testing")]
    pub fn shared_entities(&self) -> Arc<EntitySnapshot> {
        Arc::clone(self.memo().entities())
    }

    /// The recorded test report, `<root>/specforge-report.json` ([`REPORT_FILE`]), read through
    /// `disk` (ADR 0139): `Ok(None)` for a detached project (the disk is not asked) or when the
    /// disk has no such file; an error when it is there but unusable, never memoized. One
    /// `disk.read` per call; parsed again only when its bytes changed.
    pub fn test_report(&self, disk: &dyn Disk) -> Result<Option<Arc<TestReport>>, ReportError> {
        self.memo().report(self.report_file().as_deref(), disk)
    }

    /// [`Self::test_report`] and the coverage of this project's entity snapshot against it,
    /// computed once per report content: what every coverage view reads.
    pub fn recorded(&self, disk: &dyn Disk) -> Result<Recorded, ReportError> {
        self.memo().at(self.report_file().as_deref(), disk)
    }

    /// Where its report is: none for a detached project.
    fn report_file(&self) -> Option<PathBuf> {
        self.root().map(|root| root.join(REPORT_FILE))
    }

    /// The memo of the entity snapshot, the report and the coverage, for this state of the
    /// project.
    fn memo(&self) -> &RecordedCoverage {
        self.recorded
            .get_or_init(|| RecordedCoverage::over(&self.graph, &self.env))
    }
}

/// What the session asks of its compiled project, between stamps.
impl CompiledProject {
    /// No project: the empty environment, no source, no file, nothing
    /// reported. Verifies its updates in a debug build.
    pub(crate) fn detached() -> Self {
        let env = Arc::new(Environment::empty());
        let mut graph = env.graph_of([]);
        graph.set_verify(cfg!(debug_assertions));
        CompiledProject {
            env,
            sources: SourceCache::empty(),
            graph,
            import_diagnostics: Vec::new(),
            check_diagnostics: Vec::new(),
            computed: Arc::default(),
            recorded: OnceLock::new(),
            reported: OnceLock::new(),
            on_disk: false,
            checks_skipped: false,
            ledger_overlay: Vec::new(),
            baseline: None,
        }
    }

    /// The project of an environment whose sources are not read yet: no
    /// source, an empty graph, and it reports what its environment reports.
    /// Rooted where the environment was loaded from disk (an environment
    /// assembled in memory makes a detached one).
    pub fn unread(env: Arc<Environment>) -> Self {
        let graph = env.graph_of([]);
        let on_disk = env.project_root().is_some();
        CompiledProject {
            env,
            sources: SourceCache::empty(),
            graph,
            import_diagnostics: Vec::new(),
            check_diagnostics: Vec::new(),
            computed: Arc::default(),
            recorded: OnceLock::new(),
            reported: OnceLock::new(),
            on_disk,
            checks_skipped: false,
            ledger_overlay: Vec::new(),
            baseline: None,
        }
    }

    /// A project a test assembled in memory: `graph` (made elsewhere), the
    /// environment it was assembled in and `reported`, exactly (no check
    /// runs). `rooted`: it is rooted where the environment is; otherwise it
    /// has no root (ADR 0047 D4). Its memo is made over its own graph.
    #[cfg(feature = "testing")]
    pub(crate) fn assembled(
        env: Arc<Environment>,
        graph: GraphBuild,
        reported: Vec<Diagnostic>,
        sources: SourceCache,
        rooted: bool,
    ) -> Self {
        CompiledProject {
            env,
            sources,
            graph,
            import_diagnostics: Vec::new(),
            check_diagnostics: Vec::new(),
            computed: Arc::default(),
            recorded: OnceLock::new(),
            reported: OnceLock::from(Arc::<[Diagnostic]>::from(reported)),
            on_disk: rooted,
            checks_skipped: false,
            ledger_overlay: Vec::new(),
            baseline: None,
        }
    }

    /// The cold read in `env` (ADR 0032's one cold pipeline,
    /// [`Environment::build_sources`]): discover through `reads`, read each
    /// source through it (a `held` text, by source key, in place of its
    /// file), build the graph, resolve the imports. The checks have not run.
    pub(crate) fn read(
        env: Arc<Environment>,
        held: &BTreeMap<String, &str>,
        reads: &mut impl Reads,
    ) -> Self {
        let keys = reads.discover(env.sources());
        let rooted = env.project_root().is_some();
        let sources = env.sources().clone();
        Self::built(env, &keys, held, rooted, |key| reads.source(&sources, key))
    }

    /// The cold build of `keys` in `env` (ADR 0032's one cold pipeline, [`Environment::build_sources`]): each
    /// key a `held` text, else what `source` gives for it; the graph built, the imports resolved. The checks
    /// have not run. `on_disk` is the project's answer to rooted ([`Self::root`]): the environment's for a
    /// project read from disk, the edited project's own for its copy.
    fn built(
        env: Arc<Environment>,
        keys: &[String],
        held: &BTreeMap<String, &str>,
        on_disk: bool,
        source: impl FnMut(&str) -> Read,
    ) -> Self {
        let SourceBuild {
            sources,
            graph,
            imports,
        } = env.build_sources(keys, held, source);
        CompiledProject {
            env,
            sources,
            graph,
            import_diagnostics: imports,
            check_diagnostics: Vec::new(),
            computed: Arc::default(),
            recorded: OnceLock::new(),
            reported: OnceLock::new(),
            on_disk,
            checks_skipped: false,
            ledger_overlay: Vec::new(),
            baseline: None,
        }
    }

    /// Whether the project already holds the state `read` gives for `key`.
    pub(crate) fn is_current(&self, key: &str, read: &Read) -> bool {
        self.sources.is_current(key, read)
    }

    /// Apply each source's new state as one change of the graph build;
    /// the memo starts again and the imports are resolved again (none
    /// when detached). The checks have not run.
    pub(crate) fn apply(&mut self, reads: Vec<(String, Read)>) -> Applied {
        let changes: Vec<FileChange> = reads
            .into_iter()
            .map(|(path, read)| self.sources.change(&path, read))
            .collect();
        let applied = self.graph.apply(changes);
        self.recorded = OnceLock::new();
        self.reported = OnceLock::new();
        self.import_diagnostics = if self.on_disk {
            self.sources
                .import_diagnostics(&self.graph, self.env.sources().prefix())
        } else {
            Vec::new()
        };
        applied
    }

    /// Every check on the current graph, over a snapshot of it taken now
    /// (ADR 0019), in the environment's runtime, reading the file system
    /// through the disk `reads` gives them (ADR 0107); the memo starts again
    /// from that snapshot.
    pub(crate) fn check(&mut self, reads: &mut impl Reads) {
        let env = Arc::clone(&self.env);
        let this = &*self;
        let graph = &this.graph;
        let mut outcome = None;
        let held = self.baseline.clone();
        let mut baseline = None;
        reads.run_checks(|disk| {
            // The files `HEAD` is named by are read first, so each is an input of this run: a commit changes one.
            let stamp = crate::baseline::Baseline::ref_stamp(&env, disk);
            let read = match held {
                Some(held) if held.pinned || held.stamp == stamp => held,
                _ => Arc::new(crate::baseline::HeldBaseline {
                    read: crate::baseline::Baseline::of_head(&env),
                    stamp,
                    pinned: false,
                }),
            };
            let authored = env.entity_snapshot(graph);
            let mut ledger = Vec::new();
            let overlaid;
            let disk: &dyn specforge_registry::Disk = if this.ledger_overlay.is_empty() {
                disk
            } else {
                overlaid =
                    crate::harness::overlay::through(disk, this, &this.ledger_overlay, &mut ledger);
                &overlaid
            };
            let checked = env.checked(Some(graph), &authored, disk, &read.read);
            baseline = Some(read);
            let entities = Arc::new(checked.entities.unwrap_or(authored));
            let mut diagnostics = checked.diagnostics;
            // A write that was interrupted (ADR 0289): the next writer rolls it back, a check only says so.
            diagnostics.extend(crate::journal::pending(env.root()));
            outcome = Some((diagnostics, entities, checked.computed));
        });
        self.checks_skipped = false;
        self.baseline = baseline;
        if let Some((diagnostics, entities, computed)) = outcome {
            self.check_diagnostics = diagnostics;
            self.computed = Arc::new(computed);
            self.recorded = OnceLock::from(RecordedCoverage::of(entities));
            self.reported = OnceLock::new();
        }
    }

    /// The renames the editor applied to its buffers and has not saved (ADR 0289), in the order they were made.
    /// Changing them owes the checks: the next update runs them. `true` when they changed.
    pub(crate) fn overlay_ledger(
        &mut self,
        overlay: Vec<Arc<crate::harness::LedgerOverlay>>,
    ) -> bool {
        let same = self.ledger_overlay.len() == overlay.len()
            && self
                .ledger_overlay
                .iter()
                .zip(&overlay)
                .all(|(a, b)| Arc::ptr_eq(a, b));
        if !same {
            self.ledger_overlay = overlay;
            self.checks_skipped = true;
        }
        !same
    }

    /// The checks are skipped for this state: none is reported, and the
    /// memo is made over the graph on first use.
    pub(crate) fn skip_checks(&mut self) {
        self.check_diagnostics = Vec::new();
        self.computed = Arc::default();
        self.recorded = OnceLock::new();
        self.reported = OnceLock::new();
        self.checks_skipped = true;
    }

    /// Whether the last update skipped the checks: the next update, or a
    /// release, runs them even when it changes no file.
    pub(crate) fn checks_skipped(&self) -> bool {
        self.checks_skipped
    }

    /// Whether any of `paths` has a parse error (E001) in the graph build.
    pub(crate) fn has_parse_errors_in(&self, paths: &[String]) -> bool {
        paths.iter().any(|path| {
            self.graph
                .file_diagnostics(path)
                .iter()
                .any(|d| d.is(codes::E001))
        })
    }

    /// What replacing `previous` with this project amounts to, reported as
    /// an apply: every file rebuilt, the delta between the two graphs;
    /// never verified.
    pub(crate) fn replacing(&self, previous: &CompiledProject) -> Applied {
        Applied {
            files: self
                .graph
                .files()
                .map(|(path, _)| path.to_string())
                .collect(),
            delta: compute_graph_delta(previous.graph(), self.graph()),
            moved: moved_nodes(previous.graph(), self.graph()),
            verification: None,
        }
    }

    /// Compare every apply with a cold build (ADR 0032, 0035).
    pub(crate) fn set_verify(&mut self, enabled: bool) {
        self.graph.set_verify(enabled);
    }

    pub(crate) fn verifies(&self) -> bool {
        self.graph.verifies()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_registry::{Asked, MemoryDisk, PlainDisk};
    use specforge_test_macros::test as specforge_test;

    const MISSING_IMPORT: &str = "use \"missing\"\nbehavior a \"A\" {\n}\n";

    #[test]
    fn a_detached_project_has_no_root_and_resolves_no_import() {
        let mut detached = CompiledProject::detached();
        assert_eq!(detached.root(), None);
        detached.apply(vec![(
            "a.spec".into(),
            Read::Text(MISSING_IMPORT.to_string()),
        )]);
        let imports = |p: &CompiledProject| -> Vec<Diagnostic> {
            p.diagnostics()
                .into_iter()
                .filter(|d| d.code == "W113" || d.code == "I004" || d.code == "E025")
                .collect()
        };
        assert!(
            imports(&detached).is_empty(),
            "{:?}",
            detached.diagnostics()
        );

        // The same text in a project on disk resolves its imports.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("specforge.json"), r#"{"extensions": []}"#).unwrap();
        std::fs::write(dir.path().join("a.spec"), MISSING_IMPORT).unwrap();
        let compiled = CompiledProject::compile(
            &specforge_common::ProjectRoot::of(dir.path()).unwrap(),
            None,
        );
        assert!(
            !imports(&compiled).is_empty(),
            "{:?}",
            compiled.diagnostics()
        );
    }

    #[test]
    fn a_detached_project_asks_no_disk_for_its_report() {
        let disk = MemoryDisk::new().with_file("/specforge-report.json", b"{");
        assert!(
            CompiledProject::detached()
                .recorded(&disk)
                .unwrap()
                .report
                .is_none()
        );
        assert!(disk.asked().is_empty());
    }

    #[test]
    fn a_rooted_project_reads_its_report_through_the_disk_it_is_given() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("specforge.json"), r#"{"extensions": []}"#).unwrap();
        let project = CompiledProject::compile(
            &specforge_common::ProjectRoot::of(dir.path()).unwrap(),
            None,
        );
        let file = project.root().unwrap().join("specforge-report.json");
        let disk = MemoryDisk::new().with_file(
            &file,
            br#"{"results":{"a":{"tests":[{"name":"t","status":"passed"}]}}}"#,
        );
        assert!(project.test_report(&disk).unwrap().is_some());
        assert_eq!(disk.asked(), [Asked::Read(file)]);
        // Nothing on the file system: the memo re-keys on content.
        assert!(project.test_report(&PlainDisk).unwrap().is_none());
    }

    #[specforge_test(
        behavior = "snapshot_entities_once",
        verify = "the checks, the check passes and the coverage of one compile read one snapshot"
    )]
    fn skipping_the_checks_reports_none_and_scores_the_graph_on_first_use() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("specforge.json"),
            r#"{"name":"s","version":"0.1.0","extensions":["@specforge/software"]}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("a.spec"), "behavior alpha \"A\" {\n}\n").unwrap();
        let runtime = specforge_component::ComponentRuntime::with_user_cache();
        let mut project = CompiledProject::compile(
            &specforge_common::ProjectRoot::of(dir.path()).unwrap(),
            Some(Arc::new(runtime)),
        );
        let checked = project.diagnostics();
        assert!(checked.iter().any(|d| d.code == "W006"), "{checked:?}");
        assert!(!project.checks_skipped());

        project.skip_checks();
        assert!(project.checks_skipped());
        assert!(
            project.diagnostics().iter().all(|d| d.code != "W006"),
            "{:?}",
            project.diagnostics()
        );
        assert_eq!(project.entities().kind_of("alpha"), Some("behavior"));
    }
}
