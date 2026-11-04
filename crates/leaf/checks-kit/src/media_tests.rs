//! The media sniffers against headers written byte by byte: no binary fixture is committed.

use crate::sniff::{CONTAINERS, Image, container, image};
use crate::svg::{Active, At, Refusal, is_svg, scan};

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    out
}

fn gif(width: u16, height: u16) -> Vec<u8> {
    let mut out = b"GIF89a".to_vec();
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0]);
    out
}

fn jpeg(width: u16, height: u16) -> Vec<u8> {
    let mut out = vec![0xff, 0xd8, 0xff, 0xe0, 0, 16];
    out.extend_from_slice(b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0");
    out.extend_from_slice(&[0xff, 0xc0, 0, 17, 8]);
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&[3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1, 0xff, 0xd9]);
    out
}

fn webp(width: u32, height: u32) -> Vec<u8> {
    let mut out = b"RIFF\x16\0\0\0WEBPVP8X\x0a\0\0\0\0\0\0\0".to_vec();
    out.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
    out.extend_from_slice(&(height - 1).to_le_bytes()[..3]);
    out
}

const SVG: &[u8] = b"<?xml version=\"1.0\"?>\n<!-- a logo -->\n<svg xmlns=\"http://www.w3.org/2000/svg\" \
    viewBox=\"0 0 10 10\"><a href=\"https://example.com/\"><rect width=\"10\" height=\"10\"/></a></svg>\n";

#[test]
fn an_image_reads_its_format_and_size() {
    let sized = |format, width, height| {
        Some(Image {
            format,
            size: Some((width, height)),
        })
    };
    assert_eq!(image(&png(640, 480)), sized("png", 640, 480));
    assert_eq!(image(&gif(16, 9)), sized("gif", 16, 9));
    assert_eq!(image(&jpeg(64, 32)), sized("jpeg", 64, 32));
    assert_eq!(image(&webp(300, 200)), sized("webp", 300, 200));
    assert_eq!(
        image(SVG),
        Some(Image {
            format: "svg",
            size: None
        })
    );
    // A header cut before its size is still its format.
    assert_eq!(image(&png(640, 480)[..14]).map(|i| i.format), Some("png"));
    assert_eq!(
        image(&png(1 << 20, 1 << 20)).and_then(Image::pixels),
        Some(1 << 40)
    );
    for not in [
        &b"%PDF-1.7\n"[..],
        b"plain text, not an image",
        b"<html></html>",
        b"",
    ] {
        assert_eq!(image(not), None, "{not:?}");
    }
}

/// An EBML header whose `DocType` is `doc_type`.
fn ebml(doc_type: &[u8]) -> Vec<u8> {
    let mut body = vec![0x42, 0x86, 0x81, 0x01];
    body.extend_from_slice(&[0x42, 0x82, 0x80 | doc_type.len() as u8]);
    body.extend_from_slice(doc_type);
    let mut out = vec![0x1a, 0x45, 0xdf, 0xa3, 0x80 | body.len() as u8];
    out.extend_from_slice(&body);
    out
}

fn iso(brand: &[u8; 4]) -> Vec<u8> {
    let mut out = b"\0\0\0\x18ftyp".to_vec();
    out.extend_from_slice(brand);
    out.extend_from_slice(b"\0\0\0\0isommp42");
    out
}

#[test]
fn a_container_is_known_by_its_signature() {
    let id3_then = |frame: &[u8]| {
        let mut out = b"ID3\x04\0\0\0\0\0\x02\0\0".to_vec();
        out.extend_from_slice(frame);
        out
    };
    let cases: [(Vec<u8>, &str); 13] = [
        (iso(b"isom"), "mp4"),
        (iso(b"qt  "), "mov"),
        (iso(b"M4A "), "m4a"),
        (ebml(b"webm"), "webm"),
        (ebml(b"matroska"), "mkv"),
        (b"RIFF\0\0\0\0AVI LIST".to_vec(), "avi"),
        (b"RIFF\0\0\0\0WAVEfmt ".to_vec(), "wav"),
        (vec![0xff, 0xfb, 0x90, 0x64], "mp3"),
        (id3_then(&[0xff, 0xfb, 0x90, 0x64]), "mp3"),
        (b"OggS\0\x02".to_vec(), "ogg"),
        (b"fLaC\0\0\0\x22".to_vec(), "flac"),
        (vec![0xff, 0xf1, 0x50, 0x80], "aac"),
        (id3_then(&[0xff, 0xf1, 0x50, 0x80]), "aac"),
    ];
    for (head, name) in &cases {
        assert_eq!(container(head), Some(*name), "{head:?}");
    }
    let named: std::collections::BTreeSet<_> = cases.iter().map(|(_, name)| *name).collect();
    assert_eq!(named.len(), CONTAINERS.len(), "every container has a case");
    // Images and look-alikes are no media container.
    for not in [
        iso(b"avif"),
        iso(b"heic"),
        webp(10, 10),
        ebml(b"other"),
        vec![0xff, 0xff, 0xff, 0xff],
        b"%PDF-1.7".to_vec(),
        Vec::new(),
    ] {
        assert_eq!(container(&not), None, "{not:?}");
    }
}

fn active(svg: &str) -> Option<Active> {
    scan(svg.as_bytes()).expect("well formed")
}

#[test]
fn svg_active_content_is_found_at_its_line() {
    assert_eq!(active(std::str::from_utf8(SVG).unwrap_or_default()), None);
    let onload =
        active("<svg>\n  <g>\n    <rect onload=\"alert(1)\"/>\n  </g>\n</svg>").expect("found");
    assert_eq!(onload.at, At { line: 3, column: 5 });
    assert!(
        onload.what.contains("'onload' on <rect>"),
        "{}",
        onload.what
    );
    for svg in [
        "<svg><script>alert(1)</script></svg>",
        "<svg><svg:SCRIPT xmlns:svg=\"x\"/></svg>",
        "<svg><a href=\"javascript:alert(1)\"/></svg>",
        "<svg><a xlink:href=\" \tJaVa\nScRiPt:alert(1)\"/></svg>",
        "<svg><a href=\"&#106;avascript:alert(1)\"/></svg>",
        "<svg><a><set attributeName=\"href\" to=\"javascript:alert(1)\"/></a></svg>",
        "<svg><animate values=\"https://a;javascript:alert(1)\"/></svg>",
        "<!DOCTYPE svg [<!ENTITY x \"&lt;script&gt;\">]><svg>&x;</svg>",
        "<!DOCTYPE svg [<!ENTITY j \"javascript:\">]><svg><a href=\"&j;alert(1)\"/></svg>",
    ] {
        assert!(active(svg).is_some(), "{svg}");
    }
    // The first one found is reported.
    let first =
        active("<svg><a href=\"#x\"/>\n<script/><rect onclick=\"f()\"/></svg>").expect("found");
    assert_eq!(first.at.line, 2);
    assert!(first.what.contains("<script>"), "{}", first.what);
}

#[test]
fn an_svg_that_is_not_well_formed_or_not_svg_is_refused() {
    let not_xml = |svg: &str| matches!(scan(svg.as_bytes()), Err(Refusal::NotXml { .. }));
    for svg in [
        "<svg><g></svg>",
        "<svg></svg><svg></svg>",
        "<svg></svg>text",
        "<svg><g>",
        "<svg a=\"1\" a=\"2\"></svg>",
        "<svg><!-- a -- b --></svg>",
    ] {
        assert!(not_xml(svg), "{svg}: {:?}", scan(svg.as_bytes()));
    }
    let Err(Refusal::NotXml { at, .. }) = scan(b"<svg>\n  <g>\n  </h>\n</svg>") else {
        panic!("a mismatched end tag is not well formed");
    };
    assert_eq!(at.line, 3);
    assert_eq!(
        scan(b"<html><svg/></html>"),
        Err(Refusal::NotSvg {
            root: "html".to_string()
        })
    );
    assert_eq!(scan(b"<?xml version=\"1.0\"?>\n"), Err(Refusal::NoRoot));
    let deep = format!("<svg>{}{}</svg>", "<g>".repeat(300), "</g>".repeat(300));
    assert!(not_xml(&deep));
    assert!(is_svg(SVG));
    assert!(
        is_svg(b"\xef\xbb\xbf<svg/>"),
        "a byte order mark is skipped"
    );
    assert!(!is_svg(b"<html/>") && !is_svg(&png(1, 1)));
}

#[test]
fn no_media_input_panics() {
    for sample in [
        png(8, 8),
        gif(8, 8),
        jpeg(8, 8),
        webp(8, 8),
        ebml(b"webm"),
        iso(b"isom"),
        SVG.to_vec(),
    ] {
        for cut in 0..=sample.len() {
            let _ = (
                image(&sample[..cut]),
                container(&sample[..cut]),
                scan(&sample[..cut]),
            );
        }
        for at in 0..sample.len() {
            for flip in [0x01, 0x80, 0xff] {
                let mut flipped = sample.clone();
                flipped[at] ^= flip;
                let _ = (image(&flipped), container(&flipped), scan(&flipped));
            }
        }
    }
}
