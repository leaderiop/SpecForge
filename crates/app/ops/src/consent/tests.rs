use super::*;
use specforge_effects::{Approval, CapabilityApproval, Keyed};
use specforge_protocol_types::{PortScope, ProcessScope};
use specforge_test_macros::test as specforge_test;

fn approval(scope: &str) -> Approval {
    Approval::Capability(CapabilityApproval {
        project: "/p".to_string(),
        extension: "@acme/runner".to_string(),
        provenance: Keyed::Builtin,
        scope: scope.to_string(),
        granted: PortScope::Process(ProcessScope {
            argv: vec!["cargo".into(), "test".into()],
            deadline_ms: None,
        }),
        call_kinds: Vec::new(),
        digest: String::new(),
        specforge: String::new(),
        approved_at: String::new(),
    })
}

#[specforge_test(port = "Consent", verify = "Consent contract is satisfied")]
fn the_user_consent_keeps_the_port_contract() {
    let dir = tempfile::tempdir().unwrap();
    specforge_effects::testing::consent_contract(&UserConsent::at(
        dir.path().join("home").join("consent.json"),
    ));
}

#[specforge_test(
    behavior = "approve_capability",
    verify = "the consent store is replaced privately and atomically, never through a link"
)]
#[cfg(unix)]
fn the_consent_store_is_replaced_privately_never_through_a_link() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let victim = dir.path().join("victim.json");
    let held = br#"{"version": 1}"#;
    std::fs::write(&victim, held).unwrap();
    std::fs::set_permissions(&victim, std::fs::Permissions::from_mode(0o644)).unwrap();
    let store = dir.path().join("consent.json");
    std::os::unix::fs::symlink(&victim, &store).unwrap();
    UserConsent::at(store.clone())
        .approve(approval("tests"))
        .unwrap();
    assert_eq!(
        std::fs::read(&victim).unwrap(),
        held,
        "the link's target is untouched"
    );
    let meta = std::fs::symlink_metadata(&store).unwrap();
    assert!(meta.file_type().is_file(), "the store is a regular file");
    assert_eq!(meta.permissions().mode() & 0o777, 0o600);

    let nested = dir
        .path()
        .join("home")
        .join(".specforge")
        .join("consent.json");
    UserConsent::at(nested.clone())
        .approve(approval("tests"))
        .unwrap();
    let parent = std::fs::metadata(nested.parent().unwrap()).unwrap();
    assert_eq!(parent.permissions().mode() & 0o777, 0o700);
}

#[specforge_test(
    behavior = "approve_capability",
    verify = "an approval is never written over a consent store that cannot be read"
)]
fn the_consent_store_is_never_overwritten_when_it_cannot_be_read() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("consent.json");
    let broken = br#"{"version": 1, "capabilities": [],}"#;
    std::fs::write(&store, broken).unwrap();
    let consent = UserConsent::at(store.clone());
    let err = consent.approve(approval("tests")).unwrap_err();
    assert!(err.to_string().contains("consent.json"), "{err}");
    assert_eq!(std::fs::read(&store).unwrap(), broken.to_vec());
    assert!(consent.approved(&approval("tests")).is_err());
    let w186 = unusable_store(&err);
    assert_eq!(w186.code, "W186");
    assert!(w186.message.contains("consent.json"), "{}", w186.message);
}

#[test]
fn a_store_named_inside_the_project_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    std::fs::create_dir_all(project.join(".config")).unwrap();
    let inside = project.join(".config").join("consent.json");
    let consent = UserConsent::at(inside.clone()).within(&project);
    let err = consent.list().unwrap_err();
    assert!(err.why.contains("inside the project"), "{err}");
    assert!(consent.approve(approval("tests")).is_err());
    assert!(!inside.exists());
    let outside = UserConsent::at(dir.path().join("consent.json")).within(&project);
    assert!(outside.list().is_ok());
}

#[test]
fn a_newer_store_version_is_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("consent.json");
    let newer = br#"{"version": 2, "capabilities": [], "roots": [], "grants": []}"#;
    std::fs::write(&store, newer).unwrap();
    let consent = UserConsent::at(store.clone());
    let err = consent.list().unwrap_err();
    assert!(err.why.contains("newer specforge"), "{err}");
    assert!(consent.approve(approval("tests")).is_err());
    assert_eq!(std::fs::read(&store).unwrap(), newer.to_vec());
}

#[test]
fn two_approvals_at_once_keep_both() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("consent.json");
    let threads: Vec<_> = (0..8)
        .map(|n| {
            let store = store.clone();
            std::thread::spawn(move || {
                UserConsent::at(store)
                    .approve(approval(&format!("s{n}")))
                    .unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(UserConsent::at(store).list().unwrap().capabilities.len(), 8);
}

#[specforge_test(
    behavior = "approve_capability",
    verify = "an approval is keyed by project, extension provenance and scope, and covers narrower grants"
)]
fn an_approval_is_keyed_by_provenance_and_covers_narrower_grants() {
    use specforge_protocol_types::{GitOp, GitScope};
    let dir = tempfile::tempdir().unwrap();
    let consent = UserConsent::at(dir.path().join("consent.json"));
    let git = |ops: &[GitOp], provenance: Keyed, project: &str| {
        let Approval::Capability(base) = approval("history") else {
            unreachable!()
        };
        Approval::Capability(CapabilityApproval {
            provenance,
            project: project.to_string(),
            granted: PortScope::Git(GitScope { ops: ops.to_vec() }),
            ..base
        })
    };
    let digest = |d: &str| Keyed::Digest(d.to_string());
    consent
        .approve(git(&[GitOp::Read, GitOp::Write], Keyed::Builtin, "/p"))
        .unwrap();
    assert!(
        consent
            .approved(&git(&[GitOp::Read], Keyed::Builtin, "/p"))
            .unwrap(),
        "a narrower grant"
    );
    assert!(
        !consent
            .approved(&git(&[GitOp::Read], Keyed::Builtin, "/q"))
            .unwrap(),
        "another project"
    );
    assert!(
        !consent
            .approved(&git(&[GitOp::Read], digest("ab"), "/p"))
            .unwrap(),
        "a builtin approval never meets a third-party load"
    );
    consent
        .approve(git(&[GitOp::Read], digest("ab"), "/p"))
        .unwrap();
    assert!(
        !consent
            .approved(&git(&[GitOp::Read], digest("cd"), "/p"))
            .unwrap(),
        "another blob"
    );
    assert!(
        !consent
            .approved(&git(&[GitOp::Read, GitOp::Write], digest("ab"), "/p"))
            .unwrap(),
        "a wider grant asks again"
    );
}
