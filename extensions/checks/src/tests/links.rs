//! The links family: what each checker plans for a URL and finds in canned answers, as the host would send them.

use serde_json::json;
use specforge_protocol_types::{
    CheckFinding, Header, OutcomeClass, OutcomeData, Probe, ProbeOutcome, Redirected, Unreachable,
};

use super::{judged, planned};
use crate::families::links::days_left;

const URL: &str = "https://a.example/page";

fn answer(class: OutcomeClass, data: OutcomeData) -> ProbeOutcome {
    ProbeOutcome {
        probe: Probe::HttpHead {
            url: URL.into(),
            redirects: 5,
        },
        class,
        data,
        observed_at: 0,
        from_cache: false,
    }
}

fn http(status: u16, headers: &[(&str, &str)], body: &str, truncated: bool) -> OutcomeData {
    OutcomeData::Http {
        status: Some(status),
        final_url: URL.into(),
        headers: headers.iter().map(|(n, v)| Header::plain(*n, *v)).collect(),
        body: body.as_bytes().to_vec(),
        truncated,
    }
}

fn status(code: u16) -> ProbeOutcome {
    let class = if (200..300).contains(&code) {
        OutcomeClass::Ok
    } else {
        OutcomeClass::Status(code)
    };
    answer(class, http(code, &[], "", false))
}

fn codes(findings: &[CheckFinding]) -> Vec<&str> {
    findings.iter().map(|f| f.code.as_str()).collect()
}

/// The one code `checker` finds in [`URL`] once it answered `outcome`, or none.
fn code_of(checker: &str, args: serde_json::Value, outcome: ProbeOutcome) -> Option<String> {
    let found = judged(checker, URL, args, outcome);
    assert!(found.len() < 2, "{found:?}");
    found.into_iter().next().map(|f| f.code)
}

#[test]
fn links_url_heads_the_page_with_five_redirects_and_no_fragment() {
    let plan = planned("url", "https://a.example/page#top", json!({}));
    assert_eq!(
        plan.probes,
        [Probe::HttpHead {
            url: URL.into(),
            redirects: 5
        }]
    );
    assert!(plan.findings.is_empty());
}

#[test]
fn links_status_table_every_row() {
    let rows: [(ProbeOutcome, Option<&str>); 14] = [
        (status(200), None),
        (status(204), None),
        (status(404), Some("E093")),
        (status(410), Some("E093")),
        (status(401), Some("W182")),
        (status(403), Some("W182")),
        (status(429), Some("W181")),
        (status(500), Some("W181")),
        (status(418), Some("W181")),
        (
            answer(
                OutcomeClass::Unreachable(Unreachable::Dns),
                OutcomeData::None,
            ),
            Some("W172"),
        ),
        (
            answer(OutcomeClass::Deadline, OutcomeData::None),
            Some("W172"),
        ),
        (
            answer(OutcomeClass::Redirect(Redirected::Limit), OutcomeData::None),
            Some("W173"),
        ),
        (
            answer(
                OutcomeClass::Redirect(Redirected::Downgrade),
                OutcomeData::None,
            ),
            Some("W173"),
        ),
        (
            answer(OutcomeClass::Denied, OutcomeData::None),
            Some("W173"),
        ),
    ];
    for (outcome, code) in rows {
        let shown = format!("{:?}", outcome.class);
        for checker in ["url", "url_anchor", "content_type", "redirect"] {
            let found = code_of(checker, json!({"allow": ["text/html"]}), outcome.clone());
            // A 2xx answer with no Content-Type is content_type's own E104, not the table's.
            let expected = match (checker, code) {
                ("content_type", None) => Some("E104"),
                _ => code,
            };
            assert_eq!(found.as_deref(), expected, "{checker} on {shown}");
        }
    }
    for why in [Unreachable::Connect, Unreachable::Tls, Unreachable::Timeout] {
        let outcome = answer(OutcomeClass::Unreachable(why), OutcomeData::None);
        assert_eq!(code_of("url", json!({}), outcome).as_deref(), Some("W172"));
    }
    let refused = answer(
        OutcomeClass::Redirect(Redirected::Refused),
        OutcomeData::None,
    );
    assert_eq!(code_of("url", json!({}), refused).as_deref(), Some("W173"));
}

#[test]
fn links_a_lowered_status_is_w183_only_for_404_and_410() {
    let warn = json!({"status": "warn"});
    assert_eq!(
        code_of("url", warn.clone(), status(404)).as_deref(),
        Some("W183")
    );
    assert_eq!(
        code_of("url", warn.clone(), status(410)).as_deref(),
        Some("W183")
    );
    assert_eq!(code_of("url", warn, status(500)).as_deref(), Some("W181"));
}

#[test]
fn links_a_capped_answer_is_read_by_its_status() {
    let capped = answer(OutcomeClass::Capped, http(200, &[], "", true));
    assert_eq!(code_of("url", json!({}), capped), None);
}

#[test]
fn links_credentials_are_e091_and_never_fetched() {
    let text = "https://ada:secret@a.example/page";
    let plan = planned("url", text, json!({}));
    assert!(plan.probes.is_empty(), "{:?}", plan.probes);
    let found = &plan.findings[0];
    assert_eq!(found.code, "E091");
    let [start, end] = found.at.expect("on the credentials");
    assert_eq!(&text[start..end], "ada:secret");
}

#[test]
fn links_a_malformed_or_relative_url_is_e091_and_never_fetched() {
    for text in ["/page", "https://", "https://a.example/a page", "not a url"] {
        let plan = planned("url", text, json!({}));
        assert!(plan.probes.is_empty(), "{text}: {:?}", plan.probes);
        assert_eq!(codes(&plan.findings), ["E091"], "{text}");
    }
}

#[test]
fn links_a_scheme_outside_schemes_is_e092_on_the_scheme() {
    let plan = planned("url", "http://a.example/", json!({}));
    assert!(plan.probes.is_empty());
    assert_eq!(codes(&plan.findings), ["E092"]);
    assert_eq!(plan.findings[0].at, Some([0, 4]));

    let both = json!({"schemes": ["https", "http"]});
    assert_eq!(planned("url", "http://a.example/", both).probes.len(), 1);
    for never in [
        "javascript:alert(1)",
        "data:text/html,x",
        "file:///etc/passwd",
    ] {
        let allowed = json!({"schemes": ["javascript", "data", "file"]});
        let plan = planned("url", never, allowed);
        assert_eq!(codes(&plan.findings), ["E092"], "{never}");
        assert!(plan.probes.is_empty());
    }
}

#[test]
fn links_fragment_missing_is_e112_on_the_fragment() {
    let text = "https://a.example/page#usage";
    let plan = planned("url_anchor", text, json!({}));
    assert_eq!(
        plan.probes,
        [Probe::HttpGet {
            url: URL.into(),
            range: Some([0, 1024 * 1024 - 1]),
            max_bytes: 1024 * 1024
        }]
    );
    let page = |body: &str| answer(OutcomeClass::Ok, http(200, &[], body, false));
    let found = judged("url_anchor", text, json!({}), page("<h2 id=\"install\">"));
    assert_eq!(codes(&found), ["E112"]);
    let [start, end] = found[0].at.expect("on the fragment");
    assert_eq!(&text[start..end], "usage");
    assert!(judged("url_anchor", text, json!({}), page("<a name=\"usage\">")).is_empty());
    // With no fragment the page is only headed.
    let bare = planned("url_anchor", URL, json!({}));
    assert!(matches!(bare.probes[..], [Probe::HttpHead { .. }]));
}

#[test]
fn links_fragment_beyond_the_cap_is_undecided() {
    let text = "https://a.example/page#usage";
    let cut = answer(OutcomeClass::Capped, http(200, &[], "<p id=\"a\">", true));
    assert!(judged("url_anchor", text, json!({}), cut).is_empty());
    let range = "x".repeat(1024 * 1024);
    let full = answer(OutcomeClass::Ok, http(206, &[], &range, false));
    assert!(judged("url_anchor", text, json!({}), full).is_empty());
}

#[test]
fn links_content_type_outside_allow_or_missing_is_e104() {
    let typed = |value: &str| {
        answer(
            OutcomeClass::Ok,
            http(200, &[("content-type", value)], "", false),
        )
    };
    let allow = json!({"allow": ["application/pdf"]});
    assert_eq!(
        code_of("content_type", allow.clone(), typed("Application/PDF; q=1")),
        None
    );
    assert_eq!(
        code_of(
            "content_type",
            allow.clone(),
            typed("text/html; charset=utf-8")
        )
        .as_deref(),
        Some("E104")
    );
    assert_eq!(
        code_of("content_type", allow, status(200)).as_deref(),
        Some("E104")
    );
}

#[test]
fn links_redirect_ending_outside_hosts_is_w173() {
    let landed = |url: &str| {
        let mut data = http(200, &[], "", false);
        if let OutcomeData::Http { final_url, .. } = &mut data {
            *final_url = url.into();
        }
        answer(OutcomeClass::Ok, data)
    };
    let hosts = json!({"hosts": ["*.example.org", "a.example"]});
    assert_eq!(code_of("redirect", hosts.clone(), landed(URL)), None);
    assert_eq!(
        code_of(
            "redirect",
            hosts.clone(),
            landed("https://docs.example.org/x")
        ),
        None
    );
    assert_eq!(
        code_of("redirect", hosts, landed("https://evil.example/")).as_deref(),
        Some("W173")
    );
}

#[test]
fn links_tls_warns_within_its_days_and_refuses_http() {
    let plan = planned("tls", "https://a.example:8443/x", json!({}));
    assert_eq!(
        plan.probes,
        [Probe::HttpTls {
            host: "a.example".into(),
            port: 8443
        }]
    );
    let http_url = planned("tls", "http://a.example/", json!({}));
    assert_eq!(codes(&http_url.findings), ["E092"]);
    assert!(http_url.probes.is_empty());

    // 2026-01-01T00:00:00Z.
    let now = 1_767_225_600_000;
    let cert = |end: &str| ProbeOutcome {
        observed_at: now,
        ..answer(
            OutcomeClass::Ok,
            OutcomeData::Tls {
                not_after: Some(end.into()),
                issuer: None,
                chain_valid: true,
            },
        )
    };
    assert_eq!(
        code_of("tls", json!({}), cert("2026-01-10T00:00:00Z")).as_deref(),
        Some("W178")
    );
    assert_eq!(
        code_of("tls", json!({}), cert("2026-03-01T00:00:00Z")),
        None
    );
    assert_eq!(
        code_of("tls", json!({"days": 90}), cert("2026-03-01T00:00:00Z")).as_deref(),
        Some("W178")
    );
    assert_eq!(
        code_of("tls", json!({}), cert("2025-12-01T00:00:00Z")).as_deref(),
        Some("W178")
    );
    let unreachable = answer(
        OutcomeClass::Unreachable(Unreachable::Connect),
        OutcomeData::None,
    );
    assert_eq!(
        code_of("tls", json!({}), unreachable).as_deref(),
        Some("W172")
    );
}

#[test]
fn links_tls_rejected_chain_is_w172_and_w178_near_its_end() {
    // 2026-01-01T00:00:00Z.
    let now = 1_767_225_600_000;
    let rejected = |end: &str| ProbeOutcome {
        observed_at: now,
        ..answer(
            OutcomeClass::Unreachable(Unreachable::Tls),
            OutcomeData::Tls {
                not_after: Some(end.into()),
                issuer: Some("CN=self".into()),
                chain_valid: false,
            },
        )
    };
    let far = judged("tls", URL, json!({}), rejected("2030-01-01T00:00:00Z"));
    assert_eq!(codes(&far), ["W172"]);
    assert!(far[0].message.contains("not trusted"), "{}", far[0].message);
    let near = judged("tls", URL, json!({}), rejected("2026-01-05T00:00:00Z"));
    assert_eq!(codes(&near), ["W172", "W178"]);
}

#[test]
fn links_days_left_counts_whole_days_to_the_end() {
    let now = 1_767_225_600_000; // 2026-01-01T00:00:00Z
    assert_eq!(days_left("2026-01-01T00:00:00Z", now), Some(0));
    assert_eq!(days_left("2026-01-15T12:00:00Z", now), Some(14));
    assert_eq!(days_left("2025-12-31T23:59:59Z", now), Some(-1));
    assert_eq!(days_left("2024-02-29T00:00:00Z", 0), Some(19_782));
    assert_eq!(days_left("not a date", now), None);
}

#[test]
fn links_dns_resolves_a_host_name_and_refuses_a_malformed_one() {
    let plan = planned("dns", "a.example", json!({}));
    assert_eq!(
        plan.probes,
        [Probe::DnsResolve {
            host: "a.example".into()
        }]
    );
    let bad = planned("dns", "a..example", json!({}));
    assert_eq!(codes(&bad.findings), ["E103"]);
    assert!(bad.probes.is_empty());
    let gone = answer(
        OutcomeClass::Unreachable(Unreachable::Dns),
        OutcomeData::None,
    );
    let found = judged("dns", "a.example", json!({}), gone);
    assert_eq!(codes(&found), ["W172"]);
    let found = answer(
        OutcomeClass::Ok,
        OutcomeData::Resolve {
            addresses: vec!["192.0.2.1".into()],
        },
    );
    assert!(judged("dns", "a.example", json!({}), found).is_empty());
}
