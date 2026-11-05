//! The syntax of the links, files and git families.

use crate::files::{DigestPin, LineEnds, LineRange, Text, glob};
use crate::git::{FileAt, ObjectName, RefName};
use crate::html::anchors;
use crate::links::{UrlParts, fragment_id, host_matches};

#[test]
fn url_parts_point_at_the_scheme_and_fragment() {
    let text = "https://a.example/page?q=1#top";
    let parts = UrlParts::parse(text).unwrap();
    assert_eq!(&text[parts.scheme.clone()], "https");
    assert_eq!(&text[parts.fragment.clone().unwrap()], "top");
    assert_eq!(parts.credentials, None);
    assert_eq!(parts.url.host_str(), Some("a.example"));
    assert_eq!(UrlParts::parse("https://a.example").unwrap().fragment, None);
}

#[test]
fn a_fragment_id_is_percent_decoded_as_utf8() {
    assert_eq!(fragment_id("caf%C3%A9"), "café");
    assert_eq!(fragment_id("a%20b"), "a b");
    assert_eq!(fragment_id("plain"), "plain");
    assert_eq!(fragment_id("bad%FF"), "bad\u{FFFD}");
}

#[test]
fn url_parts_flag_credentials() {
    let text = "https://user:secret@a.example/x";
    let parts = UrlParts::parse(text).unwrap();
    assert_eq!(&text[parts.credentials.unwrap()], "user:secret");
    let text = "https://a:b@c@a.example/";
    assert_eq!(
        &text[UrlParts::parse(text).unwrap().credentials.unwrap()],
        "a:b@c"
    );
    // The parser's reading of a backslash, not a split by hand: this host is evil.example, and there is no user.
    let parts = UrlParts::parse("https://evil.example\\@good.example/").unwrap();
    assert_eq!(parts.url.host_str(), Some("evil.example"));
    assert_eq!(parts.credentials, None);
}

#[test]
fn url_parts_refuse_what_the_parser_would_rewrite() {
    assert_eq!(
        UrlParts::parse(" https://a.example").unwrap_err().at,
        Some(0..1)
    );
    assert_eq!(
        UrlParts::parse("https://a\t.example").unwrap_err().at,
        Some(9..10)
    );
    assert!(UrlParts::parse("/relative").is_err());
    assert!(UrlParts::parse("").is_err());
    // A non-ASCII host keeps every byte range on a character boundary.
    let text = "https://bücher.example/#ñ";
    let parts = UrlParts::parse(text).unwrap();
    assert_eq!(&text[parts.fragment.unwrap()], "ñ");
}

#[test]
fn host_matches_reads_a_wildcard_as_a_subdomain() {
    assert!(host_matches("a.example", "A.example"));
    assert!(host_matches("*.example", "a.example"));
    assert!(host_matches("*.example", "a.b.example"));
    assert!(!host_matches("*.example", "example"));
    assert!(!host_matches("*.example", "evilexample"));
    assert!(!host_matches("a.example", "a.example.evil"));
}

#[test]
fn html_anchors_reads_id_and_name_and_ignores_scripts() {
    let page = br#"<!doctype html><!-- <p id="commented"> -->
        <h1 ID="top">t</h1><a name='old'>o</a><input name="field"><div id=bare class=x></div>
        <script>var s = '<p id="scripted">';</script><title><p id="titled"></title>
        <p data-id="other" id="" title="<p id='in-a-value'>">"#;
    let found = anchors(page);
    let found: Vec<&str> = found.iter().map(String::as_str).collect();
    assert_eq!(found, ["", "bare", "old", "top"]);
}

#[test]
fn html_anchors_reads_a_page_cut_anywhere() {
    let page = br#"<a name="x" id='y'><script>a</script><!-- c --><p id=z>"#;
    for end in 0..page.len() {
        let _ = anchors(&page[..end]);
    }
    assert!(anchors(b"<").is_empty());
    assert!(anchors(b"< <<<").is_empty());
    assert!(anchors(b"<a = = =>").is_empty());
    assert!(anchors(&vec![b'<'; 100_000]).is_empty());
    assert!(anchors(&b"<a ".repeat(30_000)).is_empty());
}

#[test]
fn html_anchors_survives_hostile_pages() {
    let names = |page: &[u8]| anchors(page).into_iter().collect::<Vec<_>>();
    // Unclosed: a tag, a quoted value, a comment and a raw-text element each run to the end.
    assert_eq!(names(br#"<p id="open"#), ["open"]);
    assert_eq!(names(br#"<p id=bare"#), ["bare"]);
    assert_eq!(names(br#"<p id="x <p id='y'>"#), ["x <p id='y'>"]);
    assert!(names(br#"<!-- <p id="hidden">"#).is_empty());
    assert!(names(br#"<script><p id="hidden">"#).is_empty());
    // A comment does not nest: the first `-->` closes it, as in HTML.
    assert_eq!(names(br#"<!-- <!-- --> <p id="after"> -->"#), ["after"]);
    assert!(names(br#"<!-- <!-- <p id="in"> -->"#).is_empty());
    // A raw-text element closes only on its own name, in any case.
    assert_eq!(
        names(br#"<script></scrip></style></SCRIPT ><p id="s">"#),
        ["s"]
    );
    // Bytes that are not UTF-8 name an anchor lossily, and never fail.
    assert_eq!(names(b"<p id=\"\xff\">"), ["\u{fffd}"]);
    // Huge: one 4 MiB value, and 200,000 attributes, each read once.
    let mut huge = b"<p id=\"".to_vec();
    huge.resize(huge.len() + (4 << 20), b'a');
    huge.extend_from_slice(b"\"><a id=next>");
    let found = anchors(&huge);
    assert_eq!(found.len(), 2);
    assert!(found.contains("next"));
    let many = [b"<p ".as_slice(), &b"x=1 ".repeat(200_000), b"id=last>"].concat();
    assert_eq!(names(&many), ["last"]);
    assert!(names(&b"<script></".repeat(100_000)).is_empty());
}

#[test]
fn line_range_parses_a_line_and_a_span_and_refuses_start_after_end() {
    let span = LineRange::parse("src/a.rs:10-40").unwrap();
    assert_eq!((span.path, span.start, span.end), ("src/a.rs", 10, 40));
    let line = LineRange::parse("a.rs:7").unwrap();
    assert_eq!((line.start, line.end), (7, 7));
    assert_eq!(LineRange::parse("a.rs:40-10").unwrap_err().at, Some(5..10));
    assert!(LineRange::parse("a.rs").is_err());
    assert!(LineRange::parse("a.rs:0").is_err());
    assert!(LineRange::parse("a.rs:x").is_err());
    assert!(LineRange::parse("a.rs:+1").is_err());
    assert!(LineRange::parse(":1").is_err());
    assert!(LineRange::parse("a.rs:99999999999").is_err());
}

#[test]
fn text_classify_tells_lf_crlf_mixed_and_a_bom() {
    assert_eq!(Text::classify(b"a\nb\n").line_ends, LineEnds::Lf);
    assert_eq!(Text::classify(b"a\r\nb\r\n").line_ends, LineEnds::Crlf);
    assert_eq!(Text::classify(b"a\r\nb\n").line_ends, LineEnds::Mixed);
    assert_eq!(Text::classify(b"a").line_ends, LineEnds::None);
    assert!(Text::classify(b"\xEF\xBB\xBFa").bom);
    assert!(!Text::classify(b"a").bom);
    assert!(Text::classify("é".as_bytes()).utf8);
    assert!(!Text::classify(b"\xFF").utf8);
}

#[test]
fn digest_pin_needs_64_hex() {
    let hex = "A".repeat(64);
    let text = format!("a@b.bin@sha256:{hex}");
    let pin = DigestPin::parse(&text).unwrap();
    assert_eq!(pin.path, "a@b.bin");
    assert_eq!(pin.sha256, "a".repeat(64));
    assert_eq!(pin.at, 15..79);
    assert!(DigestPin::parse("a.bin").is_err());
    assert!(DigestPin::parse(&format!("@sha256:{hex}")).is_err());
    assert_eq!(
        DigestPin::parse("a@sha256:abc").unwrap_err().at,
        Some(9..12)
    );
    let wrong = format!("a@sha256:{}g", "a".repeat(63));
    assert!(DigestPin::parse(&wrong).is_err());
}

#[test]
fn glob_validate_keeps_a_pattern_relative() {
    assert!(glob::validate("docs/**/*.md").is_ok());
    assert!(glob::validate("").is_err());
    assert!(glob::validate("/etc/*").is_err());
    assert_eq!(glob::validate("a/../b").unwrap_err().at, Some(2..4));
    assert!(glob::validate("a\\b").is_err());
    assert!(glob::validate(&"a".repeat(1025)).is_err());
}

#[test]
fn ref_name_follows_check_ref_format() {
    for ok in ["main", "feature/a-b", "v1.2.3", "a@b", "é"] {
        assert_eq!(RefName::check(ok), Ok(()), "{ok}");
    }
    for bad in [
        "", "@", "-x", "/a", "a/", "a//b", "a.", "a..b", "a@{b", "a b", "a~b", "a^b", "a:b", "a?b",
        "a*b", "a[b", "a\\b", ".a", "a/.b", "a.lock", "a.lock/b", "a\u{7f}b", "a\nb",
    ] {
        assert!(RefName::check(bad).is_err(), "{bad:?}");
    }
    assert_eq!(RefName::check("a..b").unwrap_err().at, Some(1..3));
    assert_eq!(RefName::check("a/.b").unwrap_err().at, Some(2..3));
}

#[test]
fn object_name_takes_4_to_64_hex() {
    assert_eq!(ObjectName::parse("ABCD").unwrap().as_str(), "abcd");
    assert!(ObjectName::parse(&"a".repeat(64)).is_ok());
    assert!(ObjectName::parse("abc").is_err());
    assert!(ObjectName::parse(&"a".repeat(65)).is_err());
    assert_eq!(ObjectName::parse("abcg").unwrap_err().at, Some(3..4));
    // A revision expression or an option is no name.
    assert!(ObjectName::parse("HEAD~1").is_err());
    assert!(ObjectName::parse("--all").is_err());
}

#[test]
fn file_at_splits_a_revision_from_a_path() {
    let at = FileAt::parse("main:src/a.rs").unwrap();
    assert_eq!((at.rev, at.path), ("main", "src/a.rs"));
    assert!(FileAt::parse("abcd1234:a").is_ok());
    assert!(FileAt::parse("main").is_err());
    assert!(FileAt::parse("main:").is_err());
    assert!(FileAt::parse("main:/a").is_err());
    assert!(FileAt::parse("main:a/../b").is_err());
    assert_eq!(FileAt::parse("--all:a").unwrap_err().at, Some(0..5));
    assert!(FileAt::parse("HEAD~1:a").is_err());
}
