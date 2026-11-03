//! The effects of a run (ADR 0337 D4), built in one place for every surface that chooses a mode: the CLI's
//! `--effects`, MCP `validate`'s `effects` and `watch`.

use std::sync::Arc;

use specforge_common::ProjectRoot;
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
