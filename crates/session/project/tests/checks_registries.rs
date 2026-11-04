//! `@specforge/checks`' registries family as a project runs it: the committed blob asking each identifier's resolver,
//! served by the fake network's routes.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_common::Diagnostic;
use specforge_effects::EffectsMode;
use specforge_effects::testing::{FakeHttp, MemoryConsent};
use specforge_project::testing::TestProject;

use crate::value_probes::Scene;

/// `refs` (each quoted) checked by `checker` with `args`, over `http`.
fn scene(refs: &[&str], checker: &str, args: Value, http: FakeHttp) -> Scene {
    let items: Vec<String> = refs.iter().map(|r| format!("\"{r}\"")).collect();
    let checker = format!("@specforge/checks:{checker}");
    let dir = TestProject::new()
        .config(|c| {
            c["checks"] = json!({ "fields": { "feature.refs": [{ "checker": checker, "args": args }] } });
        })
        .enabling(&["@specforge/product", "@specforge/checks"])
        .file(
            "a.spec",
            format!(
                "feature f \"F\" {{\n  status planned\n  problem \"p\"\n  solution \"s\"\n  refs [{}]\n}}\n",
                items.join(", ")
            ),
        );
    Scene {
        dir,
        runtime: specforge_project::testing::component_runtime(),
        http: Arc::new(http),
    }
}

fn coded(diagnostics: &[Diagnostic], code: &str) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.code == code)
        .cloned()
        .collect()
}

const SERDE_INDEX: &str = "https://index.crates.io/se/rd/serde";
const SERDE_LINES: &str = "{\"name\":\"serde\",\"vers\":\"1.0.0\",\"yanked\":false}\n\
                           {\"name\":\"serde\",\"vers\":\"1.0.1\",\"yanked\":true}\n";
const OSV: &str = "https://api.osv.dev/v1/query";

#[specforge_test_macros::test(
    behavior = "checks_registries",
    verify = "a requirement only yanked versions meet is E105 naming the newest"
)]
fn a_requirement_only_yanked_versions_meet_is_e105_naming_the_newest() {
    let http = FakeHttp::new().route(SERDE_INDEX, 200, SERDE_LINES);
    let scene = scene(
        &["crates:serde@1", "crates:serde@=1.0.1"],
        "package",
        json!({}),
        http,
    );
    let found = scene.online().diagnostics();
    let missing = coded(&found, "E105");
    assert_eq!(missing.len(), 1, "{found:?}");
    assert!(
        missing[0]
            .message
            .contains("the newest release not yanked is 1.0.0"),
        "{}",
        missing[0].message
    );
    assert!(!missing[0].machine);
}

#[specforge_test_macros::test(
    behavior = "checks_registries",
    verify = "a registry 503 is W181, never E105"
)]
fn a_registry_503_is_w181_never_e105() {
    let http = FakeHttp::new()
        .route("https://registry.npmjs.org/react", 503, "")
        .route("https://pypi.org/pypi/requests/json", 429, "")
        .route("https://index.crates.io/no/su/nosuch", 404, "");
    let scene = scene(
        &["npm:react@^18", "pypi:requests>=2", "crates:nosuch@1"],
        "package",
        json!({}),
        http,
    );
    let found = scene.online().diagnostics();
    let outages = coded(&found, "W181");
    assert_eq!(outages.len(), 2, "{found:?}");
    assert!(outages.iter().all(|d| d.machine), "{outages:?}");
    let gone = coded(&found, "E105");
    assert_eq!(gone.len(), 1, "{found:?}");
    assert!(
        gone[0].message.contains("crates has no package 'nosuch'"),
        "{}",
        gone[0].message
    );
    assert!(!gone[0].machine, "a definite answer about the package");
}

#[specforge_test_macros::test(
    behavior = "checks_registries",
    verify = "an OSV hit is W179 with its aliases, and a withdrawn one is not"
)]
fn an_osv_hit_is_w179_with_its_aliases_and_a_withdrawn_one_is_not() {
    let answer = json!({ "vulns": [
        { "id": "RUSTSEC-2099-0001", "aliases": ["CVE-2099-1", "GHSA-xxxx"],
          "affected": [{ "ranges": [{ "type": "SEMVER", "events": [{ "introduced": "0" }, { "fixed": "1.0.2" }] }] }] },
        { "id": "RUSTSEC-2099-0002", "withdrawn": "2099-01-01T00:00:00Z" }
    ] });
    let http =
        FakeHttp::new()
            .route(SERDE_INDEX, 200, SERDE_LINES)
            .route(OSV, 200, answer.to_string());
    let scene = scene(&["crates:serde@1"], "advisory", json!({}), http);
    let found = scene.online().diagnostics();
    let advisories = coded(&found, "W179");
    assert_eq!(advisories.len(), 1, "{found:?}");
    let message = &advisories[0].message;
    assert!(
        message.contains("serde 1.0.0 has the advisory RUSTSEC-2099-0001 (CVE-2099-1, GHSA-xxxx)"),
        "{message}"
    );
    assert!(message.contains("fixed in 1.0.2"), "{message}");
    let posted: Vec<_> = scene
        .http
        .requests()
        .into_iter()
        .filter(|r| r.url == OSV)
        .collect();
    assert_eq!(posted.len(), 1);
    assert_eq!(posted[0].method, "POST");
    let body: Value = serde_json::from_slice(posted[0].body.as_deref().unwrap_or_default())
        .expect("a JSON query");
    assert_eq!(body["version"], "1.0.0");
}

#[specforge_test_macros::test(
    behavior = "checks_registries",
    verify = "a licence outside allow is W179"
)]
fn a_licence_outside_allow_is_w179() {
    let npm = json!({ "versions": { "18.2.0": {}, "17.0.2": {} } });
    let pypi = json!({ "releases": { "2.31.0": [{ "filename": "requests-2.31.0.tar.gz", "yanked": false }] } });
    let http = FakeHttp::new()
        .route("https://registry.npmjs.org/react", 200, npm.to_string())
        .route(
            "https://registry.npmjs.org/react/18.2.0",
            200,
            json!({ "license": "MIT" }).to_string(),
        )
        .route("https://pypi.org/pypi/requests/json", 200, pypi.to_string())
        .route(
            "https://pypi.org/pypi/requests/2.31.0/json",
            200,
            json!({ "info": { "license_expression": "Apache-2.0" } }).to_string(),
        );
    let scene = scene(
        &["npm:react@^18", "pypi:requests>=2"],
        "license_of",
        json!({ "allow": ["MIT"] }),
        http,
    );
    let found = scene.online().diagnostics();
    let licences = coded(&found, "W179");
    assert_eq!(licences.len(), 1, "{found:?}");
    assert!(
        licences[0]
            .message
            .contains("requests 2.31.0 is licensed Apache-2.0"),
        "{}",
        licences[0].message
    );
}

#[specforge_test_macros::test(
    behavior = "checks_registries",
    verify = "an unknown DOI is E105 and a malformed one is E103"
)]
fn an_unknown_doi_is_e105_and_a_malformed_one_is_e103() {
    let http = FakeHttp::new()
        .route(
            "https://doi.org/api/handles/10.1000/182",
            200,
            r#"{"responseCode":1}"#,
        )
        .route(
            "https://doi.org/api/handles/10.9999/none",
            404,
            r#"{"responseCode":100}"#,
        );
    let scene = scene(
        &["https://doi.org/10.1000/182", "10.9999/NONE", "11.1000/182"],
        "doi",
        json!({}),
        http,
    );
    let found = scene.online().diagnostics();
    let unknown = coded(&found, "E105");
    assert_eq!(unknown.len(), 1, "{found:?}");
    assert!(
        unknown[0]
            .message
            .contains("'10.9999/none' is not registered"),
        "{}",
        unknown[0].message
    );
    assert_eq!(coded(&found, "E103").len(), 1, "{found:?}");
    assert_eq!(
        scene.http.requests().len(),
        2,
        "a malformed DOI is not asked"
    );
}

#[specforge_test_macros::test(
    behavior = "checks_registries",
    verify = "the registries scope asks only for its seven hosts"
)]
fn the_registries_scope_asks_only_for_its_seven_hosts() {
    let scene = scene(&["crates:serde@1"], "package", json!({}), FakeHttp::new());
    let found = scene
        .checked(EffectsMode::Auto, &MemoryConsent::default())
        .diagnostics();
    let asked = coded(&found, "W170");
    assert_eq!(asked.len(), 1, "{found:?}");
    let message = &asked[0].message;
    assert!(message.contains("'registries'"), "{message}");
    for host in [
        "index.crates.io",
        "crates.io",
        "registry.npmjs.org",
        "pypi.org",
        "api.osv.dev",
        "doi.org",
        "pub.orcid.org",
    ] {
        assert!(message.contains(host), "{host}: {message}");
    }
    assert!(message.contains("POST (query)"), "{message}");
    assert!(
        !message.contains("'links'") && !message.contains("'media-remote'"),
        "{message}"
    );
    assert!(scene.http.requests().is_empty());
}
