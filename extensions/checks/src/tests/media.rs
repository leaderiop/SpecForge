//! The media family: what each checker plans for its text and finds in canned reads, as the host would send them.

use serde_json::json;
use specforge_extension_sdk::testing::MockHost;
use specforge_protocol_types::{
    ByteRange, CheckFinding, OutcomeClass, OutcomeData, PlannedItem, Probe, ProbeOutcome, Root,
    Unreachable,
};
use specforge_test::prelude::*;

use super::{finding, judged, planned};

const URL: &str = "https://cdn.example/a.pdf";

fn answer(class: OutcomeClass, data: OutcomeData) -> ProbeOutcome {
    ProbeOutcome {
        probe: Probe::FsStat {
            root: Root::Project,
            path: String::new(),
        },
        class,
        data,
        observed_at: 0,
        from_cache: false,
    }
}

/// `bytes` of a file of `length` bytes, from `start`.
fn part(bytes: &[u8], start: u64, length: u64) -> ProbeOutcome {
    answer(
        OutcomeClass::Ok,
        OutcomeData::Bytes {
            bytes: bytes.to_vec(),
            start,
            length,
            truncated: false,
        },
    )
}

fn read(bytes: &[u8]) -> ProbeOutcome {
    part(bytes, 0, bytes.len() as u64)
}

fn codes(findings: &[CheckFinding]) -> Vec<&str> {
    findings.iter().map(|f| f.code.as_str()).collect()
}

fn found(checker: &str, args: serde_json::Value, bytes: &[u8]) -> Vec<CheckFinding> {
    judged(checker, "a.bin", args, read(bytes))
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    out
}

#[specforge_test(
    behavior = "checks_media",
    verify = "a media checker reads a path under its root and an https URL by ranges, and refuses other schemes"
)]
fn a_media_checker_reads_a_path_under_its_root_and_a_url_by_ranges() {
    let plan = planned("image", "assets/a.png", json!({}));
    assert_eq!(
        plan.probes,
        [Probe::FsRead {
            root: Root::Project,
            path: "assets/a.png".into(),
            range: ByteRange::Prefix { len: 64 * 1024 },
        }]
    );
    let plan = planned("svg", "https://cdn.example/a.svg#top", json!({}));
    assert_eq!(
        plan.probes,
        [Probe::HttpGet {
            url: "https://cdn.example/a.svg".into(),
            range: ByteRange::Whole { max: 1024 * 1024 },
        }]
    );
    let plan = planned("pdf", URL, json!({}));
    assert_eq!(
        plan.probes,
        [
            Probe::HttpGet {
                url: URL.into(),
                range: ByteRange::Prefix { len: 1024 },
            },
            Probe::HttpGet {
                url: URL.into(),
                range: ByteRange::Suffix { len: 64 * 1024 },
            },
        ]
    );
    let refused = finding("media", "http://cdn.example/a.mp4", json!({})).expect("refused");
    assert_eq!(refused.code, "E099");
    let unknown = finding("image", "a.png", json!({"formats": ["png", "jpg"]})).expect("refused");
    assert_eq!(unknown.code, "E103");
    assert!(unknown.message.contains("'jpg'"), "{}", unknown.message);
    let credentials =
        finding("image", "https://u:p@cdn.example/a.png", json!({})).expect("refused");
    assert_eq!(credentials.code, "E103");
}

#[specforge_test(
    behavior = "checks_media",
    verify = "an image of a format not allowed is E099, and one over its pixels or bytes is W177"
)]
fn an_image_not_allowed_is_e099_and_one_over_its_limits_is_w177() {
    assert!(found("image", json!({}), &png(64, 64)).is_empty());
    let only_gif = found("image", json!({"formats": ["gif"]}), &png(64, 64));
    assert_eq!(codes(&only_gif), ["E099"]);
    assert!(
        only_gif[0]
            .message
            .contains("is a png image, not one of gif"),
        "{}",
        only_gif[0].message
    );
    let large = found(
        "image",
        json!({"max_pixels": 1000, "max_bytes": 10}),
        &png(100, 100),
    );
    assert_eq!(codes(&large), ["W177", "W177"]);
    assert!(
        large[1].message.contains("100x100, 10000 pixels"),
        "{}",
        large[1].message
    );
    assert_eq!(
        codes(&found("image", json!({}), b"plain text, not an image")),
        ["E099"]
    );
    // A whole file whose header stops before its size is broken; a head cut at the read is not.
    let cut = &png(64, 64)[..14];
    assert_eq!(codes(&found("image", json!({}), cut)), ["E099"]);
    assert!(judged("image", "a.png", json!({}), part(cut, 0, 1 << 20)).is_empty());
}

#[specforge_test(
    behavior = "checks_media",
    verify = "an svg with active content is E113 at its line, and one not well formed or not svg is E099"
)]
fn an_svg_with_active_content_is_e113_and_one_not_svg_is_e099() {
    assert!(found(
        "svg",
        json!({}),
        b"<svg xmlns=\"http://www.w3.org/2000/svg\"><rect/></svg>"
    )
    .is_empty());
    let active = found(
        "svg",
        json!({}),
        b"<svg>\n  <a href=\"javascript:go()\"/>\n</svg>",
    );
    assert_eq!(codes(&active), ["E113"]);
    assert!(
        active[0].message.contains("at 2:3"),
        "{}",
        active[0].message
    );
    let broken = found("svg", json!({}), b"<svg>\n<g>\n</svg>");
    assert_eq!(codes(&broken), ["E099"]);
    assert!(
        broken[0].message.contains("not well-formed XML at 3:"),
        "{}",
        broken[0].message
    );
    let html = found("svg", json!({}), b"<html/>");
    assert!(
        html[0].message.contains("root is <html>"),
        "{}",
        html[0].message
    );
    let over = judged("svg", "a.svg", json!({}), part(b"<svg>", 0, 2 << 20));
    assert_eq!(codes(&over), ["W176"]);
}

#[specforge_test(
    behavior = "checks_media",
    verify = "a media file is known by its container, and one of another container or none is E099"
)]
fn a_media_file_is_known_by_its_container() {
    let mp4 = b"\0\0\0\x18ftypisom\0\0\0\0isommp42";
    assert!(found("media", json!({}), mp4).is_empty());
    let webm_only = found("media", json!({"formats": ["webm"]}), mp4);
    assert_eq!(codes(&webm_only), ["E099"]);
    assert!(
        webm_only[0].message.contains("is mp4, not one of webm"),
        "{}",
        webm_only[0].message
    );
    assert_eq!(codes(&found("media", json!({}), &png(1, 1))), ["E099"]);
}

/// A PDF of `pages` pages whose catalog and page tree sit after `before` bytes of padding and before `after`.
fn pdf(before: usize, after: usize, pages: u32) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n%".to_vec();
    out.extend(std::iter::repeat_n(b'x', before));
    out.push(b'\n');
    let catalog = out.len();
    out.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let tree = out.len();
    let text = format!("2 0 obj\n<< /Type /Pages /Kids [] /Count {pages} >>\nendobj\n%");
    out.extend_from_slice(text.as_bytes());
    out.extend(std::iter::repeat_n(b'x', after));
    out.push(b'\n');
    let xref = out.len();
    let table = format!(
        "xref\n0 3\n0000000000 65535 f \n{catalog:010} 00000 n \n{tree:010} 00000 n \n\
         trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.extend_from_slice(table.as_bytes());
    out
}

/// The first round's reads of `file`: its head and its tail.
fn ends(file: &[u8]) -> Vec<ProbeOutcome> {
    let length = file.len() as u64;
    let tail = file.len().saturating_sub(64 * 1024);
    vec![
        part(&file[..1024.min(file.len())], 0, length),
        part(&file[tail..], tail as u64, length),
    ]
}

fn round(
    file_outcomes: Vec<ProbeOutcome>,
    args: serde_json::Value,
    carry: Option<serde_json::Value>,
) -> PlannedItem {
    MockHost::new(crate::specforge_extension_build()).judge_round(
        "pdf",
        "a.pdf",
        args,
        file_outcomes,
        carry,
    )
}

#[specforge_test(
    behavior = "checks_media",
    verify = "a pdf reads its head and tail, asks for the spans its page tree needs, then counts its pages against max_pages"
)]
fn a_pdf_asks_for_the_spans_its_page_tree_needs() {
    let small = pdf(10, 10, 4);
    assert!(round(ends(&small), json!({"max_pages": 4}), None)
        .findings
        .is_empty());
    let over = round(ends(&small), json!({"max_pages": 3}), None);
    assert_eq!(codes(&over.findings), ["W177"]);
    assert!(
        over.findings[0].message.contains("has 4 pages"),
        "{}",
        over.findings[0].message
    );

    let large = pdf(200 * 1024, 100 * 1024, 12);
    // Without max_pages, a head and a tail that read as a PDF are enough.
    assert!(round(ends(&large), json!({}), None).findings.is_empty());
    let first = round(ends(&large), json!({"max_pages": 10}), None);
    let [Probe::FsRead {
        range: ByteRange::Span { offset, len },
        ..
    }] = first.probes[..]
    else {
        panic!("one span: {:?}", first.probes);
    };
    assert_eq!(first.carry, Some(json!(2)));
    let mut outcomes = ends(&large);
    let end = (offset as usize + len as usize).min(large.len());
    outcomes.push(part(
        &large[offset as usize..end],
        offset,
        large.len() as u64,
    ));
    let second = round(outcomes, json!({"max_pages": 10}), first.carry);
    assert!(second.probes.is_empty());
    assert_eq!(codes(&second.findings), ["W177"]);
    // In the last round, a page tree still out of reach leaves max_pages unchecked.
    let last = round(ends(&large), json!({"max_pages": 10}), Some(json!(3)));
    assert_eq!(codes(&last.findings), ["W176"]);
    assert_eq!(codes(&found("pdf", json!({}), b"GIF89a")), ["E099"]);
}

#[specforge_test(
    behavior = "checks_media",
    verify = "a remote file the network did not serve is a machine warning, and one that is gone is E099"
)]
fn a_remote_file_not_served_is_a_warning_and_one_gone_is_e099() {
    let http = |class: OutcomeClass, status: Option<u16>| {
        answer(
            class,
            OutcomeData::Http {
                status,
                final_url: URL.into(),
                headers: Vec::new(),
                body: Vec::new(),
                truncated: false,
                start: 0,
                total_len: None,
            },
        )
    };
    for (class, status, code) in [
        (OutcomeClass::Unreachable(Unreachable::Dns), None, "W172"),
        (OutcomeClass::RangeIgnored, Some(200), "W172"),
        (OutcomeClass::Status(404), Some(404), "E099"),
        (OutcomeClass::Status(403), Some(403), "W182"),
        (OutcomeClass::Status(503), Some(503), "W181"),
    ] {
        let found = judged("image", URL, json!({}), http(class, status));
        assert_eq!(codes(&found), [code], "{class:?}");
    }
}
