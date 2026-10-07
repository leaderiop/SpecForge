//! What `HttpRegistry` asks a registry for: the request path of every call,
//! recorded by an in-process server that answers 404 to everything but one
//! package's version list.
//!
//! Plan 12 §3 R1, R4 and R6 as they are today; T3 and T4 flip the rows that
//! encode a bug.

use specforge_ops::extension::Trust;
use specforge_ops::registry::Registry;
use specforge_ops_registry::HttpRegistry;
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

#[specforge_test(
    behavior = "resolve_registry_source",
    verify = "a fetch requests the name and version it was given, from the registry it was given"
)]
fn the_adapter_requests_these_paths_today() {
    let served = Recording::serving(&["1.4.0", "2.0.0-beta.1"]);
    let dir = project_with(&default_registry(&served.url));
    let registry = HttpRegistry::for_project(dir.path(), "add");

    // bug (§3 R1): a version that is no version reaches the URL as it is.
    let _ = registry.fetch("@acme/tool", "1.0.0/x", true, Trust::Refuse);
    assert_eq!(
        served.requests(),
        ["/v1/packages/@acme%2Ftool/1.0.0/x"],
        "the version is not escaped"
    );

    // bug (§3 R1): ops asked for `foo` at `/bar`; the client asks for the
    // package `foo@/bar` at `latest`.
    let _ = registry.fetch("foo", "/bar", true, Trust::Refuse);
    assert_eq!(
        served.requests()[1..],
        ["/v1/packages/foo@%2Fbar/latest"],
        "the client reads the string again"
    );

    // bug (§3 R1): a requirement that is not an operator-led range is an
    // exact version: it is returned as it is, and nothing is requested.
    let before = served.requests().len();
    assert_eq!(
        registry.resolve_version("@acme/tool", "1.x").unwrap(),
        "1.x"
    );
    assert_eq!(
        registry.resolve_version("@acme/tool", "1.2").unwrap(),
        "1.2"
    );
    assert_eq!(served.requests().len(), before);

    // bug (§3 R6): `latest` is the highest version, a pre-release included.
    assert_eq!(
        registry.resolve_version("@acme/tool", "latest").unwrap(),
        "2.0.0-beta.1"
    );
    assert_eq!(
        served.requests().last().map(String::as_str),
        Some("/v1/packages/@acme%2Ftool")
    );
}

#[specforge_test(
    behavior = "resolve_registry_source",
    verify = "a fetch requests the name and version it was given, from the registry it was given"
)]
fn the_registry_is_chosen_twice_today() {
    // One registry with no default and no scope filter: the adapter falls
    // back to the first entry, the client does not.
    let served = Recording::serving(&["1.0.0"]);
    let dir = project_with(&format!(r#"{{"alias":"main","url":"{}"}}"#, served.url));
    let registry = HttpRegistry::for_project(dir.path(), "add");

    assert_eq!(
        registry.resolve_version("@acme/tool", "latest").unwrap(),
        "1.0.0"
    );
    assert_eq!(served.requests(), ["/v1/packages/@acme%2Ftool"]);

    // bug (§3 R4): the download is refused without a request.
    let error = registry
        .fetch("@acme/tool", "1.0.0", true, Trust::Refuse)
        .unwrap_err();
    assert_eq!(error.code, "R-OPS-001", "{error:?}");
    assert_eq!(served.requests().len(), 1, "no request for the download");
}
