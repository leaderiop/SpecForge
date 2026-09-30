//! Parity harness (architecture plan 01, step P0).
//!
//! `specforge check`, MCP `specforge.validate`, the LSP and `specforge watch`
//! each assemble the compiler on their own. This harness runs every fixture
//! under `tests/fixtures/parity/` through all four and compares what each
//! reports with what `check` reports.
//!
//! Diagnostics are compared as multisets of `"CODE severity file:line"`
//! keys (`-` for a diagnostic without a span; the file is relative to the
//! spec root). Two surfaces cannot say everything `check` says, so `check`'s
//! keys are projected onto what they can say:
//! - the LSP must attach a span-less diagnostic to some document, and puts
//!   it on the edited one at line 1;
//! - watch prints only counts, so its keys are one `"error"` or `"warning"`
//!   per diagnostic. Its `rebuilt` event also gets a key when the debug
//!   build's check of the incremental graph against a cold build fails.
//!
//! Where a surface disagrees with `check` today, the disagreement is written
//! down in [`EXPECTED_DIVERGENCES`]. The harness asserts that each surface
//! differs from `check` by exactly those rows: no more, no less. A later
//! step that closes a divergence makes this test fail until its row is
//! deleted, so a step can only turn green on purpose.
//!
//! These tests characterize the surfaces; they prove no spec obligation, so
//! they are not linked. The obligations the plan names for this seam
//! (`incremental_correctness`, `diagnostic_determinism`) are violated today:
//! see the `D16` row, and `check`'s W113 message, which names an import
//! cycle in HashMap order.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tempfile::TempDir;

// ── Fixtures ────────────────────────────────────────────────────────────

/// What the LSP session does after opening the entry file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Then {
    Nothing,
    /// Report `specforge.json` as changed (an extension-environment reload).
    ReloadConfig,
    /// Delete this file (relative to the project root) and report it.
    Delete(&'static str),
}

/// Each fixture, the file the harness opens in the LSP and rewrites to make
/// watch rebuild (relative to the project root), and what the LSP session
/// does next.
const FIXTURES: &[(&str, &str, Then)] = &[
    ("clean", "main.spec", Then::Nothing),
    ("missing_import", "main.spec", Then::Nothing),
    ("unknown_extension", "main.spec", Then::Nothing),
    ("import_cycle", "a.spec", Then::Nothing),
    ("product_cycle", "main.spec", Then::Nothing),
    ("spec_root_set", "spec/main.spec", Then::ReloadConfig),
    ("exclude", "main.spec", Then::Nothing),
    ("body_parser_type", "main.spec", Then::Nothing),
    ("delete_file", "main.spec", Then::Delete("other.spec")),
];

// ── Expected divergences ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Surface {
    /// MCP `specforge.validate`, through `McpServer::handle_message`.
    Mcp,
    /// The LSP `Backend`: every `publishDiagnostics` after opening the
    /// fixture's entry file.
    Lsp,
    /// What the LSP publishes after `specforge.json` changes
    /// ([`Then::ReloadConfig`]), against `check` on the same project.
    LspAfterReload,
    /// What the LSP publishes after a file is deleted ([`Then::Delete`]),
    /// against `check` on the project without that file.
    LspAfterDelete,
    /// The error and warning counts of watch's `ready` event.
    WatchReady,
    /// The error and warning counts of watch's `rebuilt` event after the
    /// entry file is rewritten.
    WatchRebuilt,
}

/// One known disagreement between a surface and `check`, on one fixture.
struct Divergence {
    /// The plan's bug ID (plan 01 §6), for the step that will close it.
    id: &'static str,
    fixture: &'static str,
    surface: Surface,
    /// Keys `check` reports that the surface does not.
    missing: &'static [&'static str],
    /// Keys the surface reports that `check` does not.
    extra: &'static [&'static str],
    /// The difference shows on some runs and not on others: the surface is
    /// nondeterministic here. The harness accepts it present or absent.
    unstable: bool,
}

/// What each surface gets wrong today. Later steps delete rows; none may be
/// added to excuse a regression.
const EXPECTED_DIVERGENCES: &[Divergence] = &[
    // D1: watch never reports resolver diagnostics; the LSP never runs the
    // resolver. E025 reaches `check` and MCP only.
    Divergence {
        id: "D1",
        fixture: "missing_import",
        surface: Surface::Lsp,
        missing: &["E025 error main.spec:1"],
        extra: &[],
        unstable: false,
    },
    Divergence {
        id: "D1",
        fixture: "missing_import",
        surface: Surface::WatchReady,
        missing: &["error"],
        extra: &[],
        unstable: false,
    },
    Divergence {
        id: "D1",
        fixture: "missing_import",
        surface: Surface::WatchRebuilt,
        missing: &["error"],
        extra: &[],
        unstable: false,
    },
    // D2: extension load diagnostics (E028) reach `check` and MCP only.
    Divergence {
        id: "D2",
        fixture: "unknown_extension",
        surface: Surface::Lsp,
        missing: &["E028 error main.spec:1"],
        extra: &[],
        unstable: false,
    },
    Divergence {
        id: "D2",
        fixture: "unknown_extension",
        surface: Surface::WatchReady,
        missing: &["error"],
        extra: &[],
        unstable: false,
    },
    Divergence {
        id: "D2",
        fixture: "unknown_extension",
        surface: Surface::WatchRebuilt,
        missing: &["error"],
        extra: &[],
        unstable: false,
    },
    // D3: W113 differs by surface. The LSP's import DAG is keyed by
    // absolute paths, so it never sees the cycle. Watch's cold build drops
    // the resolver's W113 (as D1), and its own import DAG misses the cycle
    // at cold build (see the next row), so `ready` has no W113.
    Divergence {
        id: "D3",
        fixture: "import_cycle",
        surface: Surface::Lsp,
        missing: &["W113 warning a.spec:1"],
        extra: &[],
        unstable: false,
    },
    Divergence {
        id: "D3",
        fixture: "import_cycle",
        surface: Surface::WatchReady,
        missing: &["warning"],
        extra: &[],
        unstable: false,
    },
    // D3, and a nondeterminism: `ImportDag::set_imports_resolved` resolves
    // an import only against files already inserted, and the resolver
    // hands files over in HashMap order. So one direction of the cycle
    // stays unresolved at cold build, and whether a rebuild of `a.spec`
    // re-resolves both files (W113) or only one (none) depends on that
    // order. (`check`'s own W113 message also names the cycle in HashMap
    // order, "a.spec -> b.spec" or "b.spec -> a.spec"; the keys compared
    // here carry no message.)
    Divergence {
        id: "D3",
        fixture: "import_cycle",
        surface: Surface::WatchRebuilt,
        missing: &["warning"],
        extra: &[],
        unstable: true,
    },
    // D6: `exclude` is honoured only by the LSP; `check`, MCP and watch
    // read the excluded draft, so they report its duplicate ID and W004.
    // (Patterns match as path substrings, not globs: `drafts/` works,
    // `drafts/**` excludes nothing anywhere.)
    Divergence {
        id: "D6",
        fixture: "exclude",
        surface: Surface::Lsp,
        missing: &["E002 error main.spec:1", "W004 warning drafts/draft.spec:2"],
        extra: &[],
        unstable: false,
    },
    // D4: the LSP's delete branch re-checks with the default validator, not
    // `check_graph`, so extension-rule diagnostics in surviving files (here
    // software's required `contract`, E006) are published away.
    Divergence {
        id: "D4",
        fixture: "delete_file",
        surface: Surface::LspAfterDelete,
        missing: &["E006 error main.spec:2"],
        extra: &[],
        unstable: false,
    },
    // D5: after a `specforge.json` change the LSP reloads and re-indexes
    // (the project root, not the spec root) but never republishes.
    Divergence {
        id: "D5",
        fixture: "spec_root_set",
        surface: Surface::LspAfterReload,
        missing: &["E003 error main.spec:5"],
        extra: &[],
        unstable: false,
    },
    // Not in the plan (found by this harness): with a duplicate ID, the cold
    // build keeps the first declaration (drafts/draft.spec) but watch's
    // incremental rebuild of main.spec keeps main.spec's. The rebuilt graph
    // differs from a cold build (the debug build's verification fails) and
    // the draft's W004 disappears. Closing D6 hides it on this fixture
    // without fixing it.
    Divergence {
        id: "D16",
        fixture: "exclude",
        surface: Surface::WatchRebuilt,
        missing: &["warning"],
        extra: &[INCREMENTAL_MISMATCH],
        unstable: false,
    },
    // D10: E001 suppression for body-parser kinds is dead code, so every
    // surface reports the E001s inside the `type` body. On top of that the
    // LSP's syntax-only fast path (C4-07) publishes only the E001 layer
    // while the edited file has parse errors, so it drops the rest. Once
    // D10 is closed the file parses cleanly and this row goes with it.
    Divergence {
        id: "D10",
        fixture: "body_parser_type",
        surface: Surface::Lsp,
        missing: &[
            "W002 warning main.spec:4",
            "W004 warning main.spec:4",
            "W010 warning main.spec:4",
        ],
        extra: &[],
        unstable: false,
    },
];

// ── Normalized diagnostics ──────────────────────────────────────────────

/// `"CODE severity file:line"`, or `"CODE severity -"` without a span.
fn key(code: &str, severity: &str, location: Option<(String, u64)>) -> String {
    let severity = severity.to_ascii_lowercase();
    match location {
        Some((file, line)) => format!("{code} {severity} {file}:{line}"),
        None => format!("{code} {severity} -"),
    }
}

/// Counts of each key: a multiset.
type Keys = BTreeMap<String, usize>;

fn multiset<I: IntoIterator<Item = String>>(keys: I) -> Keys {
    let mut set = Keys::new();
    for k in keys {
        *set.entry(k).or_default() += 1;
    }
    set
}

/// The keys in `a` beyond those in `b`, sorted, with repeats.
fn difference(a: &Keys, b: &Keys) -> Vec<String> {
    let mut out = Vec::new();
    for (k, &n) in a {
        let m = b.get(k).copied().unwrap_or(0);
        for _ in m..n {
            out.push(k.clone());
        }
    }
    out
}

/// Project `check`'s keys onto what the LSP can publish. A diagnostic
/// without a span has to be attached to some document, and the LSP
/// attaches it to the one being edited, at its first line.
fn as_published(keys: &Keys, edited: &str) -> Keys {
    let mut out = Keys::new();
    for (k, &n) in keys {
        let k = match k.strip_suffix(" -") {
            Some(head) => format!("{head} {edited}:1"),
            None => k.clone(),
        };
        *out.entry(k).or_default() += n;
    }
    out
}

/// Project `check`'s keys onto what watch can report: one severity per
/// error or warning.
fn severities(keys: &Keys) -> Keys {
    let mut out = Keys::new();
    for (k, &n) in keys {
        let severity = k.split(' ').nth(1).unwrap_or_default();
        if severity == "error" || severity == "warning" {
            *out.entry(severity.to_string()).or_default() += n;
        }
    }
    out
}

// ── The project under test ──────────────────────────────────────────────

struct Project {
    _dir: TempDir,
    /// Canonical project root.
    root: PathBuf,
    /// Canonical spec root (the root, or `root/spec_root`).
    spec_root: PathBuf,
    /// The fixture's entry file, absolute.
    entry: PathBuf,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parity")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn project(fixture: &str, entry: &str) -> Project {
    let dir = TempDir::new().unwrap();
    copy_tree(&fixtures_dir().join(fixture), dir.path());
    let root = fs::canonicalize(dir.path()).unwrap();
    let config: Value =
        serde_json::from_str(&fs::read_to_string(root.join("specforge.json")).unwrap()).unwrap();
    let spec_root = match config["spec_root"].as_str() {
        Some(sr) => root.join(sr),
        None => root.clone(),
    };
    Project {
        entry: root.join(entry),
        _dir: dir,
        root,
        spec_root,
    }
}

impl Project {
    /// `path` relative to the spec root, as `check` prints it.
    fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.spec_root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }
}

// ── check ───────────────────────────────────────────────────────────────

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_specforge")
}

/// `specforge check --format json`, as keys in output order.
fn check(project: &Project) -> Vec<String> {
    let out = Command::new(binary())
        .args(["check", "--format", "json"])
        .arg(&project.root)
        .output()
        .unwrap();
    let diagnostics: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("check JSON ({e}): {}", String::from_utf8_lossy(&out.stdout)));
    diagnostics
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            let location = d["span"].as_object().map(|s| {
                (
                    s["file"].as_str().unwrap().to_string(),
                    s["start_line"].as_u64().unwrap(),
                )
            });
            key(
                d["code"].as_str().unwrap(),
                d["severity"].as_str().unwrap(),
                location,
            )
        })
        .collect()
}

// ── MCP ─────────────────────────────────────────────────────────────────

fn mcp(project: &Project) -> Keys {
    let mut server = specforge_mcp::McpServer::new();
    let mut call = |method: &str, params: Value| -> Value {
        let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
        let resp = server.handle_message(&req.to_string()).unwrap();
        serde_json::from_str(&resp).unwrap()
    };
    call(
        "initialize",
        json!({"projectRoot": project.root.to_str().unwrap()}),
    );
    let resp = call(
        "tools/call",
        json!({"name": "specforge.validate", "arguments": {}}),
    );
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("validate returned no text: {resp}"));
    let diagnostics: Value = serde_json::from_str(text).unwrap();
    multiset(diagnostics.as_array().unwrap().iter().map(|d| {
        let location = d["file"]
            .as_str()
            .map(|f| (f.to_string(), d["line"].as_u64().unwrap()));
        key(
            d["code"].as_str().unwrap(),
            d["severity"].as_str().unwrap(),
            location,
        )
    }))
}

// ── LSP ─────────────────────────────────────────────────────────────────

mod lsp {
    use serde_json::{Value, json};
    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
    use tokio::time::{Duration, Instant, timeout};
    use tower_lsp::lsp_types::Url;
    use tower_lsp::{LspService, Server};

    /// A minimal in-process LSP client over memory streams.
    pub struct Client {
        writer: DuplexStream,
        reader: DuplexStream,
        next_id: i64,
        server: tokio::task::JoinHandle<()>,
    }

    impl Drop for Client {
        fn drop(&mut self) {
            self.server.abort();
        }
    }

    impl Client {
        pub fn start() -> Self {
            let (writer, server_in) = tokio::io::duplex(1 << 20);
            let (server_out, reader) = tokio::io::duplex(1 << 20);
            let (service, socket) = LspService::new(specforge_lsp::backend::Backend::new);
            let server = tokio::spawn(async move {
                Server::new(server_in, server_out, socket)
                    .serve(service)
                    .await;
            });
            Client {
                writer,
                reader,
                next_id: 1,
                server,
            }
        }

        async fn write(&mut self, msg: &Value) {
            let body = msg.to_string();
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            self.writer.write_all(header.as_bytes()).await.unwrap();
            self.writer.write_all(body.as_bytes()).await.unwrap();
            self.writer.flush().await.unwrap();
        }

        async fn read(&mut self) -> Value {
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") {
                let mut byte = [0u8; 1];
                self.reader.read_exact(&mut byte).await.unwrap();
                header.push(byte[0]);
            }
            let length: usize = String::from_utf8(header)
                .unwrap()
                .lines()
                .find_map(|l| l.strip_prefix("Content-Length:"))
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            let mut body = vec![0u8; length];
            self.reader.read_exact(&mut body).await.unwrap();
            let msg: Value = serde_json::from_slice(&body).unwrap();
            // Answer server-to-client requests (progress creation,
            // capability registration) so the server never waits.
            if msg.get("method").is_some() && msg.get("id").is_some() {
                self.write(&json!({"jsonrpc": "2.0", "id": msg["id"], "result": null}))
                    .await;
            }
            msg
        }

        async fn request(&mut self, method: &str, params: Value) -> Value {
            let id = self.next_id;
            self.next_id += 1;
            self.write(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
                .await;
            loop {
                let msg = self.read().await;
                if msg.get("method").is_none() && msg["id"] == id {
                    return msg;
                }
            }
        }

        async fn notify(&mut self, method: &str, params: Value) {
            self.write(&json!({"jsonrpc": "2.0", "method": method, "params": params}))
                .await;
        }

        /// Initialize on `root` and wait until background indexing ends.
        pub async fn open_workspace(&mut self, root: &std::path::Path) {
            let uri = Url::from_file_path(root).unwrap().to_string();
            self.request(
                "initialize",
                json!({"processId": null, "rootUri": uri, "capabilities": {}}),
            )
            .await;
            self.notify("initialized", json!({})).await;
            let deadline = Instant::now() + Duration::from_secs(120);
            loop {
                let left = deadline.saturating_duration_since(Instant::now());
                let msg = timeout(left, self.read())
                    .await
                    .expect("LSP indexing never ended");
                if msg["method"] == "$/progress" && msg["params"]["value"]["kind"] == "end" {
                    return;
                }
            }
        }

        /// Collect every `publishDiagnostics` until the server goes quiet:
        /// the last list published for each URI. Waits up to `first` for
        /// the first publish, then until `quiet` passes without one.
        async fn collect(&mut self, first: Duration, quiet: Duration) -> super::Published {
            let mut published = super::Published::new();
            let deadline = Instant::now() + first;
            loop {
                let wait = if published.is_empty() {
                    deadline.saturating_duration_since(Instant::now())
                } else {
                    quiet
                };
                let Ok(msg) = timeout(wait, self.read()).await else {
                    return published;
                };
                if msg["method"] == "textDocument/publishDiagnostics" {
                    let params = &msg["params"];
                    published.insert(
                        params["uri"].as_str().unwrap().to_string(),
                        params["diagnostics"].as_array().unwrap().clone(),
                    );
                }
            }
        }

        /// Open `path` and collect what the server publishes.
        pub async fn open(&mut self, path: &std::path::Path) -> super::Published {
            let uri = Url::from_file_path(path).unwrap().to_string();
            let text = std::fs::read_to_string(path).unwrap();
            self.notify(
                "textDocument/didOpen",
                json!({"textDocument": {
                    "uri": uri, "languageId": "specforge", "version": 1, "text": text
                }}),
            )
            .await;
            let published = self
                .collect(Duration::from_secs(60), Duration::from_secs(1))
                .await;
            assert!(
                published.contains_key(&uri),
                "the LSP never published diagnostics for {uri}"
            );
            published
        }

        /// Report `path` as changed (2) or deleted (3) and collect what the
        /// server publishes in response.
        pub async fn watched_file(
            &mut self,
            path: &std::path::Path,
            change: u8,
        ) -> super::Published {
            let uri = Url::from_file_path(path).unwrap().to_string();
            self.notify(
                "workspace/didChangeWatchedFiles",
                json!({"changes": [{"uri": uri, "type": change}]}),
            )
            .await;
            self.collect(Duration::from_secs(10), Duration::from_secs(1))
                .await
        }
    }
}

/// The last diagnostics published for each URI.
type Published = BTreeMap<String, Vec<Value>>;

/// What one LSP session published: after opening the entry file, and after
/// the fixture's [`Then`] step, if any.
struct LspRun {
    opened: Keys,
    then: Option<Keys>,
}

fn lsp(project: &Project, then: Then) -> LspRun {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let (opened, then) = runtime.block_on(async {
        let mut client = lsp::Client::start();
        client.open_workspace(&project.root).await;
        let opened = client.open(&project.entry).await;
        let then = match then {
            Then::Nothing => None,
            Then::ReloadConfig => Some(
                client
                    .watched_file(&project.root.join("specforge.json"), 2)
                    .await,
            ),
            Then::Delete(file) => {
                let path = project.root.join(file);
                fs::remove_file(&path).unwrap();
                Some(client.watched_file(&path, 3).await)
            }
        };
        (opened, then)
    });
    LspRun {
        opened: published_keys(project, opened),
        then: then.map(|p| published_keys(project, p)),
    }
}

fn published_keys(project: &Project, published: Published) -> Keys {
    let mut keys = Vec::new();
    for (uri, diagnostics) in published {
        let path = tower_lsp::lsp_types::Url::parse(&uri)
            .unwrap()
            .to_file_path()
            .unwrap();
        let file = project.relative(&path);
        for d in diagnostics {
            let severity = match d["severity"].as_u64() {
                Some(1) => "error",
                Some(2) => "warning",
                Some(3) => "info",
                other => panic!("unexpected LSP severity {other:?}: {d}"),
            };
            let line = d["range"]["start"]["line"].as_u64().unwrap() + 1;
            keys.push(key(
                d["code"].as_str().unwrap(),
                severity,
                Some((file.clone(), line)),
            ));
        }
    }
    multiset(keys)
}

// ── watch ───────────────────────────────────────────────────────────────

/// Wait for a JSON event line whose `event` is `name`.
fn wait_for_event(rx: &mpsc::Receiver<String>, name: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(120);
    while Instant::now() < deadline {
        if let Ok(line) = rx.recv_timeout(Duration::from_secs(1))
            && let Ok(event) = serde_json::from_str::<Value>(&line)
            && event["event"] == name
        {
            return event;
        }
    }
    panic!("watch never reported {name}");
}

/// The key a `rebuilt` event adds when the debug build's check of the
/// incremental graph against a cold rebuild fails.
const INCREMENTAL_MISMATCH: &str = "incremental graph differs from a cold build";

/// One severity key per error and warning an event counts, plus
/// [`INCREMENTAL_MISMATCH`] when the event's verification failed.
fn event_keys(event: &Value) -> Keys {
    let mut keys = Keys::new();
    for severity in ["error", "warning"] {
        let n = event[format!("{severity}s")].as_u64().unwrap() as usize;
        if n > 0 {
            keys.insert(severity.to_string(), n);
        }
    }
    if event["verification_failed"] == true {
        keys.insert(INCREMENTAL_MISMATCH.to_string(), 1);
    }
    keys
}

/// Watch's `ready` counts, then its `rebuilt` counts after the entry file
/// is rewritten with the same entities.
fn watch(project: &Project) -> (Keys, Keys) {
    let mut child = Command::new(binary())
        .args(["watch", "--json", "--path"])
        .arg(&project.root)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                return;
            }
        }
    });
    let ready = wait_for_event(&rx, "ready");
    std::thread::sleep(Duration::from_millis(300));
    let text = fs::read_to_string(&project.entry).unwrap();
    fs::write(&project.entry, format!("{text}\n")).unwrap();
    let rebuilt = wait_for_event(&rx, "rebuilt");
    let _ = child.kill();
    let _ = child.wait();
    // A debug build checks every rebuild against a cold one; the harness
    // relies on that to see incremental drift.
    assert!(
        !rebuilt["verification"].is_null(),
        "watch did not verify the rebuild: {rebuilt}"
    );
    (event_keys(&ready), event_keys(&rebuilt))
}

// ── The comparison ──────────────────────────────────────────────────────

/// The difference the table expects for one surface on one fixture:
/// `(missing, extra)`, sorted. With `with_unstable` false, rows marked
/// unstable are left out.
fn expected(fixture: &str, surface: Surface, with_unstable: bool) -> (Vec<String>, Vec<String>) {
    let mut missing = Vec::new();
    let mut extra = Vec::new();
    for row in EXPECTED_DIVERGENCES
        .iter()
        .filter(|d| d.fixture == fixture && d.surface == surface)
        .filter(|d| with_unstable || !d.unstable)
    {
        missing.extend(row.missing.iter().map(|s| s.to_string()));
        extra.extend(row.extra.iter().map(|s| s.to_string()));
    }
    missing.sort();
    extra.sort();
    (missing, extra)
}

fn compare(fixture: &str, surface: Surface, check: &Keys, got: &Keys, failures: &mut Vec<String>) {
    let found = (difference(check, got), difference(got, check));
    let want = expected(fixture, surface, true);
    if found == want || found == expected(fixture, surface, false) {
        return;
    }
    let ids: Vec<&str> = EXPECTED_DIVERGENCES
        .iter()
        .filter(|d| d.fixture == fixture && d.surface == surface)
        .map(|d| d.id)
        .collect();
    failures.push(format!(
        "{surface:?} on `{fixture}` differs from check by\n    \
         missing {:?}, extra {:?}\n  but EXPECTED_DIVERGENCES {ids:?} says\n    \
         missing {:?}, extra {:?}\n  \
         (a closed divergence: delete its row; a new one: fix the surface)",
        found.0, found.1, want.0, want.1
    ));
}

fn assert_parity(fixture: &str) {
    let &(_, entry, then) = FIXTURES
        .iter()
        .find(|(name, _, _)| *name == fixture)
        .unwrap();
    let project = project(fixture, entry);

    // The baseline must hold still: the same keys in the same order on a
    // second run (messages are not compared, see the module docs).
    let first = check(&project);
    let second = check(&project);
    assert_eq!(first, second, "check is not deterministic on `{fixture}`");
    let check_keys = multiset(first);
    let edited = project.relative(&project.entry);

    let mut failures = Vec::new();
    compare(
        fixture,
        Surface::Mcp,
        &check_keys,
        &mcp(&project),
        &mut failures,
    );

    let (ready, rebuilt) = watch(&project);
    let watchable = severities(&check_keys);
    compare(
        fixture,
        Surface::WatchReady,
        &watchable,
        &ready,
        &mut failures,
    );
    compare(
        fixture,
        Surface::WatchRebuilt,
        &watchable,
        &rebuilt,
        &mut failures,
    );

    // Last: its Then step may delete a file.
    let run = lsp(&project, then);
    let published = as_published(&check_keys, &edited);
    compare(
        fixture,
        Surface::Lsp,
        &published,
        &run.opened,
        &mut failures,
    );
    match (then, run.then) {
        (Then::ReloadConfig, Some(after)) => compare(
            fixture,
            Surface::LspAfterReload,
            &published,
            &after,
            &mut failures,
        ),
        (Then::Delete(_), Some(after)) => {
            let now = as_published(&multiset(check(&project)), &edited);
            compare(
                fixture,
                Surface::LspAfterDelete,
                &now,
                &after,
                &mut failures,
            )
        }
        _ => {}
    }

    assert!(
        failures.is_empty(),
        "check reports {check_keys:?} on `{fixture}`\n{}",
        failures.join("\n")
    );
}

// ── Tests: one per fixture ──────────────────────────────────────────────

#[test]
fn parity_clean() {
    assert_parity("clean");
}

#[test]
fn parity_missing_import() {
    assert_parity("missing_import");
}

#[test]
fn parity_unknown_extension() {
    assert_parity("unknown_extension");
}

#[test]
fn parity_import_cycle() {
    assert_parity("import_cycle");
}

#[test]
fn parity_product_cycle() {
    assert_parity("product_cycle");
}

#[test]
fn parity_spec_root_set() {
    assert_parity("spec_root_set");
}

#[test]
fn parity_exclude() {
    assert_parity("exclude");
}

#[test]
fn parity_body_parser_type() {
    assert_parity("body_parser_type");
}

#[test]
fn parity_delete_file() {
    assert_parity("delete_file");
}

/// Every row names a fixture the harness runs, and every fixture exists.
#[test]
fn parity_table_names_real_fixtures() {
    for row in EXPECTED_DIVERGENCES {
        assert!(
            FIXTURES.iter().any(|(name, _, _)| *name == row.fixture),
            "{} names unknown fixture `{}`",
            row.id,
            row.fixture
        );
        assert!(
            !row.missing.is_empty() || !row.extra.is_empty(),
            "{} on `{}` is empty",
            row.id,
            row.fixture
        );
    }
    for (name, entry, then) in FIXTURES {
        let dir = fixtures_dir().join(name);
        assert!(
            dir.join("specforge.json").is_file(),
            "{name}: no specforge.json"
        );
        assert!(dir.join(entry).is_file(), "{name}: no {entry}");
        if let Then::Delete(file) = then {
            assert!(dir.join(file).is_file(), "{name}: no {file}");
        }
    }
}
