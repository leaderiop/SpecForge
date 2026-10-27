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
}
