use super::*;
use specforge_effects::{Approval, CapabilityApproval, KeyedBy, ProjectKey};
use specforge_protocol_types::{PortScope, ProcessScope};
use specforge_test_macros::test as specforge_test;

fn approval(scope: &str) -> Approval {
    Approval::Capability(CapabilityApproval {
        project: ProjectKey("/p".to_string()),
        extension: "@acme/runner".to_string(),
        provenance: KeyedBy::Builtin,
        scope: scope.to_string(),
        granted: PortScope::Process(ProcessScope {
            argv: vec!["cargo".into(), "test".into()],
            deadline_ms: None,
        }),
        call_kinds: Vec::new(),
        secret_env: None,
        digest: None,
        specforge: String::new(),
        approved_at: Default::default(),
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
fn a_store_inside_the_project_is_refused_however_it_is_spelled_and_even_before_it_exists() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let canonical = project.canonicalize().unwrap();
    let inside = |store: PathBuf, project: &Path| {
        let error = UserConsent::at(store).within(project).list().unwrap_err();
        error.why.contains("inside the project")
    };
    // Its parent does not exist yet.
    assert!(inside(project.join("not/yet/consent.json"), &project));
    // `/var` and `/private/var` on macOS: one spelling for the store, the other for the project.
    assert!(inside(canonical.join("not/yet/consent.json"), &project));
    assert!(inside(project.join("not/yet/consent.json"), &canonical));
    // Up from a missing directory, and back in.
    let climbs = dir.path().join("elsewhere/../project/consent.json");
    assert!(inside(climbs, &project));
    let leaves = project.join("not/../../consent.json");
    assert!(UserConsent::at(leaves).within(&project).list().is_ok());
}

#[test]
#[cfg(unix)]
fn a_store_behind_a_link_into_the_project_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&project, &link).unwrap();
    let consent = UserConsent::at(link.join("not/yet/consent.json")).within(&project);
    assert!(
        consent
            .list()
            .unwrap_err()
            .why
            .contains("inside the project")
    );
}

#[test]
fn a_relative_store_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let consent = UserConsent::at(PathBuf::from("consent.json")).within(dir.path());
    assert!(consent.list().unwrap_err().why.contains("relative"));
    assert!(consent.approve(approval("tests")).is_err());
    assert!(!Path::new("consent.json").exists());
}

#[test]
fn with_no_home_there_is_no_store_and_nothing_is_approved() {
    for home in [
        None,
        Some(OsString::new()),
        Some(OsString::from("relative")),
    ] {
        let consent = UserConsent::under(home);
        let error = consent.list().unwrap_err();
        assert!(error.why.contains("no home directory"), "{error}");
        assert!(unusable_store(&error).message.contains("no home directory"));
        assert!(consent.approve(approval("tests")).is_err());
        assert!(
            !consent
                .store()
                .starts_with(&*std::env::temp_dir().to_string_lossy())
        );
    }
    let dir = tempfile::tempdir().unwrap();
    let home = UserConsent::under(Some(dir.path().as_os_str().to_owned()));
    assert_eq!(home.path(), dir.path().join(".specforge/consent.json"));
}

#[test]
fn the_default_store_is_refused_when_the_home_is_the_project_or_lies_in_it() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    let inner = project.join("nested");
    std::fs::create_dir_all(inner.join(".specforge")).unwrap();
    let committed = project.join(".specforge/consent.json");
    std::fs::create_dir_all(committed.parent().unwrap()).unwrap();
    std::fs::write(&committed, br#"{"version": 1}"#).unwrap();

    for home in [&project, &inner] {
        let consent = UserConsent::under(Some(home.as_os_str().to_owned())).within(&project);
        let error = consent.list().unwrap_err();
        assert!(error.why.contains("inside the project"), "{error}");
        assert!(unusable_store(&error).code == "W186");
        assert!(consent.approve(approval("tests")).is_err());
    }
    assert_eq!(std::fs::read(&committed).unwrap(), br#"{"version": 1}"#);

    // The ordinary case: the project lies under the home, the store beside it.
    let home = dir.path().to_path_buf();
    let consent = UserConsent::under(Some(home.into_os_string())).within(&project);
    assert!(consent.list().is_ok());
    consent.approve(approval("tests")).unwrap();
    assert!(consent.approved(&approval("tests")).unwrap());
}

#[test]
fn no_home_keeps_its_reason_when_the_project_is_judged() {
    let dir = tempfile::tempdir().unwrap();
    let error = UserConsent::under(None)
        .within(dir.path())
        .list()
        .unwrap_err();
    assert!(error.why.contains("no home directory"), "{error}");
}

#[test]
#[cfg(unix)]
fn the_lock_is_private_and_never_goes_through_a_link() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("consent.json");
    let lock = dir.path().join("consent.json.lock");
    let victim = dir.path().join("victim");

    // A dangling link: opening through it would create the victim.
    std::os::unix::fs::symlink(&victim, &lock).unwrap();
    let consent = UserConsent::at(store.clone());
    let error = consent.approve(approval("tests")).unwrap_err();
    assert!(error.why.contains("its lock cannot be opened"), "{error}");
    assert!(!victim.exists());
    assert!(!store.exists());

    std::fs::remove_file(&lock).unwrap();
    consent.approve(approval("tests")).unwrap();
    let mode = std::fs::metadata(&lock).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
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
    let git = |ops: &[GitOp], provenance: KeyedBy, project: &str| {
        let Approval::Capability(base) = approval("history") else {
            unreachable!()
        };
        Approval::Capability(CapabilityApproval {
            provenance,
            project: ProjectKey(project.to_string()),
            granted: PortScope::Git(GitScope { ops: ops.to_vec() }),
            ..base
        })
    };
    let digest = |d: &str| KeyedBy::Digest(d.to_string());
    consent
        .approve(git(&[GitOp::Read, GitOp::Write], KeyedBy::Builtin, "/p"))
        .unwrap();
    assert!(
        consent
            .approved(&git(&[GitOp::Read], KeyedBy::Builtin, "/p"))
            .unwrap(),
        "a narrower grant"
    );
    assert!(
        !consent
            .approved(&git(&[GitOp::Read], KeyedBy::Builtin, "/q"))
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
