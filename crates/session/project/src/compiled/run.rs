//! One run of every check on a compiled project's graph.

use std::sync::{Arc, OnceLock};

use specforge_common::Diagnostic;
use specforge_graph::Computed;
use specforge_registry::EntitySnapshot;

use super::{CheckedValues, CompiledProject};
use crate::coverage::RecordedCoverage;
use crate::ledger::Reads;

/// What one run of the checks gave, held until the run's read of the project ends.
struct CheckRun {
    diagnostics: Vec<Diagnostic>,
    entities: Arc<EntitySnapshot>,
    computed: Computed,
    judgements: crate::judged::Judgements,
    value_checks: crate::values::ValueChecks,
    checked_values: Arc<CheckedValues>,
}

impl CompiledProject {
    /// Every check on the current graph, over a snapshot of it taken now (ADR 0019), in the environment's runtime,
    /// reading the file system through the disk `reads` gives them (ADR 0107), each effect record it consults an
    /// input (ADR 0337 D5); the memo starts again from that snapshot.
    pub(crate) fn check(&mut self, reads: &mut impl Reads) {
        let env = Arc::clone(&self.env);
        let this = &*self;
        let graph = &this.graph;
        let mut outcome = None;
        let (held, mut baseline) = (self.baseline.clone(), None);
        reads.run_checks(|disk| {
            this.effects().take_log();
            // The files `HEAD` is named by are read first, so each is an input of this run: a commit changes one.
            let stamp = crate::baseline::Baseline::ref_stamp(&env, disk);
            let read = match held {
                Some(held) if held.reusable(stamp.as_ref(), this.effects().journal()) => held,
                _ => Arc::new(crate::baseline::HeldBaseline {
                    read: crate::baseline::Baseline::of_head(
                        &env,
                        &crate::baseline::Checking {
                            effects: this.effects(),
                            cache: Arc::clone(&this.check_cache),
                        },
                    ),
                    stamp,
                    pinned: false,
                }),
            };
            let mut ledger = Vec::new();
            let overlaid;
            let disk: &dyn specforge_registry::Disk = if this.ledger_overlay.is_empty() {
                disk
            } else {
                overlaid =
                    crate::harness::overlay::through(disk, this, &this.ledger_overlay, &mut ledger);
                &overlaid
            };
            let mut authored = env.entity_snapshot(graph);
            // A fact still naming what a rename moved is told where it went, from the ledger (ADR 0346 D7).
            if let Some(root) = this.root().filter(|_| env.reads_facts()) {
                authored.suggest_moved(&crate::facts::moves(&crate::facts::ledger(disk, root)));
            }
            let text_of = |file: &str| this.sources.text(file).cloned();
            // One budget over both stages of the values phase (ADR 0350).
            let mut stages = crate::values::StageContext {
                effects: this.effects(),
                disk,
                cache: &this.check_cache,
                text_of: &text_of,
                spent: crate::values::Spent::starting(env.value_budget()),
            };
            // Before the rules: a verdict digests what the checks found (ADR 0337 §6).
            let mut values =
                crate::values::run(&env, &authored, crate::values::Stage::Ungated, &mut stages);
            // No checker at all leaves the values a test gave (`with_checked_values`).
            let checked_values = if env.registries().checks().is_empty() {
                Arc::clone(&this.checked_values)
            } else {
                Arc::new(values.checks.outcome_classes())
            };
            let checked = env.checked(
                Some(graph),
                &authored,
                disk,
                &read.read,
                &this.digests,
                &checked_values,
            );
            baseline = Some(read);
            let entities = Arc::new(checked.entities.unwrap_or(authored));
            // A field whose checks wait for its owner: owner state is read on the merged snapshot (ADR 0350).
            let gated =
                crate::values::run(&env, &entities, crate::values::Stage::Gated, &mut stages);
            values.checks.extend(gated.checks);
            let mut diagnostics = values.diagnostics;
            diagnostics.extend(checked.diagnostics);
            diagnostics.extend(gated.diagnostics);
            let stale = this.stale_records(disk, &entities, &checked_values, &checked.computed);
            diagnostics.extend(stale);
            diagnostics.extend(crate::values::approvals(this.effects(), &stages.spent));
            // What this machine did not perform of them: an effect's, once for the run.
            diagnostics.extend(crate::effects::not_run(&this.effects().log()));
            // A write that was interrupted (ADR 0289): the next writer rolls it back, a check only says so.
            // Read through the checks' disk, so a journal that appears or goes is an input; a detached project has none.
            if let Some(root) = this.root() {
                diagnostics.extend(crate::journal::pending(root, disk));
            }
            outcome = Some(CheckRun {
                diagnostics,
                entities,
                computed: checked.computed,
                judgements: checked.judgements,
                value_checks: values.checks,
                checked_values,
            });
            this.consulted(baseline.as_deref())
        });
        self.checks_skipped = false;
        self.baseline = baseline;
        if let Some(run) = outcome {
            self.value_checks = Arc::new(run.value_checks);
            self.checked_values = run.checked_values;
            self.check_diagnostics = run.diagnostics;
            self.computed = Arc::new(run.computed);
            self.judgements = Arc::new(run.judgements);
            self.recorded = OnceLock::from(RecordedCoverage::of(run.entities));
            self.reported = OnceLock::new();
        }
    }
}
