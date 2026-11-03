//! What an [`Environment`] asks of the outside (ADR 0287, ADR 0335, ADR 0337): the git port its surface gave, where
//! the repository is, and the effects mode every http or process effect of a run is served in.

use specforge_effects::git::{Git, Located};
use specforge_effects::{Effects, SharedGit};

use crate::Environment;

impl Environment {
    /// This environment asking `git` of version control.
    pub fn with_git(mut self, git: SharedGit) -> Self {
        self.git = Some(git);
        self
    }

    /// Where the repository of this project is, asked of `git`: a repository once found is kept.
    pub(crate) fn place(&self, git: &dyn Git) -> Located {
        if let Some(found) = self.place.get() {
            return found.clone();
        }
        let located = git.locate(self.root());
        if matches!(located, Located::Repo(_)) {
            let _ = self.place.set(located.clone());
        }
        located
    }

    /// This environment's effects in `effects`' mode (a surface's `--effects`, ADR 0337 D4).
    pub fn with_effects(mut self, effects: Effects) -> Self {
        self.effects = effects;
        self
    }

    /// Its effects: the mode, the ports a live mode performs through, the journal and what the run consulted.
    pub fn effects(&self) -> &Effects {
        &self.effects
    }

    /// The git port this environment was given; none: no judgement against history.
    pub fn git(&self) -> Option<SharedGit> {
        self.git.clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::CompiledProject;
    use specforge_effects::EffectsMode;
    use specforge_test_macros::test as specforge_test;

    #[specforge_test(
        behavior = "bring_session_up_to_date",
        verify = "a project opens with cached effects and no ports, over its own effects journal"
    )]
    fn a_project_opens_with_cached_effects_and_no_ports() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("specforge.json"), r#"{"extensions": []}"#).unwrap();
        let root = specforge_common::ProjectRoot::of(dir.path()).unwrap();
        let compiled = CompiledProject::compile(&root, None);
        let effects = compiled.environment().effects();
        assert_eq!(effects.mode(), EffectsMode::Cached);
        assert!(effects.ports().is_none(), "a compile performs no effect");
        assert_eq!(
            effects.journal().dir(),
            Some(root.dir().join(".specforge/effects/journal").as_path())
        );
        // One assembled in memory has no journal to write.
        assert_eq!(
            CompiledProject::detached()
                .environment()
                .effects()
                .journal()
                .dir(),
            None
        );
    }

    #[test]
    fn an_effects_rerun_runs_the_checks_in_its_own_effects_over_the_same_compile() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("specforge.json"), r#"{"extensions": []}"#).unwrap();
        let root = specforge_common::ProjectRoot::of(dir.path()).unwrap();
        let compiled = CompiledProject::compile(&root, None);
        let journal = compiled.environment().effects().journal().clone();
        let rerun = compiled.effects_rerun(std::sync::Arc::new(
            specforge_effects::Effects::replay(journal),
        ));
        assert_eq!(rerun.effects().mode(), EffectsMode::Replay);
        assert_eq!(
            compiled.effects().mode(),
            EffectsMode::Cached,
            "the served compile is unchanged"
        );
        assert_eq!(rerun.diagnostics(), compiled.diagnostics());
    }
}
