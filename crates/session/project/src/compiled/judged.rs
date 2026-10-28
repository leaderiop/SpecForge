//! A project a write is being judged on (ADR 0289).

use super::CompiledProject;
use std::sync::OnceLock;

impl CompiledProject {
    /// What this project reports, the guard passes included, judged against itself: the report a write is compared
    /// with, so a guard that already objects to the project does not refuse the write that fixes it (ADR 0289). Equal
    /// to [`Self::checked_diagnostics`] when the project declares no guard pass.
    pub fn judged_diagnostics(&self) -> Vec<specforge_common::Diagnostic> {
        if self.env.registries().guard_passes().next().is_none() {
            return self.checked_diagnostics();
        }
        let mut checked = self.clone();
        if self.checks_skipped {
            checked.check(&mut crate::ledger::Unrecorded::plain());
        }
        checked.judged_against(self).diagnostics()
    }

    /// This project, which a write is being judged on, with the guard passes (ADR 0289) run over it: `previous` is
    /// the project before the write. Their findings join its checks'; none runs unless the project declares a guard
    /// pass, and a compile never runs one.
    pub fn judged_against(mut self, previous: &CompiledProject) -> CompiledProject {
        let Some(runtime) = self.env.runtime() else {
            return self;
        };
        if self.env.registries().guard_passes().next().is_none() {
            return self;
        }
        let baseline = crate::baseline::Baseline::of_project(previous);
        let found = crate::guards::run(
            &self.env,
            self.memo().entities(),
            &**runtime as &dyn specforge_wasm::WasmRuntime,
            &baseline,
            &self.judgements,
            &specforge_registry::PlainDisk,
        );
        self.check_diagnostics.extend(found);
        self.reported = OnceLock::new();
        self
    }

    /// What this project reports now (the settings' gating and raises applied), with the findings of the guard passes
    /// that report one of `codes` (qualified, `gsd/E512`) added: a gate asks (ADR 0291). Each such pass runs once over
    /// this project, which is also its `previous` (nothing is written), and its input says `trigger`. A project
    /// declaring no such pass reports what [`Self::checked_diagnostics`] does, and a compile never runs one.
    pub fn gated_against_self(
        &self,
        codes: &[String],
        trigger: &specforge_protocol_types::Trigger,
    ) -> Vec<specforge_common::Diagnostic> {
        let registries = self.env.registries();
        let named: Vec<&specforge_registry::DeclaredPass> = registries
            .guard_passes()
            .filter(|declared| {
                declared.reports().iter().any(|local| {
                    codes.contains(&specforge_diagnostics::qualify(&declared.extension, local))
                })
            })
            .collect();
        if named.is_empty() {
            return self.checked_diagnostics();
        }
        let Some(runtime) = self.env.runtime() else {
            // Fail closed: a gate that cannot ask its guard passes is not satisfied.
            let mut found = self.checked_diagnostics();
            found.extend(
                named
                    .iter()
                    .flat_map(|declared| unevaluated(declared, codes, "no runtime")),
            );
            return found;
        };
        let mut checked = self.clone();
        if self.checks_skipped {
            checked.check(&mut crate::ledger::Unrecorded::plain());
        }
        let baseline = crate::baseline::Baseline::of_project(self);
        let context = crate::passes::PassContext {
            previous: Some(&baseline),
            judgements: Some(&checked.judgements),
            trigger: Some(trigger),
            files: Some(self.env.file_source(&specforge_registry::PlainDisk)),
            ..crate::passes::PassContext::of(checked.memo().entities())
        };
        let reports = crate::passes::run_extension_passes(
            named.iter().copied(),
            &context,
            &**runtime as &dyn specforge_wasm::WasmRuntime,
            registries.codes(),
        );
        let mut found = Vec::new();
        for (declared, report) in named.iter().zip(reports) {
            // Fail closed: a pass that failed cannot say its code is absent, so the gate fails on it.
            if report.summary["failed"] == true {
                let why = report
                    .findings
                    .first()
                    .map_or("it failed", |f| f.message.as_str());
                found.extend(unevaluated(declared, codes, why));
            }
            found.extend(report.findings);
        }
        checked.check_diagnostics.extend(found);
        checked.reported = OnceLock::new();
        checked.diagnostics()
    }

    /// The digests kept for the judged rules (ADR 0290): an entity's until the graph delta names it, a file's per stamp.
    pub fn digests(&self) -> &crate::judged::DigestMemo {
        &self.digests
    }

    /// Each judged rule's state on each entity, as the last checks found it (ADR 0290).
    pub fn judgements(&self) -> &crate::judged::Judgements {
        &self.judgements
    }

    /// The digests kept for the judged rules against a cold evaluation, when this project verifies (ADR 0290): the
    /// first divergence, or `Ok` when there is none (or nothing to compare).
    pub(crate) fn verify_digests(&self) -> Result<(), String> {
        if !self.verifies() || self.checks_skipped {
            return Ok(());
        }
        self.digests.verify(
            self.graph(),
            self.env.registries().field_forms(),
            &specforge_registry::PlainDisk,
        )
    }
}

/// An error of each of `codes` that `declared` reports, saying the pass could not be asked (`why`).
fn unevaluated(
    declared: &specforge_registry::DeclaredPass,
    codes: &[String],
    why: &str,
) -> Vec<specforge_common::Diagnostic> {
    declared
        .reports()
        .iter()
        .map(|local| specforge_diagnostics::qualify(&declared.extension, local))
        .filter(|code| codes.contains(code))
        .map(|code| {
            specforge_common::Diagnostic::from_extension(
                &declared.extension,
                code,
                specforge_common::Severity::Error,
                format!(
                    "guard pass '{}' could not be evaluated: {why}",
                    declared.pass.name
                ),
            )
        })
        .collect()
}
