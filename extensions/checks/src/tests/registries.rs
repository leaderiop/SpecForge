//! The registries family: what each checker asks of which registry, and what it finds in canned answers.

use serde_json::json;
use specforge_extension_sdk::testing::MockHost;
use specforge_protocol_types::{
    ByteRange, CheckFinding, OutcomeClass, OutcomeData, PlannedItem, Probe, ProbeOutcome,
    Unreachable,
};
use specforge_test::prelude::*;

use super::{finding, judged, planned};

fn http(class: OutcomeClass, status: u16, body: &str) -> ProbeOutcome {
    ProbeOutcome {
        probe: Probe::HttpHead {
            url: String::new(),
            redirects: 0,
        },
        class,
        data: OutcomeData::Http {
            status: Some(status),
            final_url: String::new(),
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
            truncated: false,
            start: 0,
            total_len: None,
        },
        observed_at: 0,
        from_cache: false,
    }
}

fn ok(body: &str) -> ProbeOutcome {
    http(OutcomeClass::Ok, 200, body)
}

fn status(code: u16) -> ProbeOutcome {
    http(OutcomeClass::Status(code), code, "")
}

fn codes(findings: &[CheckFinding]) -> Vec<&str> {
    findings.iter().map(|f| f.code.as_str()).collect()
}

const SERDE_INDEX: &str = "{\"name\":\"serde\",\"vers\":\"1.0.0\",\"yanked\":false}\n\
                           {\"name\":\"serde\",\"vers\":\"1.0.1\",\"yanked\":true}\n";

fn round(
    checker: &str,
    text: &str,
    args: serde_json::Value,
    outcomes: Vec<ProbeOutcome>,
    carry: Option<serde_json::Value>,
) -> PlannedItem {
    MockHost::new(crate::specforge_extension_build())
        .judge_round(checker, text, args, outcomes, carry)
}

#[specforge_test(
    behavior = "checks_registries",
    verify = "a package asks its ecosystem's index, and a coordinate no registry names is E103"
)]
fn a_package_asks_its_ecosystems_index() {
    for (text, url) in [
        ("crates:serde@1", "https://index.crates.io/se/rd/serde"),
        ("npm:react@^18", "https://registry.npmjs.org/react"),
        ("pypi:Requests>=2", "https://pypi.org/pypi/requests/json"),
    ] {
        assert_eq!(
            planned("package", text, json!({})).probes,
            [Probe::HttpGet {
                url: url.into(),
                range: ByteRange::Whole {
                    max: 16 * 1024 * 1024
                },
            }],
            "{text}"
        );
    }
    let refused = finding("package", "cargo:serde", json!({})).expect("refused");
    assert_eq!((refused.code.as_str(), refused.at), ("E103", Some([0, 5])));
}

#[specforge_test(
    behavior = "checks_registries",
    verify = "a requirement only yanked releases meet is E105 on the requirement, naming the newest release"
)]
fn a_requirement_only_yanked_releases_meet_is_e105() {
    assert!(judged("package", "crates:serde@1", json!({}), ok(SERDE_INDEX)).is_empty());
    let yanked = judged("package", "crates:serde@=1.0.1", json!({}), ok(SERDE_INDEX));
    assert_eq!(codes(&yanked), ["E105"]);
    assert_eq!(yanked[0].at, Some([13, 19]));
    assert!(
        yanked[0]
            .message
            .contains("the newest release not yanked is 1.0.0"),
        "{}",
        yanked[0].message
    );
    let missing = judged("package", "crates:serde@1", json!({}), status(404));
    assert_eq!(codes(&missing), ["E105"]);
    for (outcome, code) in [
        (status(503), "W181"),
        (status(429), "W181"),
        (status(403), "W182"),
        (
            http(OutcomeClass::Unreachable(Unreachable::Tls), 0, ""),
            "W172",
        ),
        (ok("not json"), "W172"),
    ] {
        assert_eq!(
            codes(&judged("package", "crates:serde@1", json!({}), outcome)),
            [code]
        );
    }
}

#[specforge_test(
    behavior = "checks_registries",
    verify = "an advisory check queries OSV for the release the requirement resolves to, and reports each advisory as W179"
)]
fn an_advisory_check_queries_osv_for_the_resolved_release() {
    let first = round(
        "advisory",
        "crates:serde@1",
        json!({}),
        vec![ok(SERDE_INDEX)],
        None,
    );
    let [Probe::HttpPostQuery { url, json_body }] = &first.probes[..] else {
        panic!("one query: {:?}", first.probes);
    };
    assert_eq!(url, "https://api.osv.dev/v1/query");
    let body: serde_json::Value = serde_json::from_str(json_body).expect("JSON");
    assert_eq!(
        body,
        json!({"version": "1.0.0", "package": {"name": "serde", "ecosystem": "crates.io"}})
    );
    assert_eq!(first.carry, Some(json!("1.0.0")));
    let osv = ok(
        r#"{"vulns":[{"id":"RUSTSEC-1","aliases":["CVE-1"],"affected":[{"ranges":[{"events":[{"fixed":"1.0.2"}]}]}]},{"id":"RUSTSEC-2","withdrawn":"2020-01-01T00:00:00Z"}]}"#,
    );
    let second = round(
        "advisory",
        "crates:serde@1",
        json!({}),
        vec![ok(SERDE_INDEX), osv],
        first.carry,
    );
    assert_eq!(codes(&second.findings), ["W179"]);
    assert!(
        second.findings[0]
            .message
            .contains("serde 1.0.0 has the advisory RUSTSEC-1 (CVE-1); fixed in 1.0.2"),
        "{}",
        second.findings[0].message
    );
}

#[specforge_test(
    behavior = "checks_registries",
    verify = "a licence the release record states outside allow, or none, is W179"
)]
fn a_licence_outside_allow_is_w179() {
    let first = round(
        "license_of",
        "crates:serde@1",
        json!({"allow": ["MIT"]}),
        vec![ok(SERDE_INDEX)],
        None,
    );
    assert!(matches!(
        &first.probes[..],
        [Probe::HttpGet { url, .. }] if url == "https://crates.io/api/v1/crates/serde/1.0.0"
    ));
    let licensed = |licence: &str, allow: serde_json::Value| {
        let record = ok(&format!("{{\"version\":{{\"license\":{licence}}}}}"));
        round(
            "license_of",
            "crates:serde@1",
            json!({"allow": allow}),
            vec![ok(SERDE_INDEX), record],
            first.carry.clone(),
        )
        .findings
    };
    assert!(licensed("\"MIT OR Apache-2.0\"", json!(["MIT"])).is_empty());
    let gpl = licensed("\"GPL-3.0-only\"", json!(["MIT", "Apache-2.0"]));
    assert_eq!(codes(&gpl), ["W179"]);
    assert!(
        gpl[0].message.contains("is licensed GPL-3.0-only"),
        "{}",
        gpl[0].message
    );
    assert_eq!(codes(&licensed("null", json!(["MIT"]))), ["W179"]);
}

#[specforge_test(
    behavior = "checks_registries",
    verify = "a doi or an orcid is asked of its resolver, a malformed one is E103 and an unknown one E105"
)]
fn a_doi_or_an_orcid_is_asked_of_its_resolver() {
    assert!(matches!(
        &planned("doi", "doi:10.1000/182", json!({})).probes[..],
        [Probe::HttpGet { url, .. }] if url == "https://doi.org/api/handles/10.1000%2F182"
    ));
    assert!(matches!(
        &planned("orcid", "https://orcid.org/0000-0002-1825-0097", json!({})).probes[..],
        [Probe::HttpGet { url, .. }] if url == "https://pub.orcid.org/v3.0/0000-0002-1825-0097/record"
    ));
    assert_eq!(
        finding("doi", "11.1000/182", json!({}))
            .expect("refused")
            .code,
        "E103"
    );
    assert_eq!(
        finding("orcid", "0000-0002-1825-0098", json!({}))
            .expect("refused")
            .code,
        "E103"
    );
    assert!(judged("doi", "10.1000/182", json!({}), ok(r#"{"responseCode":1}"#)).is_empty());
    let unknown = http(OutcomeClass::Status(404), 404, r#"{"responseCode":100}"#);
    assert_eq!(
        codes(&judged("doi", "10.1000/182", json!({}), unknown)),
        ["E105"]
    );
    assert_eq!(
        codes(&judged(
            "doi",
            "10.1000/182",
            json!({}),
            ok(r#"{"responseCode":100}"#)
        )),
        ["E105"]
    );
    assert_eq!(
        codes(&judged(
            "orcid",
            "0000-0002-1825-0097",
            json!({}),
            status(404)
        )),
        ["E105"]
    );
    assert!(judged("orcid", "0000-0002-1825-0097", json!({}), ok("{}")).is_empty());
}

#[test]
fn an_answer_with_no_body_to_read_is_unchecked_never_a_pass() {
    let bodiless = |class| ProbeOutcome {
        data: OutcomeData::None,
        ..http(class, 200, "")
    };
    for class in [OutcomeClass::Ok, OutcomeClass::RangeIgnored] {
        assert_eq!(
            codes(&judged(
                "orcid",
                "0000-0002-1825-0097",
                json!({}),
                bodiless(class)
            )),
            ["W172"]
        );
        assert_eq!(
            codes(&judged("doi", "10.1000/182", json!({}), bodiless(class))),
            ["W172"]
        );
    }
}
