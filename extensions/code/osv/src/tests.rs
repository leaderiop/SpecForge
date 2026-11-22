use specforge_code_facts::vulnerability_facts;
use specforge_extension_sdk::prelude::Fact;

use super::*;

/// An osv-scanner report, hand-written to its JSON output and the OSV schema: a root `Cargo.lock` of `/project`.
const SCAN: &str = include_str!("../fixtures/osv-scanner.json");
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
    let facts = vulnerability_facts(&report, &served()).expect("the lockfile is a package's");
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
