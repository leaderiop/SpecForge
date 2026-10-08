//! Peer requirements as `doctor` reports them over a compiled project.

use std::path::Path;

use specforge_common::Severity;
use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta};
use specforge_ops::doctor::{BinaryIssue, diagnose_with};
use specforge_ops::view::ProjectView;
use specforge_project::CompiledProject;
use specforge_protocol_types::PeerDependency;
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

/// Pinned until T3 (ADR 0041): doctor reads only the lock, so a peer a
/// builtin satisfies is "not installed".
#[test]
fn pin_doctor_reports_a_builtin_peer_as_not_installed() {
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

    assert_eq!(
        report.issues,
        [BinaryIssue::PeerMismatch {
            name: "@acme/app".into(),
            peer: "@specforge/software".into(),
            required: "^1.0".into(),
            installed: None,
        }]
    );
    assert!(report.has_errors());
}

/// Pinned until T3: doctor judges a malformed range its own way, though `check` reports
/// it as E073.
#[test]
fn pin_check_reports_e073_and_doctor_its_own_peer_mismatch_on_a_malformed_range() {
    let dir = project(&["@acme/base", "@acme/bad"]);
    let runtime = InProcessRuntime::new()
        .with(served("@acme/base", "1.0.0", &[]))
        .with(served("@acme/bad", "1.0.0", &[("@acme/base", "one-ish")]));
    specforge_installed::testing::install(dir.path(), &["@acme/base", "@acme/bad"]);
    locked_peers(dir.path(), "@acme/bad", &[("@acme/base", "one-ish")]);

    let compiled = CompiledProject::compile(dir.path(), Some(&runtime));
    let all = compiled.diagnostics();
    let peer_diagnostics: Vec<_> = all
        .iter()
        .filter(|d| matches!(d.code.as_str(), "E073" | "E027"))
        .collect();
    assert_eq!(peer_diagnostics.len(), 1, "{peer_diagnostics:?}");
    assert_eq!(peer_diagnostics[0].code, "E073");
    assert_eq!(peer_diagnostics[0].severity, Severity::Error);

    let report = diagnose_with(&ProjectView::of(&compiled), true);
    let finding = report
        .findings
        .iter()
        .find(|f| f.code == "peer_mismatch")
        .expect("doctor reports the peer");
    assert_eq!(format!("{:?}", finding.status), "Error");
    assert!(
        finding.check.contains("is not a semver range"),
        "{finding:?}"
    );
}
