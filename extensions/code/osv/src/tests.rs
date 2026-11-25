use specforge_code_facts::vulnerability_facts;
use specforge_extension_sdk::prelude::Fact;

use super::*;

/// An osv-scanner report, hand-written to its JSON output and the OSV schema: a root `Cargo.lock` of `/project`.
const SCAN: &str = include_str!("../fixtures/osv-scanner.json");
/// A scan of a virtual workspace's root `Cargo.lock` and of a nested npm `web/package-lock.json`, hand-written.
const WORKSPACE: &str = include_str!("../fixtures/workspace.json");
/// osv-scanner 2.7.1's report of `/project`'s `Cargo.lock` and `web/package-lock.json`, recorded with real records.
const OSV_SCANNER_2_7_1: &str = include_str!("../fixtures/osv-scanner-2.7.1.json");
/// The served `package` facts a vulnerability collector receives, as JSON.
const PACKAGES: &str = include_str!("../fixtures/packages.json");

fn served() -> Vec<Fact> {
    serde_json::from_str(PACKAGES).expect("the served facts parse")
}

fn text<'f>(fact: &'f Fact, name: &str) -> Option<&'f str> {
    fact.fields.get(name).and_then(Value::as_str)
}

#[test]
fn a_scan_mints_vulnerabilities_citing_the_lockfiles_package() {
    let report = read(SCAN, "/project").expect("the scan reads");
    let minted = vulnerability_facts(&report, &served());
    assert!(minted.skipped.is_empty(), "{:?}", minted.skipped);
    let facts = minted.facts;
    let read_out: Vec<(&str, Option<&str>, Option<&str>)> = facts
        .iter()
        .map(|f| {
            assert!(f.id.starts_with("vuln:pkg:cargo/demo/"), "{}", f.id);
            (
                text(f, "package").unwrap_or_default(),
                text(f, "osv_id"),
                text(f, "severity"),
            )
        })
        .collect();
    assert_eq!(
        read_out,
        [
            (
                "pkg:cargo/demo",
                Some("GHSA-7v4m-2x9q-3h8c"),
                Some("medium")
            ),
            ("pkg:cargo/demo", Some("RUSTSEC-2099-0002"), Some("unknown")),
        ]
    );
    insta::assert_json_snapshot!("osv_scanner", facts);
}

#[test]
fn a_scan_reads_its_lockfile_relative_and_each_records_purl_for_its_package() {
    let report = read(SCAN, "/project").expect("the scan reads");
    assert_eq!(report.sources.len(), 1);
    let source = &report.sources[0];
    assert_eq!(source.lockfile, "Cargo.lock");
    let smallvec = &source.packages[0].records[0];
    assert_eq!(smallvec.purl.as_deref(), Some("pkg:cargo/smallvec"));
    assert_eq!(smallvec.label.as_deref(), Some("MODERATE"));
    assert_eq!(smallvec.aliases, ["RUSTSEC-2099-0001", "CVE-2099-0001"]);
    let time = &source.packages[1].records[0];
    assert_eq!((time.purl.as_deref(), time.label.as_deref()), (None, None));
    assert!(time.scores.is_empty());
}

#[test]
fn a_workspace_lockfile_reaches_every_member_and_a_nested_one_its_own_package() {
    let report = read(WORKSPACE, "/project").expect("the scan reads");
    let lockfiles: Vec<&str> = report.sources.iter().map(|s| s.lockfile.as_str()).collect();
    assert_eq!(lockfiles, ["Cargo.lock", "web/package-lock.json"]);
    // A virtual workspace has no root package.
    let served: Vec<Fact> = served()
        .into_iter()
        .filter(|f| f.id != "pkg:cargo/demo")
        .collect();
    let minted = vulnerability_facts(&report, &served);
    assert!(minted.skipped.is_empty(), "{:?}", minted.skipped);
    let facts = minted.facts;
    let read_out: Vec<(&str, &str, &str)> = facts
        .iter()
        .map(|f| {
            (
                text(f, "package").unwrap_or_default(),
                text(f, "osv_id").unwrap_or_default(),
                text(f, "severity").unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        read_out,
        [
            ("pkg:cargo/ledger", "RUSTSEC-2099-0003", "high"),
            ("pkg:cargo/ledger", "RUSTSEC-2099-0004", "unknown"),
            ("pkg:cargo/vault", "RUSTSEC-2099-0003", "high"),
            ("pkg:cargo/vault", "RUSTSEC-2099-0004", "unknown"),
            ("pkg:npm/%40demo/web", "GHSA-m3p6-8x4q-2c7v", "critical"),
        ]
    );
    let web = &facts[4];
    assert_eq!(
        web.fields["purl"],
        serde_json::json!(["pkg:npm/%40babel/traverse@7.22.0"])
    );
    insta::assert_json_snapshot!("workspace", facts);
}

#[test]
fn a_recorded_osv_scanner_2_7_1_report_mints_real_records_of_both_lockfiles() {
    let report = read(OSV_SCANNER_2_7_1, "/project").expect("the recording reads");
    let lockfiles: Vec<&str> = report.sources.iter().map(|s| s.lockfile.as_str()).collect();
    assert_eq!(lockfiles, ["Cargo.lock", "web/package-lock.json"]);
    let minted = vulnerability_facts(&report, &served());
    assert!(minted.skipped.is_empty(), "{:?}", minted.skipped);
    let packages: std::collections::BTreeSet<&str> = minted
        .facts
        .iter()
        .map(|f| text(f, "package").unwrap_or_default())
        .collect();
    assert_eq!(
        packages,
        ["pkg:cargo/demo", "pkg:npm/%40demo/web"]
            .into_iter()
            .collect()
    );
    insta::assert_json_snapshot!("osv_scanner_2_7_1", minted.facts);
}

#[test]
fn an_error_shows_the_reports_text_bounded_and_escaped() {
    let hostile = format!("\u{1b}]0;{}", "a".repeat(5_000));
    let smallvec =
        || serde_json::json!({"name": "smallvec", "version": "1.6.0", "ecosystem": "crates.io"});
    let record = || serde_json::json!({"id": "RUSTSEC-2099-0001"});
    for source in [
        serde_json::json!({"path": hostile, "type": "docker"}),
        serde_json::json!({"path": "/project/Cargo.lock", "type": hostile}),
        serde_json::json!({"path": format!("/elsewhere/{hostile}"), "type": "lockfile"}),
    ] {
        let error = read(&scan(source, smallvec(), record()), "/project").expect_err("refused");
        assert!(
            !error.contains('\u{1b}') && error.contains("\\u{1b}"),
            "{error}"
        );
        assert!(error.len() < 400, "{} bytes", error.len());
    }
}

#[test]
fn a_polyglot_scan_keeps_its_cargo_findings_and_names_what_it_skips() {
    let text = serde_json::json!({"results": [
        {"source": {"path": "/project/Cargo.lock", "type": "lockfile"}, "packages": [
            {"package": {"name": "smallvec", "version": "1.6.0", "ecosystem": "crates.io"},
             "vulnerabilities": [{"id": "RUSTSEC-2099-0001"}]}]},
        {"source": {"path": "/project/requirements.txt", "type": "lockfile"}, "packages": [
            {"package": {"name": "django", "version": "4.0", "ecosystem": "PyPI"},
             "vulnerabilities": [{"id": "PYSEC-2099-1"}]}]},
        {"source": {"path": "/project/api/package-lock.json", "type": "lockfile"}, "packages": [
            {"package": {"name": "lodash", "version": "4.17.20", "ecosystem": "npm"},
             "vulnerabilities": [{"id": "GHSA-0000-0000-0000"}]}]}
    ]})
    .to_string();
    // Only the cargo package at the root: no npm package lies at or under `api`.
    let served: Vec<Fact> = served()
        .into_iter()
        .filter(|f| f.id == "pkg:cargo/demo")
        .collect();
    let minted = vulnerability_facts(&read(&text, "/project").expect("the scan reads"), &served);
    let ids: Vec<&str> = minted.facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["vuln:pkg:cargo/demo/RUSTSEC-2099-0001"]);
    assert_eq!(minted.skipped.len(), 2, "{:?}", minted.skipped);
    assert!(
        minted.skipped[0].contains("PyPI") && minted.skipped[0].contains("requirements.txt"),
        "{}",
        minted.skipped[0]
    );
    assert!(
        minted.skipped[1].contains("api/package-lock.json"),
        "{}",
        minted.skipped[1]
    );
}

/// A minimal scan of one `source` holding one `package` of `ecosystem`, its record `vulnerability`.
fn scan(source: Value, package: Value, vulnerability: Value) -> String {
    serde_json::json!({
        "results": [{
            "source": source,
            "packages": [{"package": package, "vulnerabilities": [vulnerability]}]
        }]
    })
    .to_string()
}

#[test]
fn a_bad_scan_is_an_error_naming_its_value_or_key_and_mints_nothing() {
    let lockfile = || serde_json::json!({"path": "/project/Cargo.lock", "type": "lockfile"});
    let smallvec =
        || serde_json::json!({"name": "smallvec", "version": "1.6.0", "ecosystem": "crates.io"});
    let record = || serde_json::json!({"id": "RUSTSEC-2099-0001"});
    let rows: [(&str, String, &[&str]); 8] = [
        ("no results", "{}".to_string(), &["results"]),
        (
            "results no array",
            "{\"results\": {}}".to_string(),
            &["results"],
        ),
        (
            "source type docker",
            scan(
                serde_json::json!({"path": "/project/Dockerfile", "type": "docker"}),
                smallvec(),
                record(),
            ),
            &["docker", "/project/Dockerfile"],
        ),
        (
            "a path outside the project",
            scan(
                serde_json::json!({"path": "/elsewhere/Cargo.lock", "type": "lockfile"}),
                smallvec(),
                record(),
            ),
            &["/elsewhere/Cargo.lock"],
        ),
        (
            "no source.path",
            scan(
                serde_json::json!({"type": "lockfile"}),
                smallvec(),
                record(),
            ),
            &["results[0].source.path"],
        ),
        (
            "a package with no version",
            scan(
                lockfile(),
                serde_json::json!({"name": "smallvec", "ecosystem": "crates.io"}),
                record(),
            ),
            &["results[0].packages[0].package.version"],
        ),
        (
            "a vulnerability with no id",
            scan(lockfile(), smallvec(), serde_json::json!({"summary": "x"})),
            &["results[0].packages[0].vulnerabilities[0].id"],
        ),
        ("no JSON", "{\"results\":".to_string(), &["no JSON"]),
    ];
    for (case, text, named) in rows {
        let error = read(&text, "/project").expect_err(case);
        for name in named {
            assert!(error.contains(name), "{case}: {error}");
        }
    }
}
