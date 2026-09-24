use specforge_registry::client::credentials::{
    CredentialEntry, CredentialStore, read_credentials, write_credentials,
};
use tempfile::TempDir;

#[test]
fn roundtrip_credentials() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("credentials.json");

    let mut store = CredentialStore::default();
    store.registries.insert(
        "default".to_string(),
        CredentialEntry::Token {
            token: "sfr_test_token_123".to_string(),
            expires_at: None,
            in_keyring: false,
        },
    );
    store.registries.insert(
        "private".to_string(),
        CredentialEntry::Token {
            token: "priv_token_456".to_string(),
            expires_at: None,
            in_keyring: false,
        },
    );

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
    store.registries.insert(
        "myregistry".to_string(),
        CredentialEntry::Token {
            token: "my_token".to_string(),
            expires_at: None,
            in_keyring: false,
        },
    );

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
    store.registries.insert(
        "temp".to_string(),
        CredentialEntry::Token {
            token: "token".to_string(),
            expires_at: None,
            in_keyring: false,
        },
    );
    assert!(store.remove("temp"));
    assert!(!store.remove("temp"));
    assert!(store.registries.is_empty());
}
