//! What a check's delta is taken against (ADR 0376 D7): nothing (the empty graph, so a cold run sees every node
//! added), the project at the merge-base of a revision and `HEAD` (`check --since`, MCP `validate since`), or the
//! graph of a session's last completed check (LSP, MCP, watch). A check since a revision is judged and given its
//! delta in one run; a session holds its last checked graph only while a loaded pass reads the delta.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, LazyLock};

use specforge_effects::git::Sha;
use specforge_graph::Graph;
use specforge_protocol_types::DeltaBase;

use super::CompiledProject;
use crate::Environment;
use crate::baseline::{Baseline, Checking, HeldBaseline, UnknownRevision, declared};
use crate::ledger::Unrecorded;
use crate::snapshot::{Since, Wants, load_at, served_by, since};

/// The graph a check's delta is taken from, and the base the payload names.
#[derive(Debug, Clone, Default)]
pub(crate) enum DeltaBasis {
    /// No base: the delta is taken from the empty graph.
    #[default]
    None,
    /// The project at the merge-base `sha`, compiled with the current extensions, and the collectors it serves a
    /// fact store of: a collector the check's graph does not serve too is left out of the delta (ADR 0376 D4).
    Revision {
        sha: Sha,
        graph: Arc<Graph>,
        served: BTreeSet<String>,
    },
    /// The graph of the session's last completed check.
    Previous(Arc<Graph>),
}

/// The graph of no base.
static EMPTY: LazyLock<Graph> = LazyLock::new(Graph::default);

impl DeltaBasis {
    /// The graph the delta is taken from: the empty one for no base.
    pub(crate) fn graph(&self) -> &Graph {
        match self {
            DeltaBasis::None => &EMPTY,
            DeltaBasis::Revision { graph, .. } | DeltaBasis::Previous(graph) => graph,
        }
    }

    /// The base the delta payload names: the merge-base by its full sha.
    pub(crate) fn label(&self) -> DeltaBase {
        match self {
            DeltaBasis::None => DeltaBase::None,
            DeltaBasis::Revision { sha, .. } => DeltaBase::Revision {
                id: sha.as_str().to_string(),
            },
            DeltaBasis::Previous(_) => DeltaBase::Previous,
        }
    }
}

impl CompiledProject {
    /// The base its next check takes the delta against, as the payload names it.
    pub fn delta_base(&self) -> DeltaBase {
        self.delta_base.label()
    }

    /// The project of `env` checked once since `revision` (`check --since`): the revision and its merge-base with
    /// `HEAD` are resolved, the baseline read there when history is declared and the project at the merge-base built
    /// when a loaded pass reads the delta, before any source is checked; so every pass runs once, judged against the
    /// merge-base and sent what changed since it. `UnknownRevision` before anything is checked.
    pub fn of_since(env: Environment, revision: &str) -> Result<CompiledProject, UnknownRevision> {
        let mut reads = Unrecorded::plain();
        let mut project = Self::read(Arc::new(env), &BTreeMap::new(), &mut reads);
        project.pin_since(revision)?;
        project.check(&mut reads);
        Ok(project)
    }

    /// This project judged against the merge-base of `revision` and `HEAD` instead of `HEAD`, and its delta taken
    /// from there (MCP `validate since`): itself when it was checked since that revision already ([`Self::of_since`]),
    /// else a copy pinned to it and checked once; this project keeps its own baseline and base.
    pub fn judged_since(&self, revision: &str) -> Result<CompiledProject, UnknownRevision> {
        let since_now = self
            .baseline
            .as_ref()
            .and_then(|held| held.since.as_deref());
        if since_now == Some(revision) {
            return Ok(self.clone());
        }
        let mut project = self.clone();
        project.pin_since(revision)?;
        project.check(&mut Unrecorded::plain());
        Ok(project)
    }

    /// Pin the next check to `revision`, resolved once ([`since`]) for both: its baseline the merge-base's read, and
    /// its delta base the project at the merge-base when a loaded pass reads the delta. With no merge-base the delta
    /// has no base (a cold run's), and the baseline read says why when history is declared (I204, W164).
    fn pin_since(&mut self, revision: &str) -> Result<(), UnknownRevision> {
        let env = Arc::clone(&self.env);
        let reads_delta = env.registries().passes_reading_delta() > 0;
        let resolved = since(&env, revision, declared(env.registries()) || reads_delta)?;
        let checking = Checking {
            effects: self.effects(),
            cache: Arc::clone(&self.check_cache),
        };
        let read = Baseline::read_since(&env, revision, &resolved, &checking)?;
        self.baseline = Some(Arc::new(HeldBaseline {
            stamp: None,
            since: Some(revision.to_string()),
            read,
        }));
        self.delta_base = match resolved {
            Since::Base { place, base, .. } if reads_delta => at_merge_base(&env, &place, base),
            _ => DeltaBasis::None,
        };
        Ok(())
    }

    /// Hold this project's graph as the base of its next check's delta (a session, after each check it runs), only
    /// while a loaded pass reads the delta: with none, no graph is held.
    pub(crate) fn hold_previous(&mut self) {
        self.delta_base = if self.env.registries().passes_reading_delta() == 0 {
            DeltaBasis::None
        } else {
            DeltaBasis::Previous(Arc::new(self.graph().clone()))
        };
    }
}

/// The project of `env` at the merge-base `base` of the repository `place`, built with the current extensions and
/// served the fact stores its ledger records (a snapshot's graph): the revision base. One git cannot give is no
/// base: the delta is then taken as on a cold run.
fn at_merge_base(
    env: &Environment,
    place: &specforge_effects::git::GitPlace,
    base: Sha,
) -> DeltaBasis {
    let Some(history) = env.git() else {
        return DeltaBasis::None;
    };
    let wants = Wants {
        tree: false,
        // the stores are read from the plain file system, as the checks read them
        facts: env
            .reads_facts()
            .then_some(&specforge_registry::PlainDisk as &dyn specforge_registry::Disk),
        config: false,
    };
    match load_at(env, &*history, place, &base, wants) {
        Ok(loaded) => DeltaBasis::Revision {
            sha: base,
            served: served_by(&loaded.built.graph),
            graph: Arc::new(loaded.built.graph.into_parts().0),
        },
        Err(_) => DeltaBasis::None,
    }
}
