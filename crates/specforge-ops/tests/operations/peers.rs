//! Peer requirements as `doctor` reports them over a compiled project.

use std::path::Path;

use specforge_common::Severity;
use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta};
use specforge_ops::doctor::diagnose_with;
use specforge_ops::view::ProjectView;
use specforge_project::CompiledProject;
use specforge_protocol_types::PeerDependency;
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;

/// `name` at `version`, served in process, declaring `peers` (required).
fn served(
    name: &'static str,
    version: &'static str,
    peers: &'static [(&'static str, &'static str)],
) -> impl Fn() -> ContributionsBuilder + Send + Sync + 'static {
    move || {
        let mut meta = ExtensionMeta::new(name, version);
        meta.peer_dependencies = peers
            .iter()
            .map(|(peer, range)| PeerDependency {
                name: (*peer).into(),
                version: (*range).into(),
                optional: false,
            })
            .collect();
        ContributionsBuilder::new(meta)
    }
}

/// Rewrite `specforge.lock` so `name`'s entry records `peers`.
fn locked_peers(root: &Path, name: &str, peers: &[(&str, &str)]) {
    let path = root.join("specforge.lock");
    let mut lock: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for entry in lock["entries"].as_array_mut().unwrap() {
        if entry["name"] == name {
            entry["peer_dependencies"] = peers
                .iter()
                .map(|(peer, range)| {
                    serde_json::json!({"name": peer, "version": range, "optional": false})
                })
                .collect();
        }
    }
    std::fs::write(&path, lock.to_string()).unwrap();
}

fn project(extensions: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let config = serde_json::json!({"name": "p", "version": "0.1.0", "extensions": extensions});
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    dir
}

#[specforge_test(
    behavior = "run_doctor_check",
    verify = "a peer a builtin satisfies is not reported"
)]
fn a_peer_a_builtin_satisfies_is_not_reported_by_doctor() {
    let dir = project(&["@specforge/software", "@acme/app"]);
    let runtime = InProcessRuntime::new()
        .with(served("@specforge/software", "1.0.0", &[]))
        .with(served(
            "@acme/app",
            "1.0.0",
            &[("@specforge/software", "^1.0")],
        ));
    specforge_installed::testing::install(dir.path(), &["@acme/app"]);
    locked_peers(dir.path(), "@acme/app", &[("@specforge/software", "^1.0")]);

    let compiled = CompiledProject::compile(dir.path(), Some(&runtime));
    assert!(
        !compiled.diagnostics().iter().any(|d| d.code == "E027"),
        "{:?}",
        compiled.diagnostics()
    );
    let report = diagnose_with(&ProjectView::of(&compiled), true);

    assert!(report.peers.is_empty(), "{:?}", report.peers);
    assert!(report.issues.is_empty(), "{:?}", report.issues);
    assert!(!report.has_errors(), "{:?}", report.findings);
    assert!(report.extensions_ok());
}

#[specforge_test(
    behavior = "run_doctor_check",
    verify = "doctor reports the peer requirements check reports, with the remedy each suggests"
)]
fn doctor_reports_the_peer_requirements_check_reports() {
    let dir = project(&["@acme/base", "@acme/bad"]);
    let runtime = InProcessRuntime::new()
        .with(served("@acme/base", "1.0.0", &[]))
        .with(served("@acme/bad", "1.0.0", &[("@acme/base", "one-ish")]));
    specforge_installed::testing::install(dir.path(), &["@acme/base", "@acme/bad"]);
    locked_peers(dir.path(), "@acme/bad", &[("@acme/base", "one-ish")]);

    let compiled = CompiledProject::compile(dir.path(), Some(&runtime));
    let all = compiled.diagnostics();
    let reported: Vec<_> = all
        .iter()
        .filter(|d| matches!(d.code.as_str(), "E073" | "E027"))
        .collect();
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert_eq!(reported[0].code, "E073");
    assert_eq!(reported[0].severity, Severity::Error);

    let report = diagnose_with(&ProjectView::of(&compiled), true);
    assert_eq!(report.peers.len(), 1, "{:?}", report.peers);
    assert_eq!(report.peers[0].code, "E073");
    assert_eq!(report.peers[0].message, reported[0].message);
    assert_eq!(
        Some(report.peers[0].suggestion.as_str()),
        reported[0].suggestion.as_deref()
    );
    let finding = report
        .findings
        .iter()
        .find(|f| f.code == "E073")
        .expect("doctor reports the peer");
    assert_eq!(format!("{:?}", finding.status), "Error");
    assert_eq!(finding.remediation, report.peers[0].suggestion);
    assert!(report.issues.is_empty());
    assert!(report.has_errors() && !report.extensions_ok());

    // An unsatisfied range is reported the same way: `@acme/app` wants a
    // base this project does not have.
    let dir = project(&["@acme/base", "@acme/app"]);
    let runtime = InProcessRuntime::new()
        .with(served("@acme/base", "1.0.0", &[]))
        .with(served("@acme/app", "1.0.0", &[("@acme/base", "^2.0")]));
    specforge_installed::testing::install(dir.path(), &["@acme/base", "@acme/app"]);
    let compiled = CompiledProject::compile(dir.path(), Some(&runtime));
    let all = compiled.diagnostics();
    let e027: Vec<_> = all.iter().filter(|d| d.code == "E027").collect();
    assert_eq!(e027.len(), 1, "{e027:?}");
    let report = diagnose_with(&ProjectView::of(&compiled), true);
    assert_eq!(report.peers.len(), 1, "{:?}", report.peers);
    assert_eq!(report.peers[0].code, "E027");
    assert_eq!(report.peers[0].message, e027[0].message);
}
