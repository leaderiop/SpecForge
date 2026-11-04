//! Package coordinates, version ranges, OSV answers and research identifiers against published vectors.

use crate::coord::{self, Ecosystem, Release, licence, releases};
use crate::ids::{doi, doi_url, orcid, orcid_url};
use crate::osv::{Advisory, advisories, query};
use crate::ranges::{Requirement, newest, newest_unyanked};

#[test]
fn coord_names_each_ecosystem_as_its_registry_keys_it() {
    let crates = coord::parse("crates:Serde_Json@1.0").expect("a crate");
    assert_eq!(
        (crates.ecosystem, crates.name.as_str()),
        (Ecosystem::Crates, "serde_json")
    );
    assert_eq!(
        crates.index_url(),
        "https://index.crates.io/se/rd/serde_json"
    );
    assert_eq!(crates.requirement_at, Some(18..21));
    for (name, path) in [
        ("a", "1/a"),
        ("ab", "2/ab"),
        ("abc", "3/a/abc"),
        ("cargo", "ca/rg/cargo"),
    ] {
        let coord = coord::parse(&format!("crates:{name}")).expect("a crate");
        assert_eq!(coord.index_url(), format!("https://index.crates.io/{path}"));
        assert!(matches!(coord.requirement, Requirement::Any));
    }
    let scoped = coord::parse("npm:@types/node@^20").expect("a scoped package");
    assert_eq!(scoped.name, "@types/node");
    assert_eq!(
        scoped.index_url(),
        "https://registry.npmjs.org/@types%2fnode"
    );
    assert_eq!(
        scoped.release_url("20.1.0"),
        "https://registry.npmjs.org/@types%2fnode/20.1.0"
    );
    let pypi = coord::parse("pypi:Zope.Interface[docs]>=5,<7").expect("a PyPI project");
    assert_eq!(pypi.name, "zope-interface");
    assert_eq!(
        pypi.index_url(),
        "https://pypi.org/pypi/zope-interface/json"
    );
    assert_eq!(
        pypi.release_url("6.0"),
        "https://pypi.org/pypi/zope-interface/6.0/json"
    );
    let bare = coord::parse("pypi:requests").expect("no specifier");
    assert!(matches!(bare.requirement, Requirement::Any));
}

#[test]
fn coord_refuses_what_no_registry_names() {
    for (text, at) in [
        ("cargo:serde", 0..5),
        ("crates:1serde", 7..8),
        ("crates:se rde", 9..10),
        ("npm:React", 4..9),
        ("npm:@scope", 4..10),
        ("pypi:-requests", 5..14),
        ("crates:serde@>=>1", 13..17),
    ] {
        let invalid = coord::parse(text).expect_err(text);
        assert_eq!(invalid.at, Some(at), "{text}: {}", invalid.reason);
    }
    assert!(coord::parse("serde").is_err());
}

/// Whether `requirement` of `ecosystem` admits `version`.
fn admits(ecosystem: Ecosystem, requirement: &str, version: &str) -> bool {
    Requirement::parse(ecosystem, requirement)
        .unwrap_or_else(|why| panic!("{requirement}: {why}"))
        .matches(ecosystem, version)
}

// Cargo requirements, from the examples of the Cargo book's "Specifying dependencies" and `semver`'s documentation.
const CARGO: [(&str, &str, bool); 12] = [
    ("1.2.3", "1.9.0", true),
    ("1.2.3", "2.0.0", false),
    ("^0.2.3", "0.2.9", true),
    ("^0.2.3", "0.3.0", false),
    ("~1.2", "1.2.9", true),
    ("~1.2", "1.3.0", false),
    ("1.*", "1.7.0", true),
    (">=1.2, <1.5", "1.5.0", false),
    ("=1.2.3", "1.2.3", true),
    ("1.2.3", "1.3.0-alpha.1", false),
    (">=1.3.0-alpha.1", "1.3.0-alpha.2", true),
    ("*", "0.0.1", true),
];

// node-semver's test/fixtures/range-include.js and range-exclude.js (a selection, ISC License,
// Copyright (c) Isaac Z. Schlueter and Contributors).
const NPM: [(&str, &str, bool); 16] = [
    ("1.0.0 - 2.0.0", "1.2.3", true),
    ("^1.2.3+build", "1.3.0", true),
    ("1.2.3 - 2.3.4", "2.3.5", false),
    (">=*", "0.2.4", true),
    ("", "1.0.0", true),
    ("*", "1.2.3-foo", false),
    ("~1.2.1 >=1.2.3", "1.2.3", true),
    ("~1.2.1 =1.2.3", "1.2.3", true),
    ("||", "1.3.4", true),
    ("2.x.x", "2.1.3", true),
    ("1.2.x || 2.x", "3.1.3", false),
    ("~0.0.1", "0.1.0", false),
    ("^0.1", "0.2.0", false),
    ("^1.2.3-beta.2", "1.2.3-beta.4", true),
    ("^1.2.3-beta.2", "1.2.4-beta.2", false),
    ("<1.2.3", "1.2.3-beta", false),
];

// packaging's tests/test_specifiers.py (a selection, BSD-2-Clause or Apache-2.0, Copyright (c) Donald Stufft and
// individual contributors).
const PYPI: [(&str, &str, bool); 14] = [
    ("==2.0", "2.0", true),
    ("==2.*", "2.1", true),
    ("==2.0", "2.1", false),
    ("!=2.0", "2.1", true),
    ("~=2.2", "2.3", true),
    ("~=2.2", "3.0", false),
    ("~=2.2.0", "2.3", false),
    (">=2", "2.1", true),
    ("<2", "2.0b1", false),
    ("<2.0.dev1", "2.0a1", false),
    (">=2.0.dev1", "2.0a1", true),
    (">2", "2.0.post1", false),
    (">=1.0", "1.0.post1", true),
    ("===1.0", "1.0", true),
];

#[test]
fn ranges_follow_each_ecosystem_vectors() {
    for (ecosystem, cases) in [
        (Ecosystem::Crates, &CARGO[..]),
        (Ecosystem::Npm, &NPM[..]),
        (Ecosystem::Pypi, &PYPI[..]),
    ] {
        for (requirement, version, expected) in cases {
            assert_eq!(
                admits(ecosystem, requirement, version),
                *expected,
                "{ecosystem:?} {requirement} {version}"
            );
        }
    }
}

#[test]
fn ranges_pick_the_newest_release_and_skip_the_yanked() {
    let listed = |versions: &[(&str, bool)]| -> Vec<Release> {
        versions
            .iter()
            .map(|(version, yanked)| Release {
                version: (*version).to_string(),
                yanked: *yanked,
            })
            .collect()
    };
    let crates = listed(&[
        ("1.0.0", false),
        ("1.2.0", true),
        ("1.1.0", false),
        ("2.0.0-rc.1", false),
        ("x", false),
    ]);
    let req = Requirement::parse(Ecosystem::Crates, "1").expect("a requirement");
    assert_eq!(
        newest(Ecosystem::Crates, &req, &crates).map(|r| r.version.as_str()),
        Some("1.1.0")
    );
    let only_yanked = Requirement::parse(Ecosystem::Crates, "=1.2.0").expect("a requirement");
    assert_eq!(newest(Ecosystem::Crates, &only_yanked, &crates), None);
    assert_eq!(
        newest_unyanked(Ecosystem::Crates, &crates).map(|r| r.version.as_str()),
        Some("2.0.0-rc.1")
    );
    assert_eq!(
        newest(Ecosystem::Crates, &Requirement::Any, &crates).map(|r| r.version.as_str()),
        Some("1.1.0")
    );
    // PEP 440 orders 1.10 after 1.9, and a post-release after its release.
    let pypi = listed(&[
        ("1.9", false),
        ("1.10", false),
        ("1.10.post1", false),
        ("2.0rc1", false),
    ]);
    let any = Requirement::Any;
    assert_eq!(
        newest(Ecosystem::Pypi, &any, &pypi).map(|r| r.version.as_str()),
        Some("1.10.post1")
    );
}

#[test]
fn releases_read_each_index_answer() {
    let crates = b"{\"name\":\"a\",\"vers\":\"1.0.0\",\"deps\":[],\"yanked\":false}\n{\"name\":\"a\",\"vers\":\"1.1.0\",\"yanked\":true}\n";
    assert_eq!(
        releases(Ecosystem::Crates, crates).expect("the sparse index"),
        [
            Release {
                version: "1.0.0".into(),
                yanked: false
            },
            Release {
                version: "1.1.0".into(),
                yanked: true
            }
        ]
    );
    let npm = br#"{"name":"react","dist-tags":{"latest":"18.2.0"},"versions":{"18.2.0":{"dist":{"tarball":"x"}},"17.0.2":{}}}"#;
    let versions: Vec<String> = releases(Ecosystem::Npm, npm)
        .expect("a packument")
        .into_iter()
        .map(|r| r.version)
        .collect();
    assert_eq!(versions, ["17.0.2", "18.2.0"]);
    let pypi = br#"{"info":{"name":"requests"},"releases":{
        "2.30.0":[{"filename":"requests-2.30.0-py3-none-any.whl","yanked":true},{"filename":"requests-2.30.0.tar.gz","yanked":true}],
        "2.31.0":[{"filename":"requests-2.31.0.tar.gz","yanked":false},{"filename":"x.whl","yanked":true}],
        "2.32.0":[]}}"#;
    let read = releases(Ecosystem::Pypi, pypi).expect("the JSON API");
    let yanked: Vec<(&str, bool)> = read
        .iter()
        .map(|r| (r.version.as_str(), r.yanked))
        .collect();
    assert_eq!(
        yanked,
        [("2.30.0", true), ("2.31.0", false), ("2.32.0", false)]
    );
    assert!(releases(Ecosystem::Npm, b"<html>").is_err());
}

#[test]
fn licence_reads_what_each_release_record_states() {
    let crates = br#"{"version":{"num":"1.0.0","license":"MIT OR Apache-2.0"}}"#;
    assert_eq!(
        licence(Ecosystem::Crates, crates).expect("read").as_deref(),
        Some("MIT OR Apache-2.0")
    );
    let old_npm = br#"{"license":{"type":"MIT","url":"x"}}"#;
    assert_eq!(
        licence(Ecosystem::Npm, old_npm).expect("read").as_deref(),
        Some("MIT")
    );
    let expression = br#"{"info":{"license_expression":"Apache-2.0","license":"whatever"}}"#;
    assert_eq!(
        licence(Ecosystem::Pypi, expression)
            .expect("read")
            .as_deref(),
        Some("Apache-2.0")
    );
    let classifier = br#"{"info":{"license":"see LICENSE","classifiers":["License :: OSI Approved :: MIT License"]}}"#;
    assert_eq!(
        licence(Ecosystem::Pypi, classifier)
            .expect("read")
            .as_deref(),
        Some("MIT")
    );
    let unstated = br#"{"info":{"license":"","classifiers":[]}}"#;
    assert_eq!(licence(Ecosystem::Pypi, unstated).expect("read"), None);
}

#[test]
fn osv_drops_withdrawn_advisories_and_names_the_first_fix() {
    let body = br#"{"vulns":[
        {"id":"GHSA-1","aliases":["CVE-2024-1"],"affected":[{"ranges":[{"type":"SEMVER","events":[{"introduced":"0"},{"fixed":"1.2.3"}]}]}]},
        {"id":"GHSA-2","withdrawn":"2024-01-01T00:00:00Z"}]}"#;
    assert_eq!(
        advisories(body).expect("an answer"),
        [Advisory {
            id: "GHSA-1".into(),
            aliases: vec!["CVE-2024-1".into()],
            fixed: Some("1.2.3".into())
        }]
    );
    assert_eq!(advisories(b"{}").expect("no vuln"), []);
    let query: serde_json::Value =
        serde_json::from_str(&query(Ecosystem::Pypi, "requests", "2.31.0")).expect("JSON");
    assert_eq!(query["package"]["ecosystem"], "PyPI");
}

#[test]
fn ids_read_dois_and_orcids_in_every_spelling() {
    for written in [
        "10.1000/182",
        "doi:10.1000/182",
        "https://doi.org/10.1000/182",
        "HTTP://DX.DOI.ORG/10.1000/182",
    ] {
        assert_eq!(doi(written).as_deref(), Ok("10.1000/182"), "{written}");
    }
    assert_eq!(doi("10.1002/ABC.123").as_deref(), Ok("10.1002/abc.123"));
    assert_eq!(
        doi_url("10.1000/a b#c"),
        "https://doi.org/api/handles/10.1000/a%20b%23c"
    );
    for (bad, at) in [
        ("11.1000/182", 0..7),
        ("10.abc/1", 0..6),
        ("10.1000/", 7..8),
        ("10.1000", 0..7),
    ] {
        assert_eq!(doi(bad).expect_err(bad).at, Some(at), "{bad}");
    }
    // ORCID's own example iD, and the one from its check-digit documentation that ends in X.
    assert_eq!(
        orcid("0000-0002-1825-0097").as_deref(),
        Ok("0000-0002-1825-0097")
    );
    assert_eq!(
        orcid("https://orcid.org/0000-0002-1694-233X").as_deref(),
        Ok("0000-0002-1694-233X")
    );
    assert_eq!(
        orcid_url("0000-0002-1825-0097"),
        "https://pub.orcid.org/v3.0/0000-0002-1825-0097/record"
    );
    assert!(orcid("0000-0002-1825-0098").is_err(), "a wrong check digit");
    assert!(orcid("0000000218250097").is_err(), "no hyphens");
}
