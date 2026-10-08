use assert_cmd::Command;
use specforge_test_macros::test as specforge_test;
use tempfile::TempDir;

fn specforge_cmd() -> Command {
    assert_cmd::cargo_bin_cmd!("specforge")
}

#[test]
fn add_invalid_specifier_fails() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .arg("add")
        .arg("") // empty specifier
        .arg("--path")
        .arg(dir.path())
        .assert()
        .failure();
}

#[test]
fn add_local_nonexistent_file_fails() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .arg("add")
        .arg("./nonexistent.wasm")
        .arg("--path")
        .arg(dir.path())
        .assert()
        .failure();
}

#[test]
fn add_local_file_succeeds() {
    let dir = project_without_registry();

    let wasm_path = dir.path().join("test-ext.wasm");
    std::fs::write(&wasm_path, greet_wasm()).unwrap();

    specforge_cmd()
        .arg("add")
        .arg(wasm_path.to_str().unwrap())
        .arg("--path")
        .arg(dir.path())
        .assert()
        .success();

    // Verify lock file was created
    assert!(dir.path().join("specforge.lock").exists());
}

#[test]
fn add_local_file_json_output() {
    let dir = project_without_registry();

    let wasm_path = dir.path().join("my-ext.wasm");
    std::fs::write(&wasm_path, greet_wasm()).unwrap();

    let output = specforge_cmd()
        .arg("add")
        .arg(wasm_path.to_str().unwrap())
        .arg("--path")
        .arg(dir.path())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["action"], "add");
    assert_eq!(json["source"], "local:my-ext.wasm");
}

#[test]
fn publish_without_a_built_extension_fails() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .arg("publish")
        .arg("--path")
        .arg(dir.path())
        .assert()
        .failure();
}

#[test]
fn publish_without_a_built_extension_json_output() {
    let dir = TempDir::new().unwrap();
    let output = specforge_cmd()
        .arg("publish")
        .arg("--path")
        .arg(dir.path())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["code"], "E040");
}

#[test]
fn update_without_lockfile_fails() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .arg("update")
        .arg("--path")
        .arg(dir.path())
        .assert()
        .failure();
}

#[test]
fn logout_without_credentials_succeeds() {
    specforge_cmd().arg("logout").assert().success();
}

#[test]
fn login_without_token_fails() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .arg("login")
        .arg("--path")
        .arg(dir.path())
        .assert()
        .failure();
}

// ===============================================================
// No built-in registry (ADR 0004 N1)
// ===============================================================
//
// SpecForge does not own `specforge.dev`, so it ships no default registry:
// with none configured, every registry command must fail with E063 before
// it touches the network. `NetSpy` proves "before": it is the proxy every
// HTTP client in the child is pointed at, and it counts the connections it
// receives.

/// A local listener set as the child's HTTP(S) proxy; it accepts and drops
/// every connection, counting them.
struct NetSpy {
    url: String,
    hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    home: TempDir,
}

impl NetSpy {
    fn start() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = hits.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if stream.is_ok() {
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            }
        });
        NetSpy {
            url,
            hits,
            home: TempDir::new().unwrap(),
        }
    }

    /// The environment of a spied child: every proxy variable points at the
    /// spy, and HOME (credentials, keys, pins) is a throwaway directory.
    fn env(&self) -> Vec<(&'static str, String)> {
        let mut env: Vec<(&'static str, String)> = [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ]
        .into_iter()
        .map(|var| (var, self.url.clone()))
        .collect();
        env.push(("NO_PROXY", String::new()));
        env.push(("no_proxy", String::new()));
        env.push(("HOME", self.home.path().display().to_string()));
        env
    }

    /// `specforge <args>` under the spy.
    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = specforge_cmd();
        cmd.args(args)
            .envs(self.env())
            .env_remove("SPECFORGE_REGISTRY_TOKEN");
        cmd
    }

    /// Connections received so far (after a moment for the accept loop).
    fn hits(&self) -> usize {
        std::thread::sleep(std::time::Duration::from_millis(200));
        self.hits.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// A project whose specforge.json configures no registry.
fn project_without_registry() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"demo","version":"0.1.0","extensions":[]}"#,
    )
    .unwrap();
    dir
}

/// The command failed with E063, and its hint names the `registries` key.
fn assert_no_registry(output: &std::process::Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success(), "expected failure: {stdout}");
    let json: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {stdout}"));
    assert_eq!(json["code"], "E063", "{json}");
    let hint = json["suggestion"].as_str().unwrap_or_default();
    assert!(
        hint.contains("\"registries\"") && hint.contains("specforge.json"),
        "the hint must say how to configure a registry: {json}"
    );
}

#[specforge_test(
    behavior = "add_extension_to_existing_project",
    verify = "with no registry configured, add makes no network call and reports how to configure one"
)]
fn add_without_registry_makes_no_network_call() {
    let spy = NetSpy::start();
    let dir = project_without_registry();
    let output = spy
        .command(&["add", "@acme/widget@1.0.0", "--format", "json", "--path"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_no_registry(&output);
    assert_eq!(spy.hits(), 0, "add reached the network");
}

#[specforge_test(
    behavior = "search_registry",
    verify = "with no registry configured, search makes no network call and reports how to configure one"
)]
fn search_without_registry_makes_no_network_call() {
    let spy = NetSpy::start();
    let dir = project_without_registry();
    let output = spy
        .command(&["search", "widget", "--format", "json", "--path"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_no_registry(&output);
    assert_eq!(spy.hits(), 0, "search reached the network");
}

#[specforge_test(
    behavior = "search_registry",
    verify = "an unusable specforge.json is refused with the refusal add gives, before any network call"
)]
fn search_and_login_refuse_an_unusable_config_as_add_does() {
    let spy = NetSpy::start();
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("specforge.json"), "{ \"extensions\": [").unwrap();
    let refusal = |args: &[&str]| -> (bool, serde_json::Value) {
        let output = spy
            .command(args)
            .args(["--format", "json", "--path"])
            .arg(dir.path())
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json = serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{e}: {stdout}"));
        (output.status.success(), json)
    };

    let (ok, added) = refusal(&["add", "@acme/widget@1.0.0"]);
    assert!(!ok);
    assert_eq!(added["code"], "config_invalid", "{added}");

    // (`publish` builds its declaration first; its registry read is the same
    // `configured`, covered in specforge-ops-registry.)
    for args in [&["search", "widget"][..], &["login", "--token", "t"]] {
        let (ok, json) = refusal(args);
        assert!(!ok, "{args:?}");
        assert_eq!(json["code"], added["code"], "{args:?}: {json}");
        assert_eq!(json["message"], added["message"], "{args:?}: {json}");
    }
    assert_eq!(spy.hits(), 0, "a command reached the network");
}

#[specforge_test(
    behavior = "update_all_extensions",
    verify = "with no registry configured, update makes no network call and reports how to configure one"
)]
fn update_without_registry_makes_no_network_call() {
    let spy = NetSpy::start();
    let dir = project_without_registry();
    std::fs::write(
        dir.path().join("specforge.lock"),
        r#"{"lockfile_version":1,"entries":[
            {"name":"@acme/widget","version":"1.0.0","source":"registry","wasm_hash":"00"}]}"#,
    )
    .unwrap();
    let output = spy
        .command(&["update", "--format", "json", "--path"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_no_registry(&output);
    assert_eq!(spy.hits(), 0, "update reached the network");
}

#[specforge_test(
    behavior = "publish_to_registry",
    verify = "with no registry configured, publish makes no network call and reports how to configure one"
)]
fn publish_without_registry_makes_no_network_call() {
    let spy = NetSpy::start();
    let dir = project_without_registry();
    std::fs::write(dir.path().join("greet.wasm"), greet_wasm()).unwrap();
    let output = spy
        .command(&["publish", "--format", "json"])
        .arg(dir.path().join("greet.wasm"))
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();
    assert_no_registry(&output);
    assert_eq!(spy.hits(), 0, "publish reached the network");
}

#[specforge_test(
    behavior = "validate_registry_credentials",
    verify = "with no registry configured, login makes no network call and reports how to configure one"
)]
fn login_without_registry_makes_no_network_call() {
    let spy = NetSpy::start();
    let dir = project_without_registry();
    let output = spy
        .command(&["login", "--token", "t0k3n", "--format", "json", "--path"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_no_registry(&output);
    assert_eq!(spy.hits(), 0, "login reached the network");
}

#[specforge_test(
    behavior = "provide_mcp_add_extension_tool",
    verify = "with no registry configured, add_extension makes no network call and reports how to configure one"
)]
fn mcp_add_extension_without_registry_makes_no_network_call() {
    use std::io::Write;
    let spy = NetSpy::start();
    let dir = project_without_registry();
    let mut child = std::process::Command::new(assert_cmd::cargo_bin!("specforge"))
        .arg("mcp")
        .arg(dir.path())
        .envs(spy.env())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let requests = [
        serde_json::json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
            "protocolVersion": "2025-03-26", "capabilities": {},
            "clientInfo": {"name": "t", "version": "0"}}}),
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
            "name": "specforge.add_extension",
            "arguments": {"specifier": "@acme/widget"}}}),
    ];
    {
        let stdin = child.stdin.as_mut().unwrap();
        for request in &requests {
            writeln!(stdin, "{request}").unwrap();
        }
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: serde_json::Value = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|r| r["id"] == 1)
        .unwrap_or_else(|| panic!("no response to the call: {stdout}"));

    let error = crate::e2e_fixtures::tool_error(&response);
    assert_eq!(error["diagnostic"]["code"], "E063", "{response}");
    let hint = error["diagnostic"]["suggestion"]
        .as_str()
        .unwrap_or_default();
    assert!(
        hint.contains("\"registries\"") && hint.contains("specforge.json"),
        "the hint must say how to configure a registry: {response}"
    );
    assert_eq!(spy.hits(), 0, "add_extension reached the network");
}

// ===============================================================
// The registry configuration's own diagnostics are shown
// ===============================================================
//
// Reading `registries` reports E067 (an unreadable entry), W140 (a
// duplicate alias) and I003 (no default registry). The registry commands
// show them on stderr; an entry that can't be read is skipped, and when
// none can be read the command fails with E067 instead of E063.

const UNREACHABLE: &str = "http://registry.invalid/v1";

/// A project whose specforge.json has `registries` set to `registries`.
fn project_with_registries(registries: serde_json::Value) -> TempDir {
    let dir = TempDir::new().unwrap();
    let config = serde_json::json!({
        "name": "demo",
        "version": "0.1.0",
        "extensions": [],
        "registries": registries,
    });
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    dir
}

/// `specforge <args> --path <dir>` under a spy; returns its output.
fn run_spied(args: &[&str], dir: &TempDir) -> std::process::Output {
    NetSpy::start()
        .command(args)
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap()
}

fn stderr_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[specforge_test(
    behavior = "configure_registries",
    verify = "duplicate alias produces warning"
)]
fn search_shows_a_duplicate_registry_alias() {
    let dir = project_with_registries(serde_json::json!([
        {"alias": "main", "url": UNREACHABLE, "default_registry": true},
        {"alias": "main", "url": UNREACHABLE},
    ]));
    let stderr = stderr_of(&run_spied(&["search", "widget"], &dir));
    assert!(
        stderr.contains("warning[W140]: Duplicate registry alias \"main\""),
        "{stderr}"
    );
}

#[test]
fn add_from_a_registry_shows_a_duplicate_registry_alias() {
    let dir = project_with_registries(serde_json::json!([
        {"alias": "main", "url": UNREACHABLE, "default_registry": true},
        {"alias": "main", "url": UNREACHABLE},
    ]));
    let stderr = stderr_of(&run_spied(&["add", "@acme/widget@1.0.0"], &dir));
    assert!(stderr.contains("warning[W140]"), "{stderr}");
}

#[test]
fn adding_a_builtin_does_not_read_the_registries() {
    let dir = project_with_registries(serde_json::json!([
        {"alias": "main", "url": UNREACHABLE, "default_registry": true},
        {"alias": "main", "url": UNREACHABLE},
    ]));
    let output = run_spied(&["add", "@specforge/software"], &dir);
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(
        !stderr_of(&output).contains("W140"),
        "{}",
        stderr_of(&output)
    );
}

#[test]
fn search_shows_that_no_registry_is_the_default() {
    let dir = project_with_registries(serde_json::json!([
        {"alias": "main", "url": UNREACHABLE},
    ]));
    let stderr = stderr_of(&run_spied(&["search", "widget"], &dir));
    assert!(stderr.contains("info[I003]"), "{stderr}");
}

#[test]
fn login_shows_an_unreadable_registry_entry_and_skips_it() {
    let dir = project_with_registries(serde_json::json!([
        {"alias": "main", "url": UNREACHABLE, "default_registry": true},
        {"alias": "broken"},
    ]));
    let output = run_spied(&["login", "--token", "t"], &dir);
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("error[E067]: Failed to parse registry entry at index 1"),
        "{stderr}"
    );
}

#[test]
fn search_with_no_readable_registry_entry_fails_with_e067() {
    let dir = project_with_registries(serde_json::json!([{"alias": "broken"}]));
    let output = run_spied(&["search", "widget", "--format", "json"], &dir);
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {stdout}"));
    assert_eq!(json["code"], "E067", "{json}");
    assert!(
        json["error"]
            .as_str()
            .unwrap_or_default()
            .contains("index 0"),
        "{json}"
    );
}

#[specforge_test(
    behavior = "configure_registries",
    verify = "builtins and local .wasm files install with no registry configured"
)]
fn builtins_and_local_wasm_install_without_registry() {
    let spy = NetSpy::start();
    let dir = project_without_registry();
    let wasm = dir.path().join("local-ext.wasm");
    std::fs::write(&wasm, greet_wasm()).unwrap();

    for specifier in ["@specforge/product", wasm.to_str().unwrap()] {
        let output = spy
            .command(&["add", specifier, "--format", "json", "--path"])
            .arg(dir.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "add {specifier}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    let config = std::fs::read_to_string(dir.path().join("specforge.json")).unwrap();
    assert!(config.contains("@specforge/product"), "{config}");
    let lock = std::fs::read_to_string(dir.path().join("specforge.lock")).unwrap();
    assert!(lock.contains("local:local-ext.wasm"), "{lock}");
    assert_eq!(spy.hits(), 0, "offline installs reached the network");
}

// ---------------------------------------------------------------
// Registry installs against a local fake registry
// ---------------------------------------------------------------

pub(crate) fn greet_wasm() -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/greet-extension/greet.wasm"),
    )
    .expect("the greet fixture is vendored")
}

/// A project whose only registry is `registry`.
pub(crate) fn project_on(registry: &crate::fake_registry::FakeRegistry) -> TempDir {
    let dir = TempDir::new().unwrap();
    let config = serde_json::json!({
        "name": "p",
        "version": "0.1.0",
        "extensions": ["@specforge/software"],
        "registries": registry.config_entry(),
    });
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    dir
}

#[specforge_test(
    behavior = "add_extension_to_existing_project",
    verify = "add extension without version resolves to latest compatible version"
)]
fn add_without_a_version_installs_the_latest() {
    use crate::fake_registry::{FakeRegistry, Package};
    let registry = FakeRegistry::serve(vec![
        Package::new("@sdk/greet", "0.0.1", greet_wasm()),
        Package::new("@sdk/greet", "0.1.0", greet_wasm()),
    ]);
    let dir = project_on(&registry);
    let home = TempDir::new().unwrap();

    let output = specforge_cmd()
        .args(["add", "@sdk/greet", "--allow-unsigned", "--format", "json"])
        .arg("--path")
        .arg(dir.path())
        .env("HOME", home.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["name"], "@sdk/greet", "{json}");
    assert_eq!(json["version"], "0.1.0", "{json}");
    let lock = std::fs::read_to_string(dir.path().join("specforge.lock")).unwrap();
    assert!(lock.contains("\"0.1.0\""), "{lock}");
}

// ---------------------------------------------------------------
// Publisher trust through `specforge add` (docs/registry-trust.md)
// ---------------------------------------------------------------

/// `specforge add @sdk/greet@0.1.0 --format json` in `dir`, with `home` as
/// `$HOME` (where the known-keys store lives), plus `extra` flags.
fn add_greet(dir: &TempDir, home: &TempDir, extra: &[&str]) -> std::process::Output {
    specforge_cmd()
        .args(["add", "@sdk/greet@0.1.0", "--format", "json"])
        .args(extra)
        .arg("--path")
        .arg(dir.path())
        .env("HOME", home.path())
        .output()
        .unwrap()
}

/// The add failed with `code`, and installed nothing.
fn assert_refused(output: &std::process::Output, dir: &TempDir, code: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success(), "expected failure: {stdout}");
    let json: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {stdout}"));
    assert_eq!(json["code"], code, "{json}");
    assert!(
        !dir.path().join("specforge.lock").exists(),
        "a refused package is not locked"
    );
}

fn known_keys(home: &TempDir) -> std::path::PathBuf {
    home.path().join(".specforge").join("known-keys.json")
}

#[specforge_test(
    invariant = "publisher_trust",
    verify = "specforge add pins the publisher key and records it in specforge.lock"
)]
fn add_installs_a_signed_package_and_pins_its_key() {
    use crate::fake_registry::{FakeRegistry, Package};
    let key = specforge_registry_client::SigningKey::generate();
    let registry = FakeRegistry::serve(vec![
        Package::new("@sdk/greet", "0.1.0", greet_wasm()).signed_by(&key),
    ]);
    let dir = project_on(&registry);
    let home = TempDir::new().unwrap();

    let output = add_greet(&dir, &home, &[]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let known = specforge_registry_client::load_known_keys_at(&known_keys(&home));
    assert_eq!(known.pin_for("@sdk/greet"), Some(key.key_id().as_str()));
    let lock = std::fs::read_to_string(dir.path().join("specforge.lock")).unwrap();
    assert!(
        lock.contains(&key.key_id()),
        "the lock records the key: {lock}"
    );
}

#[specforge_test(
    invariant = "publisher_trust",
    verify = "specforge add refuses a package signed by another key than the pinned one"
)]
fn add_refuses_a_package_signed_by_another_key_than_the_pinned_one() {
    use crate::fake_registry::{FakeRegistry, Package};
    let pinned = specforge_registry_client::SigningKey::generate();
    let other = specforge_registry_client::SigningKey::generate();
    let registry = FakeRegistry::serve(vec![
        Package::new("@sdk/greet", "0.1.0", greet_wasm()).signed_by(&other),
    ]);
    let dir = project_on(&registry);
    let home = TempDir::new().unwrap();
    let mut known = specforge_registry_client::KnownKeys::default();
    known.pin("@sdk/greet", &pinned.key_id());
    specforge_registry_client::save_known_keys_at(&known_keys(&home), &known).unwrap();

    let output = add_greet(&dir, &home, &["--allow-unsigned"]);

    assert_refused(&output, &dir, "R-TRUST-003");
    let known = specforge_registry_client::load_known_keys_at(&known_keys(&home));
    assert_eq!(known.pin_for("@sdk/greet"), Some(pinned.key_id().as_str()));
}

#[specforge_test(
    invariant = "publisher_trust",
    verify = "specforge add refuses an unsigned package without --allow-unsigned"
)]
fn add_refuses_an_unsigned_package_without_allow_unsigned() {
    use crate::fake_registry::{FakeRegistry, Package};
    let registry = FakeRegistry::serve(vec![Package::new("@sdk/greet", "0.1.0", greet_wasm())]);
    let dir = project_on(&registry);
    let home = TempDir::new().unwrap();

    let output = add_greet(&dir, &home, &[]);

    assert_refused(&output, &dir, "R-TRUST-001");
}

#[specforge_test(
    behavior = "verify_registry_integrity",
    verify = "mismatched SHA256 produces hard error"
)]
fn add_refuses_a_download_that_does_not_match_the_registry_sha256() {
    use crate::fake_registry::{FakeRegistry, Package};
    let key = specforge_registry_client::SigningKey::generate();
    let mut tampered = greet_wasm();
    tampered.push(0);
    let registry = FakeRegistry::serve(vec![
        Package::new("@sdk/greet", "0.1.0", greet_wasm())
            .signed_by(&key)
            .serving(tampered),
    ]);
    let dir = project_on(&registry);
    let home = TempDir::new().unwrap();

    let output = add_greet(&dir, &home, &["--allow-unsigned", "--yes"]);

    assert_refused(&output, &dir, "R-OPS-002");
    assert!(!known_keys(&home).exists(), "nothing is pinned");
}

// ---------------------------------------------------------------
// Guard: no source names specforge.dev
// ---------------------------------------------------------------

/// Files allowed to name `specforge.dev`, with the text every occurrence
/// there must contain. These are the schema URLs, which plan step 06·C6
/// moves to the canonical repository; that step deletes these rows (the
/// test fails on a row that matches nothing).
const SPECFORGE_DEV_ALLOWED: &[(&str, &str)] = &[
    (
        "crates/specforge-emitter/src/schema.rs",
        "\"https://specforge.dev/schema/graph-protocol-v{}.json\"",
    ),
    (
        "crates/specforge-ops/src/init.rs",
        "\"$schema\": \"https://specforge.dev/schema/specforge.json\"",
    ),
];

/// Replace `#[cfg(test)]` / `#[test]` items (brace-matched) with blank
/// lines, keeping line numbers.
fn strip_test_items(src: &str) -> Vec<&str> {
    let lines: Vec<&str> = src.lines().collect();
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with("#[cfg(test)]") || trimmed == "#[test]" {
            let mut j = i + 1;
            let mut depth: i64 = 0;
            let mut started = false;
            while j < lines.len() {
                for ch in lines[j].chars() {
                    match ch {
                        '{' => {
                            depth += 1;
                            started = true;
                        }
                        '}' => depth -= 1,
                        _ => {}
                    }
                }
                if (started && depth <= 0) || (!started && lines[j].trim_end().ends_with(';')) {
                    break;
                }
                j += 1;
            }
            let end = j.min(lines.len() - 1);
            out.extend(std::iter::repeat_n("", end - i + 1));
            i = end + 1;
            continue;
        }
        out.push(lines[i]);
        i += 1;
    }
    out
}

/// Every file under `dir`, skipping test directories and build output.
fn non_test_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    const SKIP: &[&str] = &["tests", "target", "node_modules", ".git"];
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if !SKIP.contains(&name.as_str()) {
                non_test_files(&path, files);
            }
        } else {
            files.push(path);
        }
    }
}

#[specforge_test(
    behavior = "configure_registries",
    verify = "no registry URL on the specforge.dev domain is compiled into non-test source"
)]
fn no_source_names_specforge_dev() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for top in ["crates", "extensions"] {
        non_test_files(&root.join(top), &mut files);
    }
    assert!(files.len() > 100, "scanner found {} files", files.len());

    let mut problems = Vec::new();
    let mut allowed_used = std::collections::BTreeSet::new();
    for path in files {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        // Binary files (the wasm blobs) are built from the sources read here.
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let file_name = rel.rsplit('/').next().unwrap_or_default();
        if file_name == "tests.rs" || file_name.ends_with("_tests.rs") {
            continue;
        }
        let lines = if rel.ends_with(".rs") {
            strip_test_items(&src)
        } else {
            src.lines().collect()
        };
        for (index, line) in lines.iter().enumerate() {
            if !line.contains("specforge.dev") {
                continue;
            }
            match SPECFORGE_DEV_ALLOWED
                .iter()
                .find(|(file, text)| *file == rel && line.contains(text))
            {
                Some(row) => {
                    allowed_used.insert(*row);
                }
                None => problems.push(format!("{rel}:{}: {}", index + 1, line.trim())),
            }
        }
    }
    for row in SPECFORGE_DEV_ALLOWED {
        if !allowed_used.contains(row) {
            problems.push(format!(
                "SPECFORGE_DEV_ALLOWED row {row:?} matches nothing: delete it"
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "SpecForge does not own specforge.dev (ADR 0004 N1); remove these:\n  {}",
        problems.join("\n  ")
    );
}
