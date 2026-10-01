//! A registry for tests: a local HTTP server that speaks the three calls
//! the registry client makes (versions, package metadata, the Wasm
//! download) from an in-memory package list, and counts its requests.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// A published package.
#[derive(Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub wasm: Vec<u8>,
    /// `(name, range)` peers its manifest declares.
    pub peers: Vec<(String, String)>,
}

impl Package {
    pub fn new(name: &str, version: &str, wasm: Vec<u8>) -> Self {
        Package {
            name: name.to_string(),
            version: version.to_string(),
            wasm,
            peers: Vec::new(),
        }
    }

    pub fn with_peer(mut self, name: &str, range: &str) -> Self {
        self.peers.push((name.to_string(), range.to_string()));
        self
    }

    fn manifest(&self) -> String {
        let peers: Vec<serde_json::Value> = self
            .peers
            .iter()
            .map(|(name, range)| serde_json::json!({"name": name, "version": range}))
            .collect();
        serde_json::json!({
            "name": self.name,
            "version": self.version,
            "manifestVersion": 2,
            "wasmPath": "extension.wasm",
            "peerDependencies": peers,
        })
        .to_string()
    }
}

pub struct FakeRegistry {
    /// The registry base URL for `specforge.json` (`.../v1`).
    pub url: String,
    hits: Arc<AtomicUsize>,
}

impl FakeRegistry {
    pub fn serve(packages: Vec<Package>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();
        let packages = Arc::new(Mutex::new(packages));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                counter.fetch_add(1, Ordering::SeqCst);
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request_line = String::new();
                let _ = reader.read_line(&mut request_line);
                // Drain the headers.
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                }
                let path = request_line.split_whitespace().nth(1).unwrap_or("/");
                let (status, body) = respond(&packages.lock().unwrap(), path);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(&body);
            }
        });
        FakeRegistry { url, hits }
    }

    /// Requests served so far.
    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }

    /// The `registries` entry naming this registry as the default.
    pub fn config_entry(&self) -> serde_json::Value {
        serde_json::json!([{ "alias": "fake", "url": self.url, "default_registry": true }])
    }
}

fn decode(name: &str) -> String {
    name.replace("%2F", "/")
        .replace("%2f", "/")
        .replace("%40", "@")
}

fn respond(packages: &[Package], path: &str) -> (&'static str, Vec<u8>) {
    let not_found = (
        "404 Not Found",
        br#"{"error":{"message":"not found"}}"#.to_vec(),
    );
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    match parts.as_slice() {
        ["v1", "packages", name] => {
            let name = decode(name);
            let versions: Vec<&str> = packages
                .iter()
                .filter(|p| p.name == name)
                .map(|p| p.version.as_str())
                .collect();
            if versions.is_empty() {
                return not_found;
            }
            let body = serde_json::json!({ "versions": versions }).to_string();
            ("200 OK", body.into_bytes())
        }
        ["v1", "packages", name, version] => {
            let name = decode(name);
            let Some(package) = packages
                .iter()
                .find(|p| p.name == name && p.version == *version)
            else {
                return not_found;
            };
            let body = serde_json::json!({
                "name": package.name,
                "version": package.version,
                "sha256": sha256(&package.wasm),
                "wasm_url": format!("/wasm/{}/{}", name.replace('/', "%2F"), version),
                "manifest": package.manifest(),
            })
            .to_string();
            ("200 OK", body.into_bytes())
        }
        ["v1", "wasm", name, version] => {
            let name = decode(name);
            match packages
                .iter()
                .find(|p| p.name == name && p.version == *version)
            {
                Some(package) => ("200 OK", package.wasm.clone()),
                None => not_found,
            }
        }
        _ => not_found,
    }
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}
