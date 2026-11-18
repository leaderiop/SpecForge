//! An environment's check of a built graph's entity snapshot: the compute passes first, then the registry build's
//! checks and rules and the check passes over the snapshot with their values merged, and the harness ledger.

use specforge_common::Diagnostic;
use specforge_diagnostics::{CodeRef, Codebook};
use specforge_graph::{Computed, GraphBuild};
use specforge_registry::rules::{CustomVerdicts, NoVerdicts};
use specforge_registry::{Disk, EntitySnapshot};

use crate::delta::Delta;
use crate::verdicts::WasmVerdicts;
use crate::{
    CheckedValues, Environment, baseline, check_passes, compute, coverage, harness, history, judged,
};

/// What [`Environment::checked`] found: the diagnostics, and the compute passes' values with the snapshot that has
/// them merged in (`None` when none gave a value).
pub struct Checked {
    pub diagnostics: Vec<Diagnostic>,
    pub entities: Option<EntitySnapshot>,
    pub computed: Computed,
    /// Each judged rule's state on each entity (ADR 0290).
    pub judgements: judged::Judgements,
    /// What changed since the run's base, when a loaded pass declares the `delta` input (ADR 0376 D6).
    pub delta: Option<Delta>,
}

impl Environment {
    /// Every check a compile runs over the graph `build`, on its entity snapshot taken now: the registry build's
    /// checks (the structural checks and the extensions' rules, in the order [`RegistryBuild::check`] runs them),
    /// then the check-phase passes; the custom verdicts and the passes run in [`Self::runtime`]. Every file a check
    /// reads goes through `disk` (ADR 0107).
    pub fn run_checks(&self, build: &GraphBuild, disk: &dyn Disk) -> Vec<Diagnostic> {
        self.checked(
            build,
            &self.entity_snapshot(build),
            disk,
            &baseline::Baseline::of_head(self, &baseline::Checking::of(self)),
            &judged::DigestMemo::default(),
            &CheckedValues::default(),
        )
        .diagnostics
    }

    /// The delta of `build` from the empty graph, its base none (ADR 0376 D7: with no base every node is added), when
    /// a loaded pass declares the `delta` input; `None` otherwise, so it is never taken for nothing.
    pub(crate) fn delta_of(&self, build: &GraphBuild) -> Option<Delta> {
        (self.registries.passes_reading_delta() > 0).then(|| {
            Delta::between(
                &specforge_graph::Graph::default(),
                build.graph(),
                self.registries.field_forms(),
                specforge_protocol_types::DeltaBase::None,
            )
        })
    }

    /// [`Self::run_checks`] over the authored snapshot `entities`, with what the compute passes gave (ADR 0287):
    /// they run first, the registry's checks and rules and the check passes read the snapshot with their values
    /// merged. `computed` is empty and `entities` is `None` (the snapshot given is the one read) when no compute
    /// pass gave a value. `build` is the graph the snapshot was taken from: the harness ledger is checked against
    /// it ([`harness::check`]). The delta of `build` is taken once ([`Self::delta_of`]): the compute and check passes
    /// that declare the `delta` input are sent its part of their kinds (ADR 0376 D6), and it is returned.
    pub fn checked(
        &self,
        build: &GraphBuild,
        entities: &EntitySnapshot,
        disk: &dyn Disk,
        baseline: &baseline::BaselineRead,
        digests: &judged::DigestMemo,
        checked: &CheckedValues,
    ) -> Checked {
        let runtime = self.runtime.as_deref();
        let delta = self.delta_of(build);
        let mut diagnostics = Vec::new();
        let mut computed = Computed::default();
        let mut merged = None;
        // The baseline with the ledger's moves applied: what the passes get as `previous`, and what history is
        // judged against.
        let harness = harness::Harness::of(&self.registries);
        // The ledger is read once, and only by a run that has a use for it: history, a harness value or a verdict.
        let ledger_bytes = (matches!(baseline, Ok(Some(_))) || !harness.is_empty()).then(|| {
            disk.read(&self.root().join(harness::LEDGER_FILE))
                .unwrap_or_default()
        });
        let translated = match (baseline, &ledger_bytes) {
            (Ok(Some(baseline)), Some(bytes)) => {
                let ledger = harness::Ledger::parse(&String::from_utf8_lossy(bytes)).ledger;
                Some(baseline.translated(&ledger, build.graph(), self.registries.field_forms()))
            }
            _ => None,
        };
        // The recorded report, once, for the passes that declare the `tests` input (protocol 1.11.0).
        let reading = self.registries.passes_reading_tests();
        let report = (reading > 0 && runtime.is_some())
            .then(|| coverage::read_report(self.root(), entities, disk));
        let tests = match &report {
            Some(coverage::ReportRead::Read(results)) => Some(results),
            _ => None,
        };
        let mut found = judged::Evaluated::default();
        if let Some(bytes) = &ledger_bytes {
            found = judged::evaluate(self, build.graph(), entities, disk, digests, checked, bytes);
        }
        // The `V` fields the fresh verdicts give are values before any compute pass runs (ADR 0290).
        let mut values = std::mem::take(&mut found.values);
        if let Some(runtime) = runtime
            && self.registries.compute_passes().next().is_some()
        {
            let (computed_values, reported) = compute::run(
                self,
                entities,
                runtime,
                translated.as_ref(),
                &values,
                &found.judgements,
                compute::Reads {
                    disk,
                    tests,
                    delta: delta.as_ref(),
                },
            );
            values = computed_values;
            diagnostics.extend(reported);
        }
        if !values.is_empty() {
            merged = Some(entities.with_computed(&values, self.registries.fields()));
            computed = values;
        }
        // The judgements owed once each owner state is known on the merged snapshot (ADR 0362); what the judged
        // rules say comes first, as it did before the compute passes ran.
        let (judgements, said) = found.settle(self, merged.as_ref().unwrap_or(entities));
        diagnostics.splice(0..0, said);
        {
            let entities = merged.as_ref().unwrap_or(entities);
            let verdicts: Box<dyn CustomVerdicts + '_> = match runtime {
                Some(runtime) => Box::new(WasmVerdicts::new(runtime, entities)),
                None => Box::new(NoVerdicts),
            };
            diagnostics.extend(
                self.registries
                    .check(&entities.rule_input(), verdicts.as_ref()),
            );
            if let Some(bytes) = &ledger_bytes {
                diagnostics.extend(harness::check(
                    &harness,
                    build.graph(),
                    self.registries.field_forms(),
                    bytes,
                    build.files().any(|(_, file)| !file.errors.is_empty()),
                ));
            }
            if let Some(runtime) = runtime {
                diagnostics.extend(check_passes::run(
                    self,
                    entities,
                    runtime,
                    translated.as_ref(),
                    &judgements,
                    compute::Reads {
                        disk,
                        tests,
                        delta: delta.as_ref(),
                    },
                ));
            }
        }
        // I205 or E045 once, unless a judged rule already said it.
        if let Some(unread) = report.as_ref().and_then(|r| r.unread(reading))
            && !diagnostics.iter().any(|d| d.code == unread.code)
        {
            diagnostics.push(unread);
        }
        match (baseline, &translated) {
            (Err(notice), _) => diagnostics.push(notice.clone()),
            (_, Some(translated)) => {
                diagnostics.extend(history::check::check(
                    translated,
                    build.graph(),
                    self.registries.field_forms(),
                    &self.registries,
                    &build
                        .files()
                        .filter(|(_, file)| !file.errors.is_empty())
                        .map(|(key, _)| key)
                        .collect::<Vec<_>>(),
                ));
            }
            _ => {}
        }
        // A finding whose code waits for an owner state speaks only on an entity in that state, read where a
        // computed status decides it (ADR 0363).
        let owners = merged.as_ref().unwrap_or(entities);
        diagnostics.retain(|d| speaks(self.registries.codes(), owners, d));
        Checked {
            diagnostics,
            entities: merged,
            computed,
            judgements,
            delta,
        }
    }
}

/// Whether `d` speaks on its entity as `entities` holds it (`Codebook::speaks`, ADR 0363): a host finding, one that
/// names no entity, one whose entity `entities` does not hold, and one whose code waits for no owner state always do;
/// another only in the owner state its code waits for.
pub(crate) fn speaks(codes: &Codebook, entities: &EntitySnapshot, d: &Diagnostic) -> bool {
    let (Some(extension), Some(subject)) = (d.origin(), d.subject()) else {
        return true;
    };
    let local = CodeRef::parse(&d.code).local;
    if !codes.waits(extension, local) {
        return true;
    }
    entities
        .get(subject)
        .is_none_or(|(record, _)| codes.speaks(extension, local, entities.is_pending(record)))
}
