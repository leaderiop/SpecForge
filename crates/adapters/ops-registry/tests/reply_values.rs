//! A registry's reply is read as values where it is read (ADR 0265): a package name, a SemVer version, a
//! lowercase SHA-256 and an RFC 3339 expiry, else R009 before anything is downloaded or kept.
//!
//! The registry here is a socket answering fixed JSON, so a value the real server never writes can be served.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use specforge_ops::asking::Asking;
use specforge_ops::extension::resolve;
use specforge_ops::registry::{Registry, TrustPolicy};
use specforge_ops_registry::{ConfiguredRegistry, TokenSource, User};
use specforge_package::{PackageName, PackageRef, Version};
use specforge_project::testing::TestProject;
use specforge_test_macros::test as specforge_test;

/// The declaration of `@acme/tool@1.0.0`, as `specforge publish` uploads it.
const MANIFEST: &str = r#"{"handshake":{"protocol_version":"1.0.0","name":"@acme/tool","version":"1.0.0","peer_dependencies":[],"sandbox_policy":null}}"#;
const WASM: &[u8] = b"\0asm-acme-tool";
const LIST: &str = "/v1/packages/@acme%2Ftool";
const METADATA: &str = "/v1/packages/@acme%2Ftool/1.0.0";
const DOWNLOAD: &str = "/v1/packages/@acme%2Ftool/1.0.0/download";
const VERIFY: &str = "/v1/auth/verify";

/// A registry on a local port answering each path with its JSON (a `200`), any other with the protocol's 404.
struct Fake {
    root: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl Fake {
    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }

    fn downloaded(&self) -> bool {
        self.requests().iter().any(|r| r.ends_with("/download"))
    }
}

fn serve(answers: Vec<(&'static str, String)>) -> Fake {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let root = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&requests);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).is_ok_and(|n| n == 1) {
                head.push(byte[0]);
            }
            let head = String::from_utf8_lossy(&head);
            // Read the request body too: answering a POST unread resets the connection.
            let length = head
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")?
                        .trim()
                        .parse()
                        .ok()
                })
                .unwrap_or(0);
            let _ = stream.read_exact(&mut vec![0u8; length]);
            let mut words = head.split_whitespace();
            let (method, path) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
            seen.lock().unwrap().push(format!("{method} {path}"));
            let (status, body) = match answers.iter().find(|(key, _)| *key == path) {
                Some((_, body)) => ("200 OK", body.clone().into_bytes()),
                None => (
                    "404 Not Found",
                    br#"{"error":{"code":"NOT_FOUND","message":"no"}}"#.to_vec(),
                ),
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(&body);
        }
    });
    Fake { root, requests }
}

fn digest() -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(WASM))
}

/// The metadata reply of `@acme/tool@1.0.0` with `key` set to `value`.
fn metadata(key: &str, value: serde_json::Value) -> String {
    let mut reply = serde_json::json!({
        "name": "@acme/tool", "version": "1.0.0", "sha256": digest(),
        "size_bytes": WASM.len(), "wasm_url": DOWNLOAD, "manifest": MANIFEST,
    });
    reply[key] = value;
    reply.to_string()
}

fn project(fake: &Fake) -> (TestProject, ConfiguredRegistry) {
    let dir = TestProject::new().raw_config(format!(
        r#"{{"name":"p","version":"0.1.0","spec_root":"spec","extensions":[],"registries":[{{"alias":"main","url":"{}","default_registry":true}}]}}"#,
        fake.root
    ));
    let registry = ConfiguredRegistry::for_project(&dir.project_root(), "add")
        .as_user(User::at(dir.home(), None));
    (dir, registry)
}

fn name() -> PackageName {
    PackageName::parse("@acme/tool").unwrap()
}

fn policy() -> TrustPolicy<'static> {
    TrustPolicy::new(true, Asking::Yes)
}

#[specforge_test(
    behavior = "check_registry_reply",
    verify = "a reply whose sha256 is not 64 lowercase hex characters is refused with R009 before its binary is downloaded, and its dry run says so"
)]
fn a_digest_that_is_no_sha256_is_refused_before_any_download() {
    for bad in [
        digest().to_uppercase(),
        "nope".to_string(),
        digest()[1..].to_string(),
    ] {
        let fake = serve(vec![
            (METADATA, metadata("sha256", serde_json::json!(bad))),
            (DOWNLOAD, String::from_utf8_lossy(WASM).into_owned()),
        ]);
        let (_dir, registry) = project(&fake);
        let version = Version::new(1, 0, 0);

        let dry = registry.published(&name(), &version, policy()).unwrap_err();
        assert_eq!(dry.code, "R009", "{bad}: {dry:?}");
        let fetched = registry.fetch(&name(), &version, policy()).unwrap_err();
        assert_eq!(fetched.code, "R009", "{bad}: {fetched:?}");
        assert!(!fake.downloaded(), "{bad}: {:?}", fake.requests());
    }
}

#[specforge_test(
    behavior = "check_registry_reply",
    verify = "a reply whose name is no package name or whose version is no SemVer version is refused with R009, not as another package"
)]
fn a_name_or_version_that_is_not_one_is_r009_not_another_package() {
    for (key, bad) in [
        ("name", "@acme/tool\u{1b}[31m"),
        ("version", "1.0.0\u{1b}[2J"),
    ] {
        let fake = serve(vec![(METADATA, metadata(key, serde_json::json!(bad)))]);
        let (_dir, registry) = project(&fake);

        let error = registry
            .published(&name(), &Version::new(1, 0, 0), policy())
            .unwrap_err();

        assert_eq!(error.code, "R009", "{key}: {error:?}");
        assert!(!error.message.chars().any(char::is_control), "{error:?}");
    }
}

#[specforge_test(
    behavior = "resolve_registry_source",
    verify = "a version list holding a version that is no SemVer version is refused with R009, never read as fewer versions"
)]
fn a_version_list_holding_no_semver_version_is_r009_not_fewer_versions() {
    for versions in [
        serde_json::json!(["1.0.0", "not-a-version"]),
        serde_json::json!(["v1", "latest"]),
    ] {
        let fake = serve(vec![(
            LIST,
            serde_json::json!({"name": "@acme/tool", "versions": versions}).to_string(),
        )]);
        let (_dir, registry) = project(&fake);

        let error = resolve(&registry, &PackageRef::parse("@acme/tool").unwrap()).unwrap_err();

        assert_eq!(error.code, "R009", "{versions}: {error:?}");
    }
}

#[specforge_test(
    behavior = "authenticate_registry_request",
    verify = "a token check whose expiry is not an RFC 3339 timestamp is refused with R009 and login keeps nothing"
)]
#[ignore = "flipped by T5"]
fn a_token_expiry_that_is_no_timestamp_is_r009_and_login_keeps_nothing() {
    let fake = serve(vec![(
        VERIFY,
        serde_json::json!({
            "valid": true, "scope": null, "label": "x",
            "expires_at": "soon\u{1b}]0;PWNED\u{7}\u{1b}[31mRED\u{202e}",
        })
        .to_string(),
    )]);
    let (dir, registry) = project(&fake);

    let error = registry
        .log_in(None, TokenSource::Given("t".into()))
        .unwrap_err();

    assert_eq!(error.code, "R009", "{error:?}");
    assert!(!error.message.chars().any(char::is_control), "{error:?}");
    assert!(!dir.home().join("credentials.json").exists());
}
