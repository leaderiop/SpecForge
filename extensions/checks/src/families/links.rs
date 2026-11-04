//! The links family: what the URLs a value names answer. A URL the text alone refuses is never fetched; the rest are
//! read from the host's answers to one probe each.

use jiff::Timestamp;
use mime::Mime;
use specforge_checks_kit::formats::hostname;
use specforge_checks_kit::html::anchors;
use specforge_checks_kit::links::{fragment_id, host_matches, UrlParts};
use specforge_checks_kit::Invalid;
use specforge_extension_sdk::prelude::*;

use super::{checker, choice_arg, finding_of, no_args, Args, Word, LINKS};
use crate::codes::*;

/// The redirects a request follows.
const HOPS: u32 = 5;
/// The most of a page `url_anchor` reads: a fragment not found in it is not decided.
const PAGE: u32 = 1024 * 1024;
/// Schemes no `schemes` argument allows.
const NEVER: [&str; 3] = ["javascript", "data", "file"];
/// The days before a certificate's end that `tls` warns from.
const DAYS: i64 = 14;

/// How `url` reports a page that is gone (404, 410): E093, or W183 on a field that lowers it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Gone {
    Error,
    Warn,
}

impl Word for Gone {
    const VOCABULARY: Vocabulary = Vocabulary::new(&["error", "warn"]);
    const ALL: &'static [Self] = &[Gone::Error, Gone::Warn];
}

/// The codes the status table reports, which every checker that requests the page reports.
const ANSWERS: [&str; 8] = [E091, E092, E093, W183, W182, W181, W172, W173];

/// The URL written, or why it is refused unread: E091 for one that does not parse, is relative or carries
/// credentials, E092 for a scheme outside `schemes`.
fn url_of(text: &str, args: Args) -> Result<UrlParts, CheckFinding> {
    let parts = UrlParts::parse(text).map_err(|invalid| {
        let reason = format!("'{text}' is not an absolute URL: {}", invalid.reason);
        finding_of(E091, Invalid { reason, ..invalid })
    })?;
    if let Some(at) = &parts.credentials {
        return Err(
            CheckFinding::new(E091, "a URL carries no user name or password")
                .at(at.start, at.end)
                .with_help("name the page without them; a check never sends them"),
        );
    }
    let scheme = parts.url.scheme();
    let allowed = match args.words("schemes") {
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
fn answered(text: &str, args: Args, outcome: &ProbeOutcome) -> Option<CheckFinding> {
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
        (_, Some(code @ (404 | 410))) if args.choice("status") == Some(Gone::Warn) => {
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

/// The one probe a URL checker asks of a URL that parsed.
type UrlProbe = fn(&UrlParts, Args) -> Probe;
/// What a page checker finds once the page answered 2xx: the value's text, the URL, its args, and the answer.
type PageJudge = fn(&str, &UrlParts, Args, &ProbeOutcome) -> Vec<CheckFinding>;

fn head(parts: &UrlParts, _: Args) -> Probe {
    Probe::HttpHead {
        url: requested(parts),
        redirects: HOPS,
    }
}

fn common(k: &mut CheckerBuilder) {
    k.arg("schemes", |f| {
        f.field_type(FieldType::StringList);
    });
    choice_arg::<Gone>("status")(k);
}

/// A checker that requests the page and reads the status table, then `judge`, when it answered 2xx.
fn page_checker(
    c: &mut ContributionsBuilder,
    name: &str,
    codes: &[&str],
    args: impl FnOnce(&mut CheckerBuilder),
    probe: UrlProbe,
    judge: PageJudge,
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

/// A checker of one URL: E091 or E092 for one it never requests, else `probe`'s answer, judged.
fn url_checker(
    c: &mut ContributionsBuilder,
    name: &str,
    codes: &[&str],
    args: impl FnOnce(&mut CheckerBuilder),
    probe: UrlProbe,
    judge: impl Fn(&str, &UrlParts, Args, &ProbeOutcome) -> Vec<CheckFinding> + 'static,
) {
    let shape = |k: &mut CheckerBuilder| {
        args(k);
        k.value(ValueForm::Url);
    };
    let plan = move |text: &str, args: Args| Ok(vec![probe(&url_of(text, args)?, args)]);
    checker(
        c,
        &LINKS,
        name,
        codes,
        shape,
        plan,
        move |text, args, outcomes| match url_of(text, args) {
            Ok(parts) => judge(text, &parts, args, &outcomes[0]),
            Err(_) => Vec::new(),
        },
    );
}

fn nothing(_: &str, _: &UrlParts, _: Args, _: &ProbeOutcome) -> Vec<CheckFinding> {
    Vec::new()
}

/// The page itself, up to [`PAGE`], when its fragment is to be found in it.
fn page(parts: &UrlParts, args: Args) -> Probe {
    match &parts.fragment {
        Some(at) if !at.is_empty() => Probe::HttpGet {
            url: requested(parts),
            range: Some([0, u64::from(PAGE) - 1]),
            max_bytes: PAGE,
        },
        _ => head(parts, args),
    }
}

fn anchored(text: &str, parts: &UrlParts, _: Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
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
    // HTML matches the fragment decoded, and `top` is the top of any page.
    let id = fragment_id(fragment);
    let anchors = anchors(body);
    let found = anchors.contains(fragment) || anchors.contains(id.as_ref());
    if fragment.is_empty() || cut || found || id.eq_ignore_ascii_case("top") {
        return Vec::new();
    }
    let page = &text[..at.start - 1];
    vec![CheckFinding::new(
        E112,
        format!("'{page}' has no element whose id or name is '{fragment}'"),
    )
    .at(at.start, at.end)]
}

fn typed(text: &str, _: &UrlParts, args: Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
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
    let Ok(media) = given.parse::<Mime>() else {
        return vec![CheckFinding::new(
            E104,
            format!("'{text}' answers a Content-Type that is no media type"),
        )];
    };
    let media = media.essence_str();
    let allow = args.words("allow");
    if allow.is_empty() || allow.iter().any(|a| a.eq_ignore_ascii_case(media)) {
        return Vec::new();
    }
    vec![CheckFinding::new(
        E104,
        format!("'{text}' is {media}, not one of {}", allow.join(", ")),
    )]
}

fn landed(text: &str, _: &UrlParts, args: Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
    let OutcomeData::Http { final_url, .. } = &outcome.data else {
        return Vec::new();
    };
    let hosts = args.words("hosts");
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

fn handshake(parts: &UrlParts, _: Args) -> Probe {
    Probe::HttpTls {
        host: parts.url.host_str().unwrap_or_default().to_string(),
        port: parts.url.port_or_known_default().unwrap_or(443),
    }
}

/// W172 for a handshake that failed or a chain the host did not trust, and W178 for a certificate near or past its
/// end, which a rejected one may be too.
fn certified(text: &str, _: &UrlParts, args: Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
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
    let days = args.integer("days").unwrap_or(DAYS);
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

/// Whole days from `now_ms` (Unix milliseconds) to `end` (RFC 3339, any offset or fraction), rounded down; None for
/// another form.
pub(crate) fn days_left(end: &str, now_ms: u64) -> Option<i64> {
    let end = end.parse::<Timestamp>().ok()?;
    let now = i64::try_from(now_ms).ok()?;
    Some((end.as_millisecond() - now).div_euclid(86_400_000))
}

/// `dns`: a host name, which the text alone may refuse (E103), resolved through the host's Http port.
fn dns(c: &mut ContributionsBuilder) {
    let plan = |text: &str, _: Args| {
        hostname(text, false)
            .map(|()| {
                vec![Probe::DnsResolve {
                    host: text.to_string(),
                }]
            })
            .map_err(|invalid| finding_of(E103, invalid))
    };
    let judge = |text: &str, _: Args, outcomes: &[ProbeOutcome]| match &outcomes[0].class {
        OutcomeClass::Unreachable(why) => vec![CheckFinding::new(
            W172,
            format!("'{text}' does not resolve ({why:?})"),
        )],
        _ => Vec::new(),
    };
    checker(c, &LINKS, "dns", &[E103, W172], no_args, plan, judge);
}

pub(super) fn contribute(c: &mut ContributionsBuilder) {
    page_checker(c, "url", &[], no_args, head, nothing);
    page_checker(c, "url_anchor", &[E112], no_args, page, anchored);
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
