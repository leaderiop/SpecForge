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
        );
        self.check_diagnostics.extend(found);
        self.reported = OnceLock::new();
        self
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
