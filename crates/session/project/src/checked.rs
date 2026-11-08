//! An environment's check of a built graph's entity snapshot: the compute passes first, then the registry build's
//! checks and rules and the check passes over the snapshot with their values merged, and the harness ledger.

use specforge_common::Diagnostic;
use specforge_graph::{Computed, GraphBuild};
use specforge_registry::rules::{CustomVerdicts, NoVerdicts};
use specforge_registry::{Disk, EntitySnapshot};

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
}

impl Environment {
    /// Every check a compile runs over the entity snapshot `entities` of a
    /// built graph: the registry build's checks (the structural
    /// checks and the extensions' rules, in the order
    /// [`RegistryBuild::check`] runs them), then the check-phase passes;
    /// the custom verdicts and the passes run in [`Self::runtime`]. Every file
    /// a check reads goes through `disk` (ADR 0107).
    pub fn run_checks(&self, entities: &EntitySnapshot, disk: &dyn Disk) -> Vec<Diagnostic> {
        self.checked(
            None,
            entities,
            disk,
            &baseline::Baseline::of_head(self),
            &judged::DigestMemo::default(),
            &CheckedValues::default(),
        )
        .diagnostics
    }

    /// [`Self::run_checks`] over the authored snapshot `entities`, with what the compute passes gave (ADR 0287):
    /// they run first, the registry's checks and rules and the check passes read the snapshot with their values
    /// merged. `computed` is empty and `entities` is `None` (the snapshot given is the one read) when no compute
    /// pass gave a value. With the graph `build` the snapshot was taken from, the harness ledger is checked too
    /// ([`harness::check`]).
    pub fn checked(
        &self,
        build: Option<&GraphBuild>,
        entities: &EntitySnapshot,
        disk: &dyn Disk,
        baseline: &baseline::BaselineRead,
        digests: &judged::DigestMemo,
        checked: &CheckedValues,
    ) -> Checked {
        let runtime = self.runtime.as_deref();
        let mut diagnostics = Vec::new();
        let mut computed = Computed::default();
        let mut merged = None;
        // The baseline with the ledger's moves applied: what the passes get as `previous`, and what history is
        // judged against.
        let harness = harness::Harness::of(&self.registries);
        // The ledger is read once, and only by a run that has a use for it: history, a harness value or a verdict.
        let ledger_bytes = build
            .filter(|_| matches!(baseline, Ok(Some(_))) || !harness.is_empty())
            .map(|_| {
                disk.read(&self.root().join(harness::LEDGER_FILE))
                    .unwrap_or_default()
            });
        let translated = match (baseline, build, &ledger_bytes) {
            (Ok(Some(baseline)), Some(build), Some(bytes)) => {
                let ledger = harness::Ledger::parse(&String::from_utf8_lossy(bytes)).ledger;
                Some(baseline.translated(&ledger, build.graph(), self.registries.field_forms()))
            }
            (Ok(Some(baseline)), None, _) => Some((**baseline).clone()),
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
        if let (Some(build), Some(bytes)) = (build, &ledger_bytes) {
            found = judged::evaluate(self, build.graph(), entities, disk, digests, checked, bytes);
            diagnostics.append(&mut found.diagnostics);
        }
        // The `V` fields the fresh verdicts give are values before any compute pass runs (ADR 0290).
        let mut values = std::mem::take(&mut found.values);
        let judgements = found.judgements;
        if let Some(runtime) = runtime
            && self.registries.compute_passes().next().is_some()
        {
            let (computed_values, reported) = compute::run(
                self,
                entities,
                runtime,
                translated.as_ref(),
                &values,
                &judgements,
                compute::Reads { disk, tests },
            );
            values = computed_values;
            diagnostics.extend(reported);
        }
        if !values.is_empty() {
            merged = Some(entities.with_computed(&values, self.registries.fields()));
            computed = values;
        }
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
            if let (Some(build), Some(bytes)) = (build, &ledger_bytes) {
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
                    disk,
                    tests,
                ));
            }
        }
        // I205 or E045 once, unless a judged rule already said it.
        if let Some(unread) = report.as_ref().and_then(|r| r.unread(reading))
            && !diagnostics.iter().any(|d| d.code == unread.code)
        {
            diagnostics.push(unread);
        }
        match (baseline, &translated, build) {
            (Err(notice), ..) => diagnostics.push(notice.clone()),
            (_, Some(translated), Some(build)) => {
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
        Checked {
            diagnostics,
            entities: merged,
            computed,
            judgements,
        }
    }
}
