//! `HttpRegistry::fetch` against a local registry: it refuses a tampered
//! download, a broken or missing signature and a re-keyed publisher, and
//! pins the key of a correctly signed package (docs/registry-trust.md).
//!
//! Each test serves one package from an in-process HTTP server and pins
//! keys in a temporary store, never in `~/.specforge`.

use sha2::{Digest, Sha256};
use specforge_ops::extension::Trust;
use specforge_ops::registry::Registry;
use specforge_ops_registry::HttpRegistry;
use specforge_registry_client::{KnownKeys, SigningKey, load_known_keys_at, save_known_keys_at};
use specforge_test_macros::test as specforge_test;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const NAME: &str = "@acme/tool";
const VERSION: &str = "1.0.0";
const WASM: &[u8] = b"\0asm-acme-tool";
const MANIFEST: &str = r#"{"name":"@acme/tool","version":"1.0.0","manifestVersion":2}"#;

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// What the registry serves for `@acme/tool@1.0.0`.
struct Served {
    name: String,
    /// The metadata's `sha256`.
    sha256: String,
    /// The downloaded bytes.
    wasm: Vec<u8>,
    manifest: String,
    /// The wire signature object, empty when unsigned.
    signature: String,
    key_id: String,
}

impl Served {
    fn unsigned() -> Self {
        Served {
            name: NAME.to_string(),
            sha256: sha256(WASM),
            wasm: WASM.to_vec(),
            manifest: MANIFEST.to_string(),
            signature: String::new(),
            key_id: String::new(),
        }
    }

    /// Signed by `key` over `name`, `wasm` and `manifest`.
    fn signed_over(key: &SigningKey, name: &str, wasm: &[u8], manifest: &str) -> Self {
        let signature = key.sign_package(
            name,
            VERSION,
            &sha256(wasm),
            &sha256(manifest.as_bytes()),
            "2026-10-03T00:00:00+00:00",
        );
        Served {
            name: name.to_string(),
            signature: serde_json::to_string(&signature).unwrap(),
            key_id: signature.key_id,
            ..Served::unsigned()
        }
    }

    fn signed(key: &SigningKey) -> Self {
        Served::signed_over(key, NAME, WASM, MANIFEST)
    }

    /// Serve it on a local port until the test process exits; the
    /// registry base URL (`.../v1`).
    fn serve(self) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let metadata = serde_json::json!({
            "name": self.name,
            "version": VERSION,
            "sha256": self.sha256,
            "wasm_url": "/wasm/acme-tool/1.0.0",
            "manifest": self.manifest,
            "signature": self.signature,
            "key_id": self.key_id,
        })
        .to_string()
        .into_bytes();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request_line = String::new();
                let _ = reader.read_line(&mut request_line);
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                }
                let path = request_line.split_whitespace().nth(1).unwrap_or("/");
                let body = if path.starts_with("/v1/wasm/") {
                    &self.wasm
                } else {
                    &metadata
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(body);
            }
        });
        url
    }
}

/// A project whose only registry serves `served`, and a known-keys store
/// path inside it.
struct Project {
    dir: TempDir,
}

impl Project {
    fn on(served: Served) -> Self {
        let url = served.serve();
        let dir = TempDir::new().unwrap();
        let config = serde_json::json!({
            "name": "p",
            "version": "0.1.0",
            "registries": [{ "alias": "local", "url": url, "default_registry": true }],
        });
        std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
        Project { dir }
    }

    fn known_keys(&self) -> PathBuf {
        self.dir.path().join("home").join("known-keys.json")
    }

    fn pin(&self, key: &SigningKey) {
        let mut known = KnownKeys::default();
        known.pin(NAME, &key.key_id());
        save_known_keys_at(&self.known_keys(), &known).unwrap();
    }

    fn registry(&self) -> HttpRegistry {
        HttpRegistry::for_project(self.dir.path(), "add").with_known_keys(self.known_keys())
    }

    fn fetch(
        &self,
        allow_unsigned: bool,
        trust: Trust,
    ) -> Result<specforge_ops::registry::Package, specforge_ops::OpError> {
        self.registry().fetch(NAME, VERSION, allow_unsigned, trust)
    }

    fn pinned(&self) -> Option<String> {
        pinned_at(&self.known_keys())
    }
}

fn pinned_at(path: &Path) -> Option<String> {
    load_known_keys_at(path).pin_for(NAME).map(str::to_string)
}

#[specforge_test(
    behavior = "verify_registry_integrity",
    verify = "mismatched SHA256 produces hard error"
)]
fn a_download_that_does_not_match_the_registry_sha256_is_refused() {
    let key = SigningKey::generate();
    let served = Served {
        wasm: b"\0asm-evil".to_vec(),
        ..Served::signed(&key)
    };
    let project = Project::on(served);

    // Neither --allow-unsigned nor --yes bypasses integrity.
    let error = project.fetch(true, Trust::AssumeYes).unwrap_err();
    assert_eq!(error.code, "R-OPS-002", "{error:?}");
    assert_eq!(
        project.pinned(),
        None,
        "nothing is pinned for a refused package"
    );
}

#[specforge_test(
    invariant = "registry_integrity",
    verify = "SHA256 mismatch produces hard error and aborts"
)]
fn a_registry_sha256_that_does_not_match_the_download_is_refused() {
    let served = Served {
        sha256: sha256(b"something else"),
        ..Served::unsigned()
    };
    let project = Project::on(served);

    let error = project.fetch(true, Trust::AssumeYes).unwrap_err();
    assert_eq!(error.code, "R-OPS-002", "{error:?}");
}

#[test]
fn a_signature_over_other_bytes_is_refused_even_with_allow_unsigned() {
    // The registry serves a consistent sha256 for the bytes it sends, but
    // the publisher signed different ones: integrity passes, the signature
    // must not.
    let key = SigningKey::generate();
    let served = Served {
        sha256: sha256(b"\0asm-evil"),
        wasm: b"\0asm-evil".to_vec(),
        ..Served::signed(&key)
    };
    let project = Project::on(served);

    let error = project.fetch(true, Trust::AssumeYes).unwrap_err();
    assert_eq!(error.code, "R-TRUST-002", "{error:?}");
    assert_eq!(project.pinned(), None);
}

#[test]
fn a_swapped_manifest_breaks_the_signature() {
    let key = SigningKey::generate();
    let served = Served {
        manifest: r#"{"name":"@acme/tool","version":"1.0.0","manifestVersion":3}"#.to_string(),
        ..Served::signed(&key)
    };
    let project = Project::on(served);

    let error = project.fetch(true, Trust::AssumeYes).unwrap_err();
    assert_eq!(error.code, "R-TRUST-002", "{error:?}");
}

#[test]
fn a_signature_object_that_is_not_json_is_refused() {
    let served = Served {
        signature: "not a signature".to_string(),
        ..Served::unsigned()
    };
    let project = Project::on(served);

    let error = project.fetch(true, Trust::AssumeYes).unwrap_err();
    assert_eq!(error.code, "R-TRUST-002", "{error:?}");
}

#[test]
fn a_key_id_the_signature_does_not_carry_is_refused() {
    let key = SigningKey::generate();
    let served = Served {
        key_id: SigningKey::generate().key_id(),
        ..Served::signed(&key)
    };
    let project = Project::on(served);

    let error = project.fetch(true, Trust::AssumeYes).unwrap_err();
    assert_eq!(error.code, "R-TRUST-004", "{error:?}");
}

#[test]
fn an_unsigned_package_is_refused_without_allow_unsigned() {
    let project = Project::on(Served::unsigned());

    let error = project.fetch(false, Trust::AssumeYes).unwrap_err();
    assert_eq!(error.code, "R-TRUST-001", "{error:?}");
    assert!(
        error
            .suggestion
            .as_deref()
            .unwrap_or_default()
            .contains("--allow-unsigned"),
        "{error:?}"
    );
}

#[test]
fn an_unsigned_package_is_accepted_with_allow_unsigned_and_pins_nothing() {
    let project = Project::on(Served::unsigned());

    let package = project.fetch(true, Trust::Refuse).unwrap();
    assert_eq!(package.wasm, WASM);
    assert_eq!(package.key_id, None);
    assert_eq!(project.pinned(), None);
}

#[test]
fn a_correctly_signed_package_is_accepted_and_its_key_pinned() {
    let key = SigningKey::generate();
    let project = Project::on(Served::signed(&key));

    let package = project.fetch(false, Trust::Refuse).unwrap();
    assert_eq!(package.name, NAME);
    assert_eq!(package.version, VERSION);
    assert_eq!(package.wasm, WASM);
    assert_eq!(package.sha256, sha256(WASM));
    assert_eq!(package.key_id, Some(key.key_id()));
    assert_eq!(project.pinned(), Some(key.key_id()));

    // A second install under the same pin is accepted as is.
    let again = project.fetch(false, Trust::Refuse).unwrap();
    assert_eq!(again.key_id, Some(key.key_id()));
    assert_eq!(project.pinned(), Some(key.key_id()));
}

#[test]
fn a_package_signed_by_another_key_than_the_pinned_one_is_refused() {
    let pinned = SigningKey::generate();
    let other = SigningKey::generate();
    let project = Project::on(Served::signed(&other));
    project.pin(&pinned);

    let error = project.fetch(false, Trust::Refuse).unwrap_err();
    assert_eq!(error.code, "R-TRUST-003", "{error:?}");
    assert!(error.message.contains(&pinned.key_id()), "{error:?}");
    assert!(error.message.contains(&other.key_id()), "{error:?}");
    // --allow-unsigned is not consent to a key change either.
    let error = project.fetch(true, Trust::Refuse).unwrap_err();
    assert_eq!(error.code, "R-TRUST-003", "{error:?}");
    assert_eq!(project.pinned(), Some(pinned.key_id()), "the pin stands");
}

#[test]
fn consent_to_a_key_change_re_pins_the_new_key() {
    let pinned = SigningKey::generate();
    let other = SigningKey::generate();
    let project = Project::on(Served::signed(&other));
    project.pin(&pinned);

    let package = project.fetch(false, Trust::AssumeYes).unwrap();
    assert_eq!(package.key_id, Some(other.key_id()));
    assert_eq!(project.pinned(), Some(other.key_id()));
}

#[test]
fn a_denied_key_is_refused_even_with_consent() {
    let key = SigningKey::generate();
    let project = Project::on(Served::signed(&key));
    let mut known = KnownKeys::default();
    known.denied_keys.push(key.key_id());
    save_known_keys_at(&project.known_keys(), &known).unwrap();

    let error = project.fetch(true, Trust::AssumeYes).unwrap_err();
    assert_eq!(error.code, "R-TRUST-005", "{error:?}");
}

#[test]
fn a_registry_answering_with_another_package_is_refused() {
    // `@acme/tool` is pinned; a registry that answers the request with a
    // validly signed `@evil/tool` must not get it past the pin.
    let pinned = SigningKey::generate();
    let evil = SigningKey::generate();
    let project = Project::on(Served::signed_over(&evil, "@evil/tool", WASM, MANIFEST));
    project.pin(&pinned);

    let error = project.fetch(false, Trust::AssumeYes).unwrap_err();
    assert_eq!(error.code, "R-TRUST-004", "{error:?}");
    assert!(error.message.contains("@evil/tool"), "{error:?}");
    assert_eq!(project.pinned(), Some(pinned.key_id()));
    let known = load_known_keys_at(&project.known_keys());
    assert_eq!(
        known.pin_for("@evil/tool"),
        None,
        "nothing is pinned for it"
    );
}
