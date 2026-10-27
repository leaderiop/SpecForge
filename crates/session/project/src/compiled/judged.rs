//! A project a write is being judged on (ADR 0289).

use super::CompiledProject;
use std::sync::OnceLock;

impl CompiledProject {
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
