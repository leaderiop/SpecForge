//! The bound of an MCP request (ADR 0092, ADR 0104).
//!
//! The rule is `specforge_ops::Containment`: it resolves a name the way the file system would and
//! says whether it lands inside a directory, and `From<Outside> for OpError` words the refusal. This
//! module builds the bound of a request.

use std::path::{Path, PathBuf};

use crate::state::McpState;
use specforge_ops::Containment;
use specforge_project::CompiledProject;

/// The bound of a request: the served project's root; while none is served, the server's root;
/// unbounded when the server has neither (a server made with no root names nothing to stay inside).
pub(crate) fn bound(state: &McpState) -> Containment {
    let project = state.session().project();
    match (project.root(), state.root.as_deref()) {
        (Some(_), _) => of_project(project, state.consent.as_deref()),
        (None, Some(root)) => Containment::within(root),
        (None, None) => Containment::unbounded(),
    }
}

/// The bound of a call on `project`, on disk: its root, and a `<name>/…` path names the spec root and the named roots
/// this machine may reach (ADR 0336 D6), under `consent`, else the user's store.
pub(crate) fn of_project(
    project: &CompiledProject,
    consent: Option<&dyn specforge_effects::Consent>,
) -> Containment {
    let Some(root) = project.root() else {
        return Containment::unbounded();
    };
    let user;
    let consent = match consent {
        Some(consent) => consent,
        None => {
            user = specforge_ops::consent::UserConsent::user(root);
            &user
        }
    };
    Containment::within(root).with_roots(specforge_project::effects::reachable_roots(
        project.environment(),
        root,
        consent,
    ))
}

/// Where `path` really is: absolute, every symlink followed, a missing tail joined on.
pub(crate) fn landed(path: &Path) -> PathBuf {
    Containment::unbounded()
        .resolve(path)
        .map_or_else(|_| path.to_path_buf(), |held| held.real)
}

#[cfg(test)]
mod tests {
    use specforge_effects::Consent;
    use specforge_effects::testing::MemoryConsent;
    use specforge_project::Environment;

    use super::*;

    #[test]
    fn a_spec_root_outside_the_project_is_named_only_once_approved() {
        let outside = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let config = serde_json::json!({"name": "p", "spec_root": outside.path()}).to_string();
        std::fs::write(dir.path().join("specforge.json"), config).unwrap();
        std::fs::write(outside.path().join("secret.md"), "x").unwrap();
        let root = specforge_common::ProjectRoot::of(dir.path()).unwrap();
        let project = CompiledProject::of(Environment::load(&root, None));

        let refused = MemoryConsent::default();
        let path = Path::new("<spec>/secret.md");
        assert!(of_project(&project, Some(&refused)).resolve(path).is_err());

        let spec = &project.environment().effects_allowed().roots[0];
        assert_eq!(spec.name, "spec");
        let target = spec.target.as_ref().unwrap().display().to_string();
        let approved = MemoryConsent::default();
        let approval = specforge_effects::RootApproval {
            project: specforge_effects::grant::project_key(dir.path()),
            root: "spec".to_string(),
            target,
            approved_at: Default::default(),
        };
        approved
            .approve(specforge_effects::Approval::Root(approval))
            .unwrap();
        let held = of_project(&project, Some(&approved)).resolve(path);
        assert!(held.is_ok(), "{held:?}");
    }
}
