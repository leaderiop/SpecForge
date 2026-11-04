//! The media family: that the image, SVG, PDF or media file a value names is one, of a format its check allows and within
//! its limits. A path is read under its root through the `files` scope; an https URL through `media-remote`, by ranges:
//! the head of an image or a media file, an SVG whole up to 1 MiB, a PDF's head and tail and then what its
//! cross-reference names.

use serde_json::{json, Value};
use specforge_checks_kit::links::UrlParts;
use specforge_checks_kit::pdf::{self, Failure, Opened};
use specforge_checks_kit::sniff::{self, CONTAINERS, IMAGES};
use specforge_checks_kit::svg::{self, Refusal};
use specforge_checks_kit::Invalid;
use specforge_extension_sdk::prelude::*;

use super::files::missing;
use super::{checker, finding_of, path_shape, rounds_checker, scope_of, Args, FILES, MEDIA};
use crate::codes::*;

/// The most of an image or a media file read: its header is in it.
const HEAD: u32 = 64 * 1024;
/// The most of an SVG read: past it, it is not checked (W176).
const SVG_READ: u64 = 1024 * 1024;
/// The formats `image` allows when its `formats` argument names none: what a web page shows.
const WEB_IMAGES: [&str; 6] = ["png", "jpeg", "webp", "gif", "svg", "avif"];
/// The rounds of probes a value runs: a PDF's head and tail, then what its cross-reference names, twice.
const ROUNDS: u64 = 3;
/// The codes a read reports, whatever the checker reads it for.
const READ: [&str; 9] = [E016, E099, E103, W172, W173, W176, W177, W181, W182];

/// Where a value's bytes are: a path under its root, or an https URL (as requested, with no fragment).
enum Source {
    Local,
    Remote(String),
}

/// The source `text` names, or why it is not read: a URL that does not parse, or carries credentials, is E103; one
/// of another scheme than https is E099.
fn source(text: &str) -> Result<Source, CheckFinding> {
    let Some((scheme, _)) = text.split_once("://") else {
        return Ok(Source::Local);
    };
    let is_scheme = scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c));
    if !is_scheme {
        return Ok(Source::Local);
    }
    if !scheme.eq_ignore_ascii_case("https") {
        return Err(on(E099, text, format!("'{text}' is read over https only")));
    }
    let parts = UrlParts::parse(text).map_err(|invalid| {
        let reason = format!("'{text}' is not an absolute URL: {}", invalid.reason);
        finding_of(E103, Invalid { reason, ..invalid })
    })?;
    if let Some(at) = &parts.credentials {
        return Err(
            CheckFinding::new(E103, "a URL carries no user name or password").at(at.start, at.end),
        );
    }
    let mut url = parts.url;
    url.set_fragment(None);
    Ok(Source::Remote(url.to_string()))
}

/// A finding of `code` on the whole value.
fn on(code: &str, text: &str, message: String) -> CheckFinding {
    CheckFinding::new(code, message).at(0, text.len())
}

fn read(text: &str, source: &Source, args: Args, range: ByteRange) -> Probe {
    match source {
        Source::Local => Probe::FsRead {
            root: args.root(),
            path: text.to_string(),
            range,
        },
        Source::Remote(url) => Probe::HttpGet {
            url: url.clone(),
            range,
        },
    }
}

/// The bytes one read served: where they start, the whole's length when it is known, and whether they stop at a cap.
struct Served<'o> {
    bytes: &'o [u8],
    start: u64,
    length: Option<u64>,
    truncated: bool,
}

impl Served<'_> {
    /// Whether the file was read to its end.
    fn whole(&self) -> bool {
        self.start == 0 && !self.truncated && self.length == Some(self.bytes.len() as u64)
    }
}

/// What a read served, or what the value is told instead; nothing when the host reported it itself (a path that
/// leaves its root, a probe not approved).
fn served<'o>(
    text: &str,
    args: Args,
    outcome: &'o ProbeOutcome,
) -> Result<Served<'o>, Option<CheckFinding>> {
    let told = |code: &str, message: String| Err(Some(on(code, text, message)));
    match (&outcome.class, &outcome.data) {
        (
            OutcomeClass::Ok | OutcomeClass::Capped,
            OutcomeData::Bytes {
                bytes,
                start,
                length,
                truncated,
            },
        ) => Ok(Served {
            bytes,
            start: *start,
            length: Some(*length),
            truncated: *truncated,
        }),
        (
            OutcomeClass::Ok | OutcomeClass::Capped,
            OutcomeData::Http {
                body,
                start,
                total_len,
                truncated,
                ..
            },
        ) => Ok(Served {
            bytes: body,
            start: *start,
            length: *total_len,
            truncated: *truncated,
        }),
        (OutcomeClass::Missing, _) => Err(Some(missing(text, args, outcome, [0, text.len()]))),
        (OutcomeClass::Denied, OutcomeData::Http { .. }) => told(
            W173,
            format!("'{text}' leads where the media-remote grant does not reach"),
        ),
        (OutcomeClass::Denied, _) => told(W176, format!("'{text}' is no regular file that can be read")),
        (OutcomeClass::Unreachable(why), _) => {
            told(W172, format!("'{text}' could not be reached ({why:?})"))
        }
        (OutcomeClass::Deadline, _) => told(W172, format!("'{text}' did not answer in time")),
        (OutcomeClass::RangeIgnored, _) => told(
            W172,
            format!("'{text}' answered a request for part of it with all of it, past the 1 MiB read: it is not checked"),
        ),
        (OutcomeClass::Redirect(why), _) => told(
            W173,
            format!("'{text}' redirects where it is not followed ({why:?})"),
        ),
        (OutcomeClass::Status(code @ (404 | 410)), _) => {
            told(E099, format!("'{text}' answered {code}: there is nothing to read"))
        }
        (OutcomeClass::Status(code @ (401 | 403)), _) => told(
            W182,
            format!("'{text}' answered {code}: it needs credentials"),
        ),
        (OutcomeClass::Status(code), _) => told(W181, format!("'{text}' answered {code}")),
        _ => Err(None),
    }
}

/// W177 when the file is longer than the check's `max_bytes`.
fn over_bytes(text: &str, args: Args, length: Option<u64>) -> Option<CheckFinding> {
    let (max, length) = (args.count("max_bytes")?, length?);
    (length > max).then(|| {
        on(
            W177,
            text,
            format!("'{text}' is {length} bytes, over max_bytes {max}"),
        )
    })
}

/// The formats a check allows: its `formats` argument, else `default`.
fn allowed<'a>(args: Args<'a>, default: &[&'a str]) -> Vec<&'a str> {
    match args.words("formats") {
        words if words.is_empty() => default.to_vec(),
        words => words,
    }
}

/// The value's source, unless its `formats` names one outside `known` (E103) or the source is refused.
fn planned_source(
    text: &str,
    args: Args,
    kind: &str,
    known: &[&str],
) -> Result<Source, CheckFinding> {
    if let Some(unknown) = args
        .words("formats")
        .into_iter()
        .find(|w| !known.contains(w))
    {
        let message = format!(
            "'{unknown}' is no {kind} format this check knows: {}",
            known.join(", ")
        );
        return Err(on(E103, text, message));
    }
    source(text)
}

/// The first read, served, or what the value is told instead.
fn first<'o>(
    text: &str,
    args: Args,
    outcomes: &'o [ProbeOutcome],
) -> Result<Served<'o>, Vec<CheckFinding>> {
    served(text, args, &outcomes[0]).map_err(|told| told.into_iter().collect())
}

fn image_judge(text: &str, args: Args, outcomes: &[ProbeOutcome]) -> Vec<CheckFinding> {
    let served = match first(text, args, outcomes) {
        Ok(served) => served,
        Err(told) => return told,
    };
    let Some(image) = sniff::image(served.bytes) else {
        return vec![on(E099, text, format!("'{text}' is not a readable image"))];
    };
    let formats = allowed(args, &WEB_IMAGES);
    if !formats.contains(&image.format) {
        let message = format!(
            "'{text}' is a {} image, not one of {}",
            image.format,
            formats.join(", ")
        );
        return vec![on(E099, text, message)];
    }
    let mut found: Vec<CheckFinding> = over_bytes(text, args, served.length).into_iter().collect();
    match (image.size, args.count("max_pixels")) {
        (Some((width, height)), Some(max)) if width.saturating_mul(height) > max => {
            let pixels = width.saturating_mul(height);
            let message =
                format!("'{text}' is {width}x{height}, {pixels} pixels, over max_pixels {max}");
            found.push(on(W177, text, message));
        }
        (None, _) if image.format != "svg" && served.whole() => {
            let message = format!(
                "'{text}' is a {} image whose size does not read",
                image.format
            );
            found.push(on(E099, text, message));
        }
        _ => {}
    }
    found
}

fn svg_judge(text: &str, args: Args, outcomes: &[ProbeOutcome]) -> Vec<CheckFinding> {
    let served = match first(text, args, outcomes) {
        Ok(served) => served,
        Err(told) => return told,
    };
    if !served.whole() {
        let size = served
            .length
            .map_or_else(String::new, |length| format!(" is {length} bytes,"));
        let message =
            format!("'{text}'{size} over the 1 MiB an svg check reads: it is not checked");
        return vec![on(W176, text, message)];
    }
    let mut found: Vec<CheckFinding> = match svg::scan(served.bytes) {
        Ok(None) => Vec::new(),
        Ok(Some(active)) => vec![on(
            E113,
            text,
            format!(
                "'{text}' carries active content at {}: {}",
                active.at, active.what
            ),
        )
        .with_help(
            "an SVG opened or embedded runs it, wherever it was meant to be an image: remove it",
        )],
        Err(Refusal::NotXml { reason, at }) => vec![on(
            E099,
            text,
            format!("'{text}' is not well-formed XML at {at}: {reason}"),
        )],
        Err(Refusal::NotSvg { root }) => vec![on(
            E099,
            text,
            format!("'{text}' is XML whose root is <{root}>, not <svg>"),
        )],
        Err(Refusal::NoRoot) => vec![on(E099, text, format!("'{text}' holds no element"))],
    };
    found.extend(over_bytes(text, args, served.length));
    found
}

fn media_judge(text: &str, args: Args, outcomes: &[ProbeOutcome]) -> Vec<CheckFinding> {
    let served = match first(text, args, outcomes) {
        Ok(served) => served,
        Err(told) => return told,
    };
    let formats = allowed(args, &CONTAINERS);
    let Some(container) = sniff::container(served.bytes) else {
        let message = format!(
            "'{text}' is not a media container this check knows ({})",
            CONTAINERS.join(", ")
        );
        return vec![on(E099, text, message)];
    };
    if !formats.contains(&container) {
        let message = format!("'{text}' is {container}, not one of {}", formats.join(", "));
        return vec![on(E099, text, message)];
    }
    over_bytes(text, args, served.length).into_iter().collect()
}

/// A PDF's page count from every round's reads: another round for the spans the reader still needs, while the count
/// is asked for and rounds are left; then the linearization's count, or W176 when the count cannot be read.
fn pdf_judge(text: &str, args: Args, outcomes: &[ProbeOutcome], carry: Option<&Value>) -> Judged {
    let Ok(source) = source(text) else {
        return Judged::Findings(Vec::new());
    };
    let mut reads = Vec::new();
    let mut length = None;
    for outcome in outcomes {
        match served(text, args, outcome) {
            Ok(served) => {
                length = length.or(served.length);
                reads.push(pdf::Read {
                    start: served.start,
                    bytes: served.bytes,
                });
            }
            Err(told) => return Judged::Findings(told.into_iter().collect()),
        }
    }
    let mut found: Vec<CheckFinding> = over_bytes(text, args, length).into_iter().collect();
    // With no length served, the reader is held to the bytes it was served.
    let reached = reads.iter().map(|r| r.start + r.bytes.len() as u64).max();
    let length = length.or(reached).unwrap_or(0);
    let max_pages = args.count("max_pages");
    let unread = |why: &str| {
        on(
            W176,
            text,
            format!("'{text}': its page count is not read ({why}), so max_pages is not checked"),
        )
    };
    let pages = match pdf::open(&reads, length) {
        Ok(Opened::Pages(pages)) => Some(pages),
        Ok(Opened::Undetermined(why)) => {
            found.extend(max_pages.map(|_| unread(&why)));
            None
        }
        Err(Failure::Invalid(why)) => {
            return Judged::Findings(vec![on(
                E099,
                text,
                format!("'{text}' is not a readable PDF: {why}"),
            )]);
        }
        // The file reads as a PDF so far; its count is read only when it is asked for.
        Err(Failure::Need { .. }) if max_pages.is_none() => None,
        Err(Failure::Need { spans, .. }) if carry.and_then(Value::as_u64).unwrap_or(1) < ROUNDS => {
            let round = carry.and_then(Value::as_u64).unwrap_or(1);
            let probes = spans
                .iter()
                .map(|span| {
                    let range = ByteRange::Span {
                        offset: span.offset,
                        len: span.len,
                    };
                    read(text, &source, args, range)
                })
                .collect();
            return Judged::more(probes, json!(round + 1));
        }
        Err(Failure::Need {
            hint: Some(pages), ..
        }) => Some(pages),
        Err(Failure::Need { hint: None, .. }) => {
            found.push(unread(
                "its page tree is past what three rounds of reads reached",
            ));
            None
        }
    };
    if let (Some(pages), Some(max)) = (pages, max_pages) {
        if pages > max {
            found.push(on(
                W177,
                text,
                format!("'{text}' has {pages} pages, over max_pages {max}"),
            ));
        }
    }
    Judged::Findings(found)
}

/// The limits `args` names, each a count.
fn limits(names: &'static [&'static str]) -> impl Fn(&mut CheckerBuilder) {
    move |k| {
        for name in names {
            k.arg(name, |f| {
                f.field_type(FieldType::Integer).minimum(0);
            });
        }
    }
}

/// A media checker's shape: a path under its root, or an https URL, read through both scopes, with `args`.
fn media_shape(args: impl FnOnce(&mut CheckerBuilder)) -> impl FnOnce(&mut CheckerBuilder) {
    path_shape(move |k| {
        k.scope(scope_of(&FILES));
        args(k);
    })
}

fn formats_arg(k: &mut CheckerBuilder) {
    k.arg("formats", |f| {
        f.field_type(FieldType::StringList);
    });
}

pub(super) fn contribute(c: &mut ContributionsBuilder) {
    let head = |text: &str, args: Args, kind: &str, known: &[&str]| {
        let source = planned_source(text, args, kind, known)?;
        Ok(vec![read(
            text,
            &source,
            args,
            ByteRange::Prefix { len: HEAD },
        )])
    };
    checker(
        c,
        &MEDIA,
        "image",
        &READ,
        media_shape(|k| {
            formats_arg(k);
            limits(&["max_bytes", "max_pixels"])(k);
        }),
        move |text, args| head(text, args, "image", &IMAGES),
        image_judge,
    );
    checker(
        c,
        &MEDIA,
        "media",
        &READ,
        media_shape(|k| {
            formats_arg(k);
            limits(&["max_bytes"])(k);
        }),
        move |text, args| head(text, args, "media", &CONTAINERS),
        media_judge,
    );
    let mut svg_codes = READ.to_vec();
    svg_codes.push(E113);
    checker(
        c,
        &MEDIA,
        "svg",
        &svg_codes,
        media_shape(limits(&["max_bytes"])),
        |text, args| {
            let source = source(text)?;
            Ok(vec![read(
                text,
                &source,
                args,
                ByteRange::Whole { max: SVG_READ },
            )])
        },
        svg_judge,
    );
    rounds_checker(
        c,
        &MEDIA,
        "pdf",
        &READ,
        media_shape(limits(&["max_bytes", "max_pages"])),
        |text, args| {
            let source = source(text)?;
            Ok(vec![
                read(text, &source, args, ByteRange::Prefix { len: pdf::HEAD }),
                read(text, &source, args, ByteRange::Suffix { len: pdf::TAIL }),
            ])
        },
        pdf_judge,
    );
}
