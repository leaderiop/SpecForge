//! The management operations (the extensions and providers listings,
//! doctor, remove, collect, inference progress and gaps) read the project
//! from their view (ADR 0015, "Management operations").

use specforge_common::Diagnostic;
use specforge_ops::collect::{Consent, Mode, Request};
use specforge_ops::extension::{self, RemoveRequest, Status};
use specforge_ops::view::ProjectView;
use specforge_project::EnabledExtension;
use specforge_project::coverage::RecordedCoverage;
use specforge_registry::RegistryBuild;
use specforge_test::prelude::*;

use crate::view_support::Project;

/// A project whose compile read `specforge.json` as enabling
/// `@acme/missing@1.2.0` and configuring one provider, with one lock entry
/// at its root, while the `specforge.json` on disk now says something
/// else entirely.
fn project() -> Project {
    let mut project = Project::new("behavior a \"A\" {\n}\n", RegistryBuild::default());
    let config = serde_json::json!({
        "extensions": ["@acme/missing@1.2.0"],
        "providers": [{"scheme": "gh", "alias": "work", "extension": "@acme/issues"}],
    });
    project.env.config.extensions = vec!["@acme/missing@1.2.0".into()];
    project.env.config.raw = Some(config);
    project.env.enabled = vec![EnabledExtension::of("@acme/missing@1.2.0", None)];
    project.env.config_found = true;
    let root = project.dir.path();
    std::fs::write(
        root.join("specforge.json"),
        r#"{"extensions": ["@other/x@9.9.9"], "providers": []}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("specforge.lock"),
        serde_json::json!({"lockfile_version": 1, "entries": [
            {"name": "@acme/locked", "version": "1.0.0", "source": "registry", "wasm_hash": "h"}
        ]})
        .to_string(),
    )
    .unwrap();
    // The compile read the lock as it is at this root.
    project.env.lock = specforge_wasm::LockState::at(root);
    project
}

#[specforge_test(
    behavior = "management_operations_over_the_project_view",
    verify = "Management Operations over the Project View: management operations hold — project_compiled, one_project_read, root_for_disk"
)]
fn management_operations_read_the_project_from_their_view() {
    // project_compiled: the view is what a compile supplied.
    let project = project();
    let e028 = [Diagnostic::new(
        specforge_common::codes::E028,
        "extension '@acme/missing' is not installed",
    )];
    let view = project.view().reporting(&e028);

    // one_project_read: the config, what each entry enabled and what the
    // surface reports come from the view, never specforge.json again.
    let listing = extension::list(&view);
    let listed: Vec<(&str, Status, Option<&str>)> = listing
        .extensions
        .iter()
        .map(|e| (e.name.as_str(), e.status, e.version.as_deref()))
        .collect();
    assert_eq!(
        listed,
        [
            ("@acme/locked", Status::NotConfigured, Some("1.0.0")),
            ("@acme/missing", Status::NotLoaded, Some("1.2.0")),
        ]
    );
    let providers = extension::providers(&view);
    assert_eq!(providers.providers.len(), 1, "{providers:?}");
    assert_eq!(providers.providers[0].alias, "work");

    let report = specforge_ops::doctor::diagnose_with(&view, true);
    let failures: Vec<&str> = report
        .load_failures
        .iter()
        .map(|f| f.code.as_str())
        .collect();
    assert_eq!(failures, ["E028"]);

    // root_for_disk: the lock is read at the view's root ...
    assert_eq!(listing.locked.len(), 1);
    assert_eq!(report.extensions_checked, 1);

    // ... and a view without one: the listings and doctor answer from what
    // the view enabled and loaded; the others refuse with no_project.
    let recorded = RecordedCoverage::default();
    let rootless = ProjectView::new(&project.graph, &project.env, None, &recorded).reporting(&e028);
    let listing = extension::list(&rootless);
    assert!(listing.locked.is_empty());
    assert_eq!(listing.extensions.len(), 1);
    assert_eq!(
        specforge_ops::doctor::diagnose_with(&rootless, true).extensions_checked,
        0
    );
    let refused = |result: Result<(), specforge_ops::OpError>| {
        assert_eq!(result.unwrap_err().code, "no_project");
    };
    refused(
        extension::remove(
            &rootless,
            &RemoveRequest {
                name: "@acme/missing",
                force: false,
                dry_run: true,
            },
        )
        .map(drop),
    );
    refused(specforge_ops::infer::progress(&rootless).map(drop));
    let runtime = specforge_wasm::testing::InProcessRuntime::new();
    refused(specforge_ops::infer::gaps(&rootless, &runtime).map(drop));
    refused(
        specforge_ops::collect::collect(
            &rootless,
            &runtime,
            Request {
                runner: None,
                mode: Mode::NoRun,
                consent: Consent::Approved,
                announce: &mut |_, _| {},
            },
        )
        .map(drop),
    );
    assert!(
        !project.dir.path().join("specforge-report.json").exists(),
        "nothing was written"
    );
}
