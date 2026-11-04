//! `@specforge/checks`' media family as a project runs it: the committed blob reading the project's own files, and
//! remote ones through `media-remote`.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_common::Diagnostic;
use specforge_effects::EffectsMode;
use specforge_effects::testing::{FakeHttp, MemoryConsent};
use specforge_project::testing::TestProject;

use crate::value_probes::Scene;

/// A feature whose `refs` holds `refs` (each quoted), checked by `checker` with `args`.
fn refs_project(refs: &[&str], checker: &str, args: Value) -> TestProject {
    let items: Vec<String> = refs.iter().map(|r| format!("\"{r}\"")).collect();
    let checker = format!("@specforge/checks:{checker}");
    TestProject::new()
        .config(|c| {
            c["checks"] = json!({ "fields": { "feature.refs": [{ "checker": checker, "args": args }] } });
        })
        .enabling(&["@specforge/product", "@specforge/checks"])
        .file(
            "a.spec",
            format!(
                "feature f \"F\" {{\n  status planned\n  problem \"p\"\n  solution \"s\"\n  refs [{}]\n}}\n",
                items.join(", ")
            ),
        )
}

fn coded(diagnostics: &[Diagnostic], code: &str) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.code == code)
        .cloned()
        .collect()
}

/// The diagnostics of the media family's codes, and of the reads it reports.
fn media_findings(diagnostics: &[Diagnostic]) -> Vec<&Diagnostic> {
    const CODES: [&str; 9] = [
        "E016", "E099", "E103", "E113", "W172", "W173", "W176", "W177", "W181",
    ];
    diagnostics
        .iter()
        .filter(|d| CODES.contains(&d.code.as_str()))
        .collect()
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    out
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

/// The repository's `assets/brand` files whose name passes `keep`, copied into `project`, and their paths there.
fn brand(project: TestProject, keep: impl Fn(&str) -> bool) -> (TestProject, Vec<String>) {
    let dir = specforge_repo_root::repo_root().join("assets/brand");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("assets/brand is there")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| keep(name))
        .collect();
    names.sort();
    let mut project = project;
    for name in &names {
        let bytes = std::fs::read(dir.join(name)).expect("a brand file reads");
        project = project.file(&format!("brand/{name}"), bytes);
    }
    let paths = names.iter().map(|name| format!("brand/{name}")).collect();
    (project, paths)
}

#[specforge_test_macros::test(
    behavior = "checks_media",
    verify = "the repository's own brand images and logos pass image and svg"
)]
fn the_repositorys_brand_images_and_logos_pass() {
    for (checker, keep) in [
        (
            "image",
            (|n: &str| n.ends_with(".png") || n.ends_with(".svg")) as fn(&str) -> bool,
        ),
        ("svg", |n: &str| {
            n.starts_with("logo") && n.ends_with(".svg")
        }),
    ] {
        let (_, paths) = brand(TestProject::new(), keep);
        assert!(paths.len() > 5, "{paths:?}");
        let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
        let (project, _) = brand(refs_project(&refs, checker, json!({})), keep);
        let found = project.compile().diagnostics();
        assert!(media_findings(&found).is_empty(), "{checker}: {found:?}");
    }
}

#[specforge_test_macros::test(
    behavior = "checks_media",
    verify = "an svg with an onload attribute is E113 at its line"
)]
fn an_svg_with_an_onload_attribute_is_e113_at_its_line() {
    let project = refs_project(&["clean.svg", "logo.svg"], "svg", json!({}))
        .file("clean.svg", "<svg xmlns=\"http://www.w3.org/2000/svg\"/>")
        .file(
            "logo.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\">\n  <g>\n    <image onload=\"fetch('/x')\"/>\n  </g>\n</svg>\n",
        );
    let found = project.compile().diagnostics();
    let active = coded(&found, "E113");
    assert_eq!(active.len(), 1, "{found:?}");
    assert!(
        active[0]
            .message
            .contains("'logo.svg' carries active content at 3:5"),
        "{}",
        active[0].message
    );
    assert!(
        active[0].message.contains("'onload'"),
        "{}",
        active[0].message
    );
    assert!(!active[0].machine);
}

#[specforge_test_macros::test(behavior = "checks_media", verify = "a png over max_pixels is W177")]
fn a_png_over_max_pixels_is_w177() {
    let project = refs_project(
        &["small.png", "large.png"],
        "image",
        json!({ "max_pixels": 1_000_000 }),
    )
    .file("small.png", png(1000, 1000))
    .file("large.png", png(4000, 3000));
    let found = project.compile().diagnostics();
    let over = coded(&found, "W177");
    assert_eq!(over.len(), 1, "{found:?}");
    assert!(
        over[0].message.contains("'large.png' is 4000x3000"),
        "{}",
        over[0].message
    );
}

#[specforge_test_macros::test(behavior = "checks_media", verify = "a pdf over max_pages is W177")]
fn a_pdf_over_max_pages_is_w177() {
    // The page tree sits past the head and the tail the first round reads: a second round reads it.
    let project = refs_project(
        &["short.pdf", "long.pdf"],
        "pdf",
        json!({ "max_pages": 10 }),
    )
    .file("short.pdf", pdf(10, 10, 4))
    .file("long.pdf", pdf(200 * 1024, 100 * 1024, 12));
    let found = project.compile().diagnostics();
    let over = coded(&found, "W177");
    assert_eq!(over.len(), 1, "{found:?}");
    assert!(
        over[0].message.contains("'long.pdf' has 12 pages"),
        "{}",
        over[0].message
    );
    assert!(media_findings(&found).len() == 1, "{found:?}");
}

const SMALL: &str = "https://cdn.example/small.pdf";
const LARGE: &str = "https://cdn.example/large.pdf";

fn remote(refs: &[&str], checker: &str, args: Value, http: FakeHttp) -> Scene {
    Scene {
        dir: refs_project(refs, checker, args),
        runtime: specforge_project::testing::component_runtime(),
        http: Arc::new(http),
    }
}

#[specforge_test_macros::test(
    behavior = "checks_media",
    verify = "a remote pdf is read by its tail through media-remote"
)]
fn a_remote_pdf_is_read_by_its_tail_through_media_remote() {
    let http = FakeHttp::new().route(SMALL, 200, pdf(10, 10, 5)).route(
        LARGE,
        200,
        vec![b'%'; 2 * 1024 * 1024],
    );
    let scene = remote(&[SMALL, LARGE], "pdf", json!({ "max_pages": 3 }), http);
    let found = scene.online().diagnostics();
    let over = coded(&found, "W177");
    assert_eq!(over.len(), 1, "{found:?}");
    assert!(
        over[0].message.contains("has 5 pages"),
        "{}",
        over[0].message
    );
    let ignored = coded(&found, "W172");
    assert_eq!(ignored.len(), 1, "{found:?}");
    assert!(
        ignored[0]
            .message
            .contains("'https://cdn.example/large.pdf'"),
        "{}",
        ignored[0].message
    );
    assert!(
        ignored[0].machine,
        "a server that ignores the range is the machine's"
    );
    assert!(coded(&found, "E099").is_empty(), "{found:?}");
    // The head and the tail are asked at once, in either order.
    let mut ranges: Vec<String> = scene
        .http
        .requests()
        .iter()
        .filter(|request| request.url == SMALL)
        .flat_map(|request| request.headers.iter().map(|h| h.value.clone()))
        .collect();
    ranges.sort();
    assert_eq!(ranges, ["bytes=-65536", "bytes=0-1023"]);
}

#[specforge_test_macros::test(
    behavior = "checks_media",
    verify = "an unapproved remote image is W170 and I206, not a finding"
)]
fn an_unapproved_remote_image_is_w170_and_i206_not_a_finding() {
    let image = "https://cdn.example/a.png";
    let scene = remote(
        &[image],
        "image",
        json!({}),
        FakeHttp::new().route(image, 200, png(1, 1)),
    );
    let found = scene
        .checked(EffectsMode::Auto, &MemoryConsent::default())
        .diagnostics();
    let asked = coded(&found, "W170");
    assert_eq!(asked.len(), 1, "{found:?}");
    assert!(
        asked[0].message.contains("'media-remote'"),
        "{}",
        asked[0].message
    );
    assert_eq!(coded(&found, "I206").len(), 1, "{found:?}");
    assert!(media_findings(&found).is_empty(), "{found:?}");
    assert!(scene.http.requests().is_empty());
}
