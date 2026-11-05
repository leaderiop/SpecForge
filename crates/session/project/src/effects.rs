//! The effects of a run (ADR 0337 D4), built in one place for every surface that chooses a mode: the CLI's
//! `--effects`, MCP `validate`'s `effects` and `watch`; and what a run's effects report, the one I206 summary of
//! the value checks this machine did not run.

use std::path::Path;
use std::sync::Arc;

use specforge_common::{Diagnostic, ProjectRoot, codes};
use specforge_effects::mode::Log;
use specforge_effects::{
    Approval, Consent, EffectJournal, Effects, EffectsMode, Grant, NoConsent, Ports, Provenance,
    RootApproval, Roots,
};

use crate::{Environment, SharedRuntime};

/// The effects of a run of `env`, the project at `root`, in `mode`, through `ports`, over its effects journal (a
/// loose root's is the user's). A live mode judges each request by the grant of the extension asking: under `consent`
/// in `auto`, the only mode that reads it; `online` is the person's approval for the run, so what the extension
/// declares and the ceiling still decide.
pub fn of(
    mode: EffectsMode,
    ports: &Ports,
    env: &Environment,
    root: &ProjectRoot,
    consent: &dyn Consent,
) -> Effects {
    let grants = match mode {
        EffectsMode::Auto => grants(env, root.dir(), consent),
        EffectsMode::Online => grants(env, root.dir(), &NoConsent)
            .into_iter()
            .map(Grant::approved_all)
            .collect(),
        EffectsMode::Cached | EffectsMode::Replay => Vec::new(),
    };
    Effects::of(mode, ports.clone(), grants, EffectJournal::for_root(root))
}

/// The effects of a call lent `ports` in `mode`, over the journal of `root` (none: detached). A guest's request reaches
/// them only once its grant permitted it ([`Effects::serve_lent`]), so they hold no grants of their own.
pub fn lent(mode: EffectsMode, ports: &Ports, root: Option<&ProjectRoot>) -> Effects {
    let journal = root.map_or_else(EffectJournal::detached, EffectJournal::for_root);
    Effects::of(mode, ports.clone(), Vec::new(), journal)
}

/// Where `extension`'s code came from: as the runtime loaded it, else as the load recorded it. Only an environment
/// assembled in memory has neither: it is keyed as a third party by its name, so no builtin approval meets it.
pub fn provenance_of(env: &Environment, extension: &str) -> Provenance {
    env.runtime()
        .and_then(|runtime| runtime.provenance(extension))
        .or_else(|| {
            env.enabled()
                .iter()
                .find(|e| e.name == extension)
                .and_then(|e| e.provenance.clone())
        })
        .unwrap_or_else(|| {
            #[cfg(not(feature = "testing"))]
            debug_assert!(false, "extension {extension} loaded with no provenance");
            Provenance::third_party(extension.as_bytes())
        })
}

/// The grant of every extension of `env`, the project at `root`, that declares a capability, in declaration order.
pub fn grants(env: &Environment, root: &Path, consent: &dyn Consent) -> Vec<Grant> {
    let roots = env
        .effects_allowed()
        .with_named(Roots::new(root, env.spec_root()));
    env.registries()
        .declarations()
        .iter()
        .filter(|declaration| !declaration.sound_capabilities().is_empty())
        .map(|declaration| {
            Grant::decide(
                declaration,
                &provenance_of(env, declaration.name()),
                &env.effects_allowed().ceiling,
                &roots,
                consent,
            )
        })
        .collect()
}

/// The environment of the project at `root`, its extensions in `runtime`, asking `ports.git` of version control,
/// its effects in `mode` ([`of`]) under `consent`.
pub fn environment(
    root: &ProjectRoot,
    runtime: Option<SharedRuntime>,
    ports: &Ports,
    mode: EffectsMode,
    consent: &dyn Consent,
) -> Environment {
    let env = Environment::load(root, runtime).with_git(Arc::clone(&ports.git));
    let effects = of(mode, ports, &env, root, consent);
    env.with_effects(effects)
}

/// The roots beside the project's a request path may name on this machine (ADR 0336 D6): the spec root and the named
/// roots inside the project, and those outside it `consent` approved, a spec root outside it among them. An unusable
/// store approves nothing.
pub fn reachable_roots(
    env: &Environment,
    root: &Path,
    consent: &dyn Consent,
) -> Vec<(String, std::path::PathBuf)> {
    let project = specforge_effects::grant::project_key(root);
    let mut roots = Vec::new();
    if !env.effects_allowed().spec_outside() {
        roots.push(("spec".to_string(), env.spec_root().to_path_buf()));
    }
    for named in &env.effects_allowed().roots {
        let Some(target) = &named.target else {
            continue;
        };
        let approved = || {
            consent
                .approved(&Approval::Root(RootApproval {
                    project: project.clone(),
                    root: named.name.clone(),
                    target: target.display().to_string(),
                    approved_at: Default::default(),
                }))
                .unwrap_or(false)
        };
        if named.inside || approved() {
            roots.push((named.name.clone(), target.clone()));
        }
    }
    roots
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
                 result, not approved, or over the run's budget",
                scopes.join(", ")
            ),
        )
        .with_suggestion("run `specforge check --effects online` to perform them"),
    )
}
