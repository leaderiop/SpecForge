//! The effects a compiled project's checks are served in (ADR 0337 D4): its environment's, or one call's own.

use std::sync::Arc;

use specforge_effects::Effects;

use super::CompiledProject;
use crate::baseline::HeldBaseline;
use crate::ledger::{Consulted, Unrecorded};

impl CompiledProject {
    /// The effects its checks were served in: one call's own ([`Self::effects_rerun`]), else its environment's.
    pub fn effects(&self) -> &Effects {
        self.effects
            .as_deref()
            .unwrap_or_else(|| self.env.effects())
    }

    /// This compile with its checks run again in `effects`, for one call (MCP `validate`'s `effects`), as
    /// [`Self::judged_since`] re-judges: nothing is recompiled, and this project is unchanged.
    pub fn effects_rerun(&self, effects: Arc<Effects>) -> CompiledProject {
        let mut project = self.clone();
        project.effects = Some(effects);
        project.check(&mut Unrecorded::plain());
        project
    }

    /// The effect records this run of the checks consulted (ADR 0337 D5), and those the checks of the `baseline` it
    /// judged against read at the commit: each an input of the checks, so a write to one runs them again.
    pub(super) fn consulted(&self, baseline: Option<&HeldBaseline>) -> Vec<Consulted> {
        let log = self.effects().log();
        let consulted = log.consulted.into_iter();
        consulted
            .map(|(path, expires)| Consulted { path, expires })
            .chain(baseline.into_iter().flat_map(HeldBaseline::consulted))
            .collect()
    }
}
