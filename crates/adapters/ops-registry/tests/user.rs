//! `User`, the user's registry files (ADR 0178): what a read of a registry carries from them.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_ops::registry::Registry;
use specforge_ops_registry::{ConfiguredRegistry, Kept, RegistryRoot, TokenSource, User};
use specforge_package::PackageName;
use specforge_project::testing::TestProject;
use specforge_registry_client::testing::MemoryClient;
use specforge_test_macros::test as specforge_test;

#[specforge_test(requirement = "FEATURE-110")]
fn a_kept_token_whose_secret_is_gone_refuses_the_read() {
    let project = TestProject::new().config(|c| {
        c["name"] = "p".into();
        c["registries"] = serde_json::json!([
            { "alias": "local", "url": "https://local.invalid", "default_registry": true }
        ]);
    });
    let client = MemoryClient::new();
    // An entry whose secret `login --token` kept is gone: nothing under secrets/.
    std::fs::create_dir_all(project.home()).unwrap();
    std::fs::write(
        project.home().join("credentials.json"),
        r#"{"registries":{"https://local.invalid":{}}}"#,
    )
    .unwrap();
    let registry = ConfiguredRegistry::for_project(&project.project_root(), "add", &crate::ports())
        .with_client(client.clone())
        .as_user(User::at(project.home(), None));

    let outcome = registry.versions(&PackageName::parse("@acme/tool").unwrap());

    let calls = client.calls();
    let error = outcome.expect_err("a lost secret is not an empty bearer token");
    assert_eq!(error.code, "R-AUTH-021", "{error:?} (calls: {calls:?})");
    assert!(calls.is_empty(), "{calls:?}");
}

#[specforge_test(requirement = "FEATURE-110")]
fn a_read_carries_the_credential_kept_for_its_root_only() {
    // Two entries with one alias and two roots (W140): they do not share a credential.
    let project = TestProject::new().config(|c| {
        c["name"] = "p".into();
        c["registries"] = serde_json::json!([
            { "alias": "main", "url": "https://a.invalid", "scope_filter": "@a" },
            { "alias": "main", "url": "https://b.invalid", "default_registry": true }
        ]);
    });
    std::fs::create_dir_all(project.home()).unwrap();
    std::fs::write(
        project.home().join("credentials.json"),
        r#"{"registries":{"https://a.invalid":{"token":"ta"}}}"#,
    )
    .unwrap();
    let client = MemoryClient::new();
    let registry = ConfiguredRegistry::for_project(&project.project_root(), "add", &crate::ports())
        .with_client(client.clone())
        .as_user(User::at(project.home(), None));

    let _ = registry.versions(&PackageName::parse("@a/x").unwrap());
    let _ = registry.versions(&PackageName::parse("@b/x").unwrap());

    let sent: Vec<Option<String>> = client
        .calls()
        .iter()
        .map(|call| call.credential.as_ref().map(|c| c.token().to_string()))
        .collect();
    assert_eq!(sent, [Some("ta".to_string()), None]);
}

#[cfg(unix)]
#[specforge_test(requirement = "FEATURE-110")]
fn a_user_store_another_user_could_have_written_counts_as_missing() {
    use std::os::unix::fs::PermissionsExt;
    let project = TestProject::new().config(|c| {
        c["name"] = "p".into();
        c["registries"] = serde_json::json!([
            { "alias": "main", "url": "https://a.invalid", "default_registry": true }
        ]);
    });
    std::fs::create_dir_all(project.home()).unwrap();
    let store = project.home().join("credentials.json");
    std::fs::write(
        &store,
        r#"{"registries":{"https://a.invalid":{"token":"ta"}}}"#,
    )
    .unwrap();
    std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o666)).unwrap();
    let client = MemoryClient::new();
    let registry = ConfiguredRegistry::for_project(&project.project_root(), "add", &crate::ports())
        .with_client(client.clone())
        .as_user(User::at(project.home(), None));

    let _ = registry.versions(&PackageName::parse("@a/x").unwrap());

    let sent: Vec<bool> = client
        .calls()
        .iter()
        .map(|c| c.credential.is_some())
        .collect();
    assert_eq!(sent, [false], "a group-writable store gives no credential");
}

#[specforge_test(requirement = "FEATURE-110")]
fn a_user_store_a_git_worktree_tracks_is_refused_before_any_request() {
    use specforge_effects::testing::FakeGit;
    let project = TestProject::new().config(|c| {
        c["name"] = "p".into();
        c["registries"] = serde_json::json!([
            { "alias": "main", "url": "https://a.invalid", "default_registry": true }
        ]);
    });
    std::fs::create_dir_all(project.home()).unwrap();
    let store = br#"{"registries":{"https://a.invalid":{"token":"ta"}}}"#;
    std::fs::write(project.home().join("credentials.json"), store).unwrap();
    // The home is a repository's working tree, and its `HEAD` holds the store: it came with a clone.
    let mut git = FakeGit::repo(project.home(), "abc", Vec::new());
    git.blobs = vec![("credentials.json".into(), store.to_vec())];
    let user = User::at(project.home(), None).tracked_by(std::sync::Arc::new(git));
    let client = MemoryClient::new();
    let registry = ConfiguredRegistry::for_project(&project.project_root(), "add", &crate::ports())
        .with_client(client.clone())
        .as_user(user.clone());

    let error = registry
        .versions(&PackageName::parse("@a/x").unwrap())
        .expect_err("a tracked store is refused");

    assert_eq!(error.code, "R012", "{error:?}");
    assert!(error.message.contains("tracked"), "{error:?}");
    assert!(client.calls().is_empty(), "{:?}", client.calls());
    assert_eq!(user.credential_health().failures, 1);
}

// ── login and logout ──────────────────────────────────────────────────────────────────────────────

/// A project with one registry, alias `main`, at `https://main.invalid`; a client accepting `t`.
fn project_on_main() -> (TestProject, MemoryClient) {
    let project = TestProject::new().config(|c| {
        c["name"] = "p".into();
        c["registries"] = serde_json::json!([
            { "alias": "main", "url": "https://main.invalid", "default_registry": true }
        ]);
    });
    (project, MemoryClient::new().accepting("t"))
}

fn registry_for(
    project: &TestProject,
    client: &MemoryClient,
    operation: &str,
) -> ConfiguredRegistry {
    ConfiguredRegistry::for_project(&project.project_root(), operation, &crate::ports())
        .with_client(client.clone())
        .as_user(User::at(project.home(), None))
}

fn root(text: &str) -> RegistryRoot {
    RegistryRoot::parse(text).unwrap()
}

fn store_of(project: &TestProject) -> serde_json::Value {
    let text = std::fs::read_to_string(project.home().join("credentials.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn kept_roots(project: &TestProject) -> Vec<String> {
    let mut roots: Vec<String> = store_of(project)["registries"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    roots.sort();
    roots
}

#[specforge_test(requirement = "FEATURE-110")]
fn login_keeps_the_credential_under_the_registrys_root() {
    let (project, client) = project_on_main();

    let logged_in = registry_for(&project, &client, "login")
        .log_in(None, TokenSource::Given("t".into()))
        .unwrap();

    assert_eq!(logged_in.alias, "main");
    assert_eq!(logged_in.root, root("https://main.invalid"));
    assert_eq!(kept_roots(&project), ["https://main.invalid"]);
}

#[specforge_test(requirement = "FEATURE-110")]
fn login_with_a_token_keeps_the_secret_out_of_the_store() {
    let (project, client) = project_on_main();

    let logged_in = registry_for(&project, &client, "login")
        .log_in(Some("main"), TokenSource::Given("t".into()))
        .unwrap();

    assert_eq!(logged_in.kept, Kept::Secret);
    let store = std::fs::read_to_string(project.home().join("credentials.json")).unwrap();
    assert!(!store.contains("\"t\""), "{store}");
    let secrets: Vec<_> = std::fs::read_dir(project.home().join("secrets"))
        .unwrap()
        .collect();
    assert_eq!(secrets.len(), 1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = secrets[0]
            .as_ref()
            .unwrap()
            .metadata()
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    // And the kept secret is what a read then carries.
    let read = registry_for(&project, &client, "add");
    let _ = read.versions(&PackageName::parse("@acme/x").unwrap());
    let sent = client.calls().last().unwrap().credential.clone().unwrap();
    assert_eq!(sent.token(), "t");
}

#[specforge_test(requirement = "FEATURE-110")]
fn login_refuses_an_unreadable_store_before_any_request_and_keeps_it() {
    let (project, client) = project_on_main();
    std::fs::create_dir_all(project.home()).unwrap();
    let broken = r#"{"registries": {},}"#;
    std::fs::write(project.home().join("credentials.json"), broken).unwrap();

    let error = registry_for(&project, &client, "login")
        .log_in(None, TokenSource::Given("t".into()))
        .unwrap_err();

    assert_eq!(error.code, "R012", "{error:?}");
    assert!(client.calls().is_empty());
    assert_eq!(
        std::fs::read_to_string(project.home().join("credentials.json")).unwrap(),
        broken
    );
}

#[specforge_test(requirement = "FEATURE-110")]
fn a_token_the_registry_refuses_writes_nothing() {
    let (project, client) = project_on_main();

    let error = registry_for(&project, &client, "login")
        .log_in(None, TokenSource::Given("x".into()))
        .unwrap_err();

    assert_eq!(error.code, "R001", "{error:?}");
    assert!(!project.home().join("credentials.json").exists());
}

#[specforge_test(requirement = "FEATURE-110")]
fn two_entries_with_one_alias_keep_two_credentials() {
    let (project, client) = project_on_main();
    registry_for(&project, &client, "login")
        .log_in(Some("main"), TokenSource::Given("t".into()))
        .unwrap();
    // Another project, with another root under the same alias, for the same user.
    let other = TestProject::new().config(|c| {
        c["name"] = "q".into();
        c["registries"] = serde_json::json!([
            { "alias": "main", "url": "https://other.invalid", "default_registry": true }
        ]);
    });
    ConfiguredRegistry::for_project(&other.project_root(), "login", &crate::ports())
        .with_client(client.clone())
        .as_user(User::at(project.home(), None))
        .log_in(Some("main"), TokenSource::Given("t".into()))
        .unwrap();

    assert_eq!(
        kept_roots(&project),
        ["https://main.invalid", "https://other.invalid"]
    );
}

#[specforge_test(requirement = "FEATURE-110")]
fn logout_forgets_only_its_roots_credential() {
    let (project, client) = project_on_main();
    registry_for(&project, &client, "login")
        .log_in(None, TokenSource::Given("t".into()))
        .unwrap();
    // A second root's credential, kept beside it.
    let mut store = store_of(&project);
    store["registries"]["https://other.invalid"] = serde_json::json!({"token_env": "R20_OTHER"});
    std::fs::write(project.home().join("credentials.json"), store.to_string()).unwrap();

    let out = User::at(project.home(), None)
        .log_out(&root("https://main.invalid"))
        .unwrap();

    assert!(out.removed);
    assert_eq!(kept_roots(&project), ["https://other.invalid"]);
    let left = std::fs::read_dir(project.home().join("secrets")).unwrap();
    assert_eq!(left.count(), 0, "its secret file is gone");
}

#[specforge_test(requirement = "FEATURE-110")]
fn logout_of_a_root_with_nothing_kept_succeeds_silently() {
    let (project, _) = project_on_main();

    let out = User::at(project.home(), None)
        .log_out(&root("https://main.invalid"))
        .unwrap();

    assert!(!out.removed);
    assert!(!project.home().join("credentials.json").exists());
}

// ── no user home ──────────────────────────────────────────────────────────────────────────────────

/// The user directory's refusal for `file`: its cause and its remedy.
fn assert_no_home(error: &specforge_ops::OpError, code: &str, file: &str) {
    assert_eq!(error.code, code, "{error:?}");
    assert_eq!(
        error.message,
        format!(
            "no user directory to keep {file} in: HOME (USERPROFILE on Windows) is unset or relative"
        ),
        "{error:?}"
    );
    assert_eq!(
        error.suggestion.as_deref(),
        Some("set HOME (USERPROFILE on Windows) to an absolute path"),
        "{error:?}"
    );
}

#[specforge_test(requirement = "FEATURE-110")]
fn a_user_with_no_home_refuses_every_registry_file_and_writes_nothing() {
    use specforge_common::home::home_from;
    use specforge_ops::registry::KeyPin;
    use specforge_ops::registry::testing::declaration;
    use specforge_package::DeclaredPackage;
    // A relative home is no home (ADR 0375 D5): it used to put the files under `./.specforge` of the directory a run
    // starts in. That directory here is an empty one of the test's own (nextest runs each test in its own process).
    let cwd = tempfile::TempDir::new().unwrap();
    let before = std::env::current_dir().unwrap();
    std::env::set_current_dir(cwd.path()).unwrap();
    let home = home_from(Some(".".into()), None);
    assert_eq!(home, None);
    let user = User::from_home(home, Some("t".into()));
    let (project, client) = project_on_main();
    let registry = |operation: &str| {
        ConfiguredRegistry::for_project(&project.project_root(), operation, &crate::ports())
            .with_client(client.clone())
            .as_user(user.clone())
    };
    let name = PackageName::parse("@a/x").unwrap();

    let read = registry("add").versions(&name).unwrap_err();
    let pinned = registry("add")
        .pin(&[KeyPin {
            name: name.clone(),
            key_id: "k".into(),
            replaces: None,
        }])
        .unwrap_err();
    let declared = declaration("@a/x", "1.0.0", &[]);
    let published = registry("publish")
        .publish(&DeclaredPackage::of(&declared).unwrap(), b"\0asm")
        .unwrap_err();
    let logged_in = registry("login")
        .log_in(None, TokenSource::Given("t".into()))
        .unwrap_err();
    let logged_out = user.log_out(&root("https://main.invalid")).unwrap_err();
    let health = User::from_home(None, None).credential_health();

    let left: Vec<_> = std::fs::read_dir(cwd.path()).unwrap().collect();
    std::env::set_current_dir(before).unwrap();
    assert_no_home(&read, "R012", "credentials.json");
    assert_no_home(&pinned, "R-TRUST-006", "known-keys.json");
    // The environment's token stands in for the store: the publisher key is the file refused.
    assert_no_home(&published, "E074", "signing-key.json");
    assert_no_home(&logged_in, "R012", "credentials.json");
    assert_no_home(&logged_out, "R012", "credentials.json");
    assert_eq!(health.failures, 1);
    assert!(!health.signing_key_present);
    assert_eq!(
        health
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        [
            "no user directory to keep credentials.json in: HOME (USERPROFILE on Windows) is unset or relative — set HOME (USERPROFILE on Windows) to an absolute path"
        ]
    );
    assert!(client.calls().is_empty(), "{:?}", client.calls());
    assert!(
        left.is_empty(),
        "written in the directory the run started in: {left:?}"
    );
}
