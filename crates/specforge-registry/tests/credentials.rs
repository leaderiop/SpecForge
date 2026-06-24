use specforge_registry::client::credentials::{
    read_credentials, write_credentials, CredentialStore,
};
use tempfile::TempDir;

#[test]
fn roundtrip_credentials() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("credentials.json");

    let mut store = CredentialStore::default();
    store.set_token("default", "sfr_test_token_123".to_string());
    store.set_token("private", "priv_token_456".to_string());

    write_credentials(&path, &store).unwrap();
    let loaded = read_credentials(&path).unwrap();

    assert_eq!(loaded.registries.len(), 2);
    assert!(loaded.registries.contains_key("default"));
    assert!(loaded.registries.contains_key("private"));
}

#[test]
fn read_nonexistent_returns_empty() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nonexistent.json");
    let store = read_credentials(&path).unwrap();
    assert!(store.registries.is_empty());
}

#[test]
fn get_credential_returns_bearer() {
    let mut store = CredentialStore::default();
    store.set_token("myregistry", "my_token".to_string());

    let cred = store.get_credential("myregistry").unwrap();
    assert_eq!(cred.alias, "myregistry");
    match cred.auth_method {
        specforge_registry::AuthMethod::Bearer(t) => assert_eq!(t, "my_token"),
        _ => panic!("expected Bearer"),
    }
}

#[test]
fn remove_credential() {
    let mut store = CredentialStore::default();
    store.set_token("temp", "token".to_string());
    assert!(store.remove("temp"));
    assert!(!store.remove("temp"));
    assert!(store.registries.is_empty());
}
