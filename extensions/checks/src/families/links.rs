//! The links family: what the URLs a value names answer. A URL the text alone refuses is never fetched; the rest are
//! read from the host's answers to one probe each.

use specforge_checks_kit::formats::hostname;
use specforge_checks_kit::html::anchors;
use specforge_checks_kit::links::{host_matches, UrlParts};
use specforge_extension_sdk::prelude::*;

use super::{finding_of, word, Args, LINKS, TEXT};
use crate::codes::*;

const SWITCH: &str = "families.links";
/// The redirects a request follows.
const HOPS: u32 = 5;
/// The most of a page `url_anchor` reads: a fragment not found in it is not decided.
const PAGE: u32 = 1024 * 1024;
/// Schemes no `schemes` argument allows.
const NEVER: [&str; 3] = ["javascript", "data", "file"];
const STATUS: Vocabulary = Vocabulary::new(&["error", "warn"]);
/// The days before a certificate's end that `tls` warns from.
const DAYS: i64 = 14;

/// The codes the status table reports, which every checker that requests the page reports.
const ANSWERS: [&str; 8] = [E091, E092, E093, W183, W182, W181, W172, W173];

fn list<'a>(args: &'a Args, name: &str) -> Vec<&'a str> {
    args.get(name)
        .and_then(serde_json::Value::as_array)
        .map(|items| items.iter().filter_map(serde_json::Value::as_str).collect())
        .unwrap_or_default()
}

/// The URL written, or why it is refused unread: E091 for one that does not parse, is relative or carries
/// credentials, E092 for a scheme outside `schemes`.
fn url_of(text: &str, args: &Args) -> Result<UrlParts, CheckFinding> {
    let parts = UrlParts::parse(text).map_err(|invalid| {
        let finding = CheckFinding::new(
            E091,
            format!("'{text}' is not an absolute URL: {}", invalid.reason),
        );
        match invalid.at {
            Some(at) => finding.at(at.start, at.end),
            None => finding,
        }
    })?;
    if let Some(at) = &parts.credentials {
        return Err(
            CheckFinding::new(E091, "a URL carries no user name or password")
                .at(at.start, at.end)
                .with_help("name the page without them; a check never sends them"),
        );
    }
    let scheme = parts.url.scheme();
    let allowed = match list(args, "schemes") {
        schemes if schemes.is_empty() => vec!["https"],
        schemes => schemes,
    };
    if NEVER.contains(&scheme) || !allowed.iter().any(|s| s.eq_ignore_ascii_case(scheme)) {
        let shown = allowed.join(", ");
        return Err(CheckFinding::new(
            E092,
            format!("the scheme '{scheme}' is not one of {shown}"),
        )
        .at(parts.scheme.start, parts.scheme.end));
    }
    if parts.url.host_str().is_none_or(str::is_empty) {
        return Err(CheckFinding::new(E091, format!("'{text}' names no host")));
    }
    Ok(parts)
}

/// The URL a request asks for: as parsed, with no fragment, which no request sends.
fn requested(parts: &UrlParts) -> String {
    let mut url = parts.url.clone();
    url.set_fragment(None);
    url.to_string()
}

/// The status table: what the answer to a request of `text` reports, or nothing for a page that is there.
fn answered(text: &str, args: &Args, outcome: &ProbeOutcome) -> Option<CheckFinding> {
    let status = match &outcome.data {
        OutcomeData::Http { status, .. } => *status,
        _ => None,
    };
    let (code, message) = match (&outcome.class, status) {
        (OutcomeClass::Unreachable(why), _) => {
            (W172, format!("'{text}' could not be reached ({why:?})"))
        }
        (OutcomeClass::Deadline, _) => (W172, format!("'{text}' did not answer in time")),
        (OutcomeClass::Redirect(why), _) => (
            W173,
            format!("'{text}' redirects where it is not followed ({why:?})"),
        ),
        (OutcomeClass::Denied, _) => (
            W173,
            format!("'{text}' leads where the links grant does not reach"),
        ),
        (_, Some(200..=299) | None) => return None,
        (_, Some(code @ (404 | 410))) if word(args, "status") == Some("warn") => {
            (W183, format!("'{text}' answered {code}"))
        }
        (_, Some(code @ (404 | 410))) => (
            E093,
            format!("'{text}' answered {code}: the page does not exist"),
        ),
        (_, Some(code @ (401 | 403))) => (
            W182,
            format!("'{text}' answered {code}: it needs credentials"),
        ),
        (_, Some(code)) => (W181, format!("'{text}' answered {code}")),
    };
    Some(CheckFinding::new(code, message))
}

/// The one probe a checker's plan asks of a URL that parsed.
type Probing = fn(&UrlParts, &Args) -> Probe;
/// What it finds once the page answered 2xx: the value's text, the URL, its args, and the answer.
type Judging = fn(&str, &UrlParts, &Args, &ProbeOutcome) -> Vec<CheckFinding>;

fn head(parts: &UrlParts, _: &Args) -> Probe {
    Probe::HttpHead {
        url: requested(parts),
        redirects: HOPS,
    }
}

fn common(k: &mut CheckerBuilder) {
    k.arg("schemes", |f| {
        f.field_type(FieldType::StringList);
    });
    k.arg("status", |f| {
        f.vocabulary(&STATUS);
    });
}

/// A checker that requests the page and reads the status table, then `judge`, when it answered 2xx.
fn page_checker(
    c: &mut ContributionsBuilder,
    name: &str,
    codes: &[&str],
    args: impl FnOnce(&mut CheckerBuilder),
    probe: Probing,
    judge: Judging,
) {
    let mut all = ANSWERS.to_vec();
    all.extend_from_slice(codes);
    url_checker(
        c,
        name,
        &all,
        |k| {
            common(k);
            args(k);
        },
        probe,
        move |text, parts, args, outcome| match answered(text, args, outcome) {
            Some(found) => vec![found],
            None => judge(text, parts, args, outcome),
        },
    );
}

fn url_checker(
    c: &mut ContributionsBuilder,
    name: &str,
    codes: &[&str],
    args: impl FnOnce(&mut CheckerBuilder),
    probe: Probing,
    judge: impl Fn(&str, &UrlParts, &Args, &ProbeOutcome) -> Vec<CheckFinding> + 'static,
) {
    c.checker(name, |k| {
        args(k);
        k.applies_to(TEXT)
            .value(ValueForm::Url)
            .scope(LINKS)
            .codes(codes)
            .when(SWITCH)
            .plan(move |item| match url_of(&item.text, &item.args) {
                _ if item.text.is_empty() => PlannedItem::default(),
                Ok(parts) => PlannedItem::probes(vec![probe(&parts, &item.args)]),
                Err(found) => PlannedItem::findings(vec![found]),
            })
            .judge(
                move |item| match (url_of(&item.text, &item.args), item.outcomes.first()) {
                    (Ok(parts), Some(outcome)) => judge(&item.text, &parts, &item.args, outcome),
                    _ => Vec::new(),
                },
            );
    });
}

fn nothing(_: &str, _: &UrlParts, _: &Args, _: &ProbeOutcome) -> Vec<CheckFinding> {
    Vec::new()
}

/// The page itself, up to [`PAGE`], when its fragment is to be found in it.
fn page(parts: &UrlParts, _: &Args) -> Probe {
    match &parts.fragment {
        Some(at) if !at.is_empty() => Probe::HttpGet {
            url: requested(parts),
            range: Some([0, u64::from(PAGE) - 1]),
            max_bytes: PAGE,
        },
        _ => head(parts, &Args::new()),
    }
}

fn anchored(text: &str, parts: &UrlParts, _: &Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
    let (
        Some(at),
        OutcomeData::Http {
            body, truncated, ..
        },
    ) = (&parts.fragment, &outcome.data)
    else {
        return Vec::new();
    };
    let fragment = &text[at.clone()];
    // A page cut off at the cap, or a range of it as long as the cap, may hold the anchor past it.
    let cut = *truncated || body.len() >= PAGE as usize;
    if fragment.is_empty() || cut || anchors(body).contains(fragment) {
        return Vec::new();
    }
    let page = &text[..at.start - 1];
    vec![CheckFinding::new(
        E112,
        format!("'{page}' has no element whose id or name is '{fragment}'"),
    )
    .at(at.start, at.end)]
}

fn typed(text: &str, _: &UrlParts, args: &Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
    let OutcomeData::Http { headers, .. } = &outcome.data else {
        return Vec::new();
    };
    let given = headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case("content-type"))
        .map(|header| header.value.as_str());
    let Some(given) = given else {
        return vec![CheckFinding::new(
            E104,
            format!("'{text}' answers no Content-Type"),
        )];
    };
    let media = given.split(';').next().unwrap_or_default().trim();
    let allow = list(args, "allow");
    if allow.is_empty() || allow.iter().any(|a| a.eq_ignore_ascii_case(media)) {
        return Vec::new();
    }
    vec![CheckFinding::new(
        E104,
        format!("'{text}' is {media}, not one of {}", allow.join(", ")),
    )]
}

fn landed(text: &str, _: &UrlParts, args: &Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
    let OutcomeData::Http { final_url, .. } = &outcome.data else {
        return Vec::new();
    };
    let hosts = list(args, "hosts");
    let host = UrlParts::parse(final_url)
        .ok()
        .and_then(|parts| parts.url.host_str().map(str::to_string))
        .unwrap_or_default();
    if hosts.is_empty() || hosts.iter().any(|pattern| host_matches(pattern, &host)) {
        return Vec::new();
    }
    vec![CheckFinding::new(
        W173,
        format!("'{text}' ends on '{host}', not one of {}", hosts.join(", ")),
    )]
}

fn handshake(parts: &UrlParts, _: &Args) -> Probe {
    Probe::HttpTls {
        host: parts.url.host_str().unwrap_or_default().to_string(),
        port: parts.url.port_or_known_default().unwrap_or(443),
    }
}

/// W172 for a handshake that failed or a chain the host did not trust, and W178 for a certificate near or past its
/// end, which a rejected one may be too.
fn certified(text: &str, _: &UrlParts, args: &Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
    let mut found = Vec::new();
    let rejected = matches!(
        outcome.data,
        OutcomeData::Tls {
            chain_valid: false,
            ..
        }
    );
    if let OutcomeClass::Unreachable(why) = &outcome.class {
        let message = if rejected {
            format!("the certificate of '{text}' is not trusted ({why:?}): self-signed, for another host or unknown")
        } else {
            format!("'{text}' could not be reached ({why:?})")
        };
        found.push(CheckFinding::new(W172, message));
    } else if rejected {
        found.push(CheckFinding::new(
            W172,
            format!("the certificate of '{text}' is not trusted"),
        ));
    }
    let OutcomeData::Tls {
        not_after: Some(end),
        ..
    } = &outcome.data
    else {
        return found;
    };
    let days = args
        .get("days")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(DAYS);
    match days_left(end, outcome.observed_at) {
        Some(left) if left < 0 => found.push(CheckFinding::new(
            W178,
            format!("the certificate of '{text}' ended on {end}"),
        )),
        Some(left) if left < days => found.push(CheckFinding::new(
            W178,
            format!("the certificate of '{text}' ends on {end}, in {left} day(s)"),
        )),
        _ => {}
    }
    found
}

/// Whole days from `now_ms` (Unix milliseconds) to `end` (`YYYY-MM-DDTHH:MM:SSZ`), rounded down; None for another form.
pub(crate) fn days_left(end: &str, now_ms: u64) -> Option<i64> {
    let number = |range: std::ops::Range<usize>| end.get(range)?.parse::<i64>().ok();
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    // Days from 1970-01-01 to a civil date (H. Hinnant's `days_from_civil`).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let of_era = y - era * 400;
    let of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    let days = era * 146_097 + of_cycle - 719_468;
    let seconds = days * 86_400 + hour * 3_600 + minute * 60 + second;
    let now = i64::try_from(now_ms / 1000).ok()?;
    Some((seconds - now).div_euclid(86_400))
}

/// `dns`: a host name, which the text alone may refuse (E103), resolved through the host's Http port.
fn dns(c: &mut ContributionsBuilder) {
    c.checker("dns", |k| {
        k.applies_to(TEXT)
            .scope(LINKS)
            .codes(&[E103, W172])
            .when(SWITCH)
            .plan(|item| match hostname(&item.text, false) {
                _ if item.text.is_empty() => PlannedItem::default(),
                Ok(()) => PlannedItem::probes(vec![Probe::DnsResolve {
                    host: item.text.clone(),
                }]),
                Err(invalid) => PlannedItem::findings(vec![finding_of(invalid)]),
            })
            .judge(
                |item| match item.outcomes.first().map(|outcome| &outcome.class) {
                    Some(OutcomeClass::Unreachable(why)) => vec![CheckFinding::new(
                        W172,
                        format!("'{}' does not resolve ({why:?})", item.text),
                    )],
                    _ => Vec::new(),
                },
            );
    });
}

pub(super) fn contribute(c: &mut ContributionsBuilder) {
    let none = |_: &mut CheckerBuilder| {};
    page_checker(c, "url", &[], none, head, nothing);
    page_checker(c, "url_anchor", &[E112], none, page, anchored);
    let allow = |k: &mut CheckerBuilder| {
        k.arg("allow", |f| {
            f.field_type(FieldType::StringList);
        });
    };
    page_checker(c, "content_type", &[E104], allow, head, typed);
    let hosts = |k: &mut CheckerBuilder| {
        k.arg("hosts", |f| {
            f.field_type(FieldType::StringList);
        });
    };
    page_checker(c, "redirect", &[], hosts, head, landed);
    let days = |k: &mut CheckerBuilder| {
        k.arg("days", |f| {
            f.field_type(FieldType::Integer).minimum(0);
        });
    };
    url_checker(
        c,
        "tls",
        &[E091, E092, W172, W178],
        days,
        handshake,
        certified,
    );
    dns(c);
}
