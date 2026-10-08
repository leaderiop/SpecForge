//! What `HttpRegistry` asks a registry for: the request path of every call,
//! recorded by an in-process server that answers 404 to everything but one
//! package's version list.
//!
//! Plan 12 §3 R1, R4 and R6.

use specforge_ops::extension::{Trust, resolve};
use specforge_ops::registry::Registry;
use specforge_ops_registry::HttpRegistry;
use specforge_protocol_types::package::{PackageName, PackageRef, Version};
use specforge_test_macros::test as specforge_test;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

/// A registry on a local port, and every request line it has been sent.
struct Recording {
    /// The base URL (`http://127.0.0.1:<port>/v1`).
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl Recording {
    /// Serves `/v1/packages/@acme%2Ftool` as a package published at
    /// `versions`; everything else is 404.
    fn serving(versions: &[&str]) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&requests);
        let list = serde_json::json!({ "versions": versions }).to_string();
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
                let path = request_line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .to_string();
                let (status, body) = if path == "/v1/packages/@acme%2Ftool" {
                    ("200 OK", list.clone())
                } else {
                    (
                        "404 Not Found",
                        r#"{"error":{"code":"NOT_FOUND","message":"nope"}}"#.to_string(),
                    )
                };
                log.lock().unwrap().push(path);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        Recording { url, requests }
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

/// A project whose `registries` is `registry`, as `entry` writes it.
fn project_with(registry: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let config = format!(
        r#"{{"name":"p","version":"0.1.0","spec_root":"spec","extensions":[],"registries":[{registry}]}}"#
    );
    std::fs::write(dir.path().join("specforge.json"), config).unwrap();
    dir
}

fn default_registry(url: &str) -> String {
    format!(r#"{{"alias":"main","url":"{url}","default_registry":true}}"#)
}

fn name(text: &str) -> PackageName {
    PackageName::parse(text).unwrap()
}

#[specforge_test(
    behavior = "resolve_registry_source",
    verify = "a fetch requests the name and version it was given, from the registry it was given"
)]
fn the_adapter_requests_these_paths() {
    let served = Recording::serving(&["1.4.0", "2.0.0-beta.1"]);
    let dir = project_with(&default_registry(&served.url));
    let registry = HttpRegistry::for_project(dir.path(), "add");

    // The registry is asked for the versions, and ops picks among them:
    // `1.x`, `1.2` and `latest` are no longer fetched as versions (§3 R1,
    // R6).
    for (reference, want) in [
        ("@acme/tool@1.x", "1.4.0"),
        ("@acme/tool@1.2", "1.4.0"),
        ("@acme/tool", "1.4.0"),
        ("@acme/tool@>=2.0.0-beta.1", "2.0.0-beta.1"),
    ] {
        let version = resolve(&registry, &PackageRef::parse(reference).unwrap()).unwrap();
        assert_eq!(version.to_string(), want, "{reference}");
        assert_eq!(
            served.requests().last().map(String::as_str),
            Some("/v1/packages/@acme%2Ftool"),
            "{reference}"
        );
    }

    // An exact version asks the registry for nothing.
    let before = served.requests().len();
    let exact = resolve(&registry, &PackageRef::parse("@acme/tool@9.9.9").unwrap()).unwrap();
    assert_eq!(exact.to_string(), "9.9.9");
    assert_eq!(served.requests().len(), before);

    // A fetch requests the name and the version it was given: a version
    // cannot carry a `/` or a `?` into the URL, and `2.0.0+build.1` is one.
    let _ = registry.fetch(
        &name("@acme/tool"),
        &Version::new(1, 0, 0),
        true,
        Trust::Refuse,
    );
    assert_eq!(
        served.requests().last().map(String::as_str),
        Some("/v1/packages/@acme%2Ftool/1.0.0")
    );
    let build = "2.0.0+build.1".parse().unwrap();
    let _ = registry.fetch(&name("@acme/tool"), &build, true, Trust::Refuse);
    assert_eq!(
        served.requests().last().map(String::as_str),
        Some("/v1/packages/@acme%2Ftool/2.0.0+build.1")
    );
}

// Pins a bug: a name no registry serves is asked of the first entry, which
// here is scoped to another owner (plan 06 §3 R1). T1 flips it.
#[test]
fn the_adapter_asks_a_registry_scoped_to_another_name_today() {
    let served = Recording::serving(&["1.0.0"]);
    let dir = project_with(&format!(
        r#"{{"alias":"acme","url":"{}","scope_filter":"@acme"}}"#,
        served.url
    ));
    let registry = HttpRegistry::for_project(dir.path(), "add");

    let error = registry.versions(&name("@other/x")).unwrap_err();

    assert_eq!(error.code, "R-RES-001", "{error:?}");
    assert_eq!(served.requests(), ["/v1/packages/@other%2Fx"]);
}

#[specforge_test(
    behavior = "resolve_registry_source",
    verify = "a fetch requests the name and version it was given, from the registry it was given"
)]
fn the_registry_is_chosen_once() {
    // One registry with no default and no scope filter: the adapter falls
    // back to the first entry, and the client fetches from it (§3 R4).
    let served = Recording::serving(&["1.0.0"]);
    let dir = project_with(&format!(r#"{{"alias":"main","url":"{}"}}"#, served.url));
    let registry = HttpRegistry::for_project(dir.path(), "add");

    assert_eq!(
        registry.versions(&name("@acme/tool")).unwrap(),
        [Version::new(1, 0, 0)]
    );
    assert_eq!(served.requests(), ["/v1/packages/@acme%2Ftool"]);

    // The download goes to the same registry: a request, answered 404.
    let error = registry
        .fetch(
            &name("@acme/tool"),
            &Version::new(1, 0, 0),
            true,
            Trust::Refuse,
        )
        .unwrap_err();
    assert_eq!(error.code, "R006", "{error:?}");
    assert_eq!(
        served.requests(),
        [
            "/v1/packages/@acme%2Ftool",
            "/v1/packages/@acme%2Ftool/1.0.0"
        ]
    );
}
