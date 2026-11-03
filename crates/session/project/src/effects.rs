//! The effects of a run (ADR 0337 D4), built in one place for every surface that chooses a mode: the CLI's
//! `--effects`, MCP `validate`'s `effects` and `watch`; and what a run's effects report, the one I206 summary of
//! the value checks this machine did not run.

use std::sync::Arc;

use specforge_common::{Diagnostic, ProjectRoot, codes};
use specforge_effects::mode::Log;
use specforge_effects::{EffectJournal, Effects, EffectsMode, Ports};

use crate::{Environment, SharedRuntime};

/// The effects of a run of the project at `root` in `mode`, through `ports`, over its effects journal (a loose
/// root's is the user's). Until grants (ports plan 02), `auto` approves no effect.
pub fn of(mode: EffectsMode, ports: &Ports, root: &ProjectRoot) -> Effects {
    Effects::of(
        mode,
        ports.clone(),
        Arc::new(|_| false),
        EffectJournal::for_root(root),
    )
}

/// The environment of the project at `root`, its extensions in `runtime`, asking `ports.git` of version control,
/// its effects in `mode` ([`of`]).
pub fn environment(
    root: &ProjectRoot,
    runtime: Option<SharedRuntime>,
    ports: &Ports,
    mode: EffectsMode,
) -> Environment {
    Environment::load(root, runtime)
        .with_git(Arc::clone(&ports.git))
        .with_effects(of(mode, ports, root))
}

/// I206, once per compile: how many effects `log` counts as not performed, by scope (`links 2, registries 3`);
/// none when every one was. A `machine` diagnostic: `--strict` never promotes it.
pub fn not_run(log: &Log) -> Option<Diagnostic> {
    let total: usize = log.not_run.values().sum();
    if total == 0 {
        return None;
    }
    let scopes: Vec<String> = log
        .not_run
        .iter()
        .filter(|(_, n)| **n > 0)
        .map(|(scope, n)| format!("{scope} {n}"))
        .collect();
    let plural = if total == 1 { "" } else { "s" };
    Some(
        Diagnostic::new(
            codes::I206,
            format!(
                "{total} value check{plural} not run on this machine ({}): offline, cached mode with no fresh \
                 result, or not approved",
                scopes.join(", ")
            ),
        )
        .with_suggestion("run `specforge check --effects online` to perform them"),
    )
}
