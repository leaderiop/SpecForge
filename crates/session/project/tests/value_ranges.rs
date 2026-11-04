//! A read names its byte range (ADR 0336): a prefix, a suffix or a span of a file or of a URL's body, answered with
//! where the bytes start and how long the whole is.

use specforge_effects::EffectsMode;
use specforge_effects::testing::{FakeHttp, MemoryConsent};
use specforge_extension_sdk::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

use crate::value_probes::{EXT, Scene, coded, described};

/// A finding that tells the class, where the bytes start, the whole's length and the bytes read.
fn told(item: &CheckItem) -> Vec<CheckFinding> {
    item.outcomes
        .iter()
        .map(|outcome| {
            let text = match &outcome.data {
                OutcomeData::Bytes {
                    bytes,
                    start,
                    length,
                    ..
                } => format!("{start}/{length}:{}", String::from_utf8_lossy(bytes)),
                OutcomeData::Http {
                    body,
                    start,
                    total_len,
                    ..
                } => format!(
                    "{} {start}/{total_len:?}:{}",
                    outcome.class,
                    String::from_utf8_lossy(body)
                ),
                other => format!("{other:?}"),
            };
            CheckFinding::new("E001", text)
        })
        .collect()
}

/// `name`, which reads `range` of its value: a file when `fs`, else a URL.
fn reader(c: &mut ContributionsBuilder, name: &str, fs: bool, range: ByteRange) {
    c.checker(name, move |k| {
        k.applies_to(&["string", "string_list"])
            .scope(if fs { "files" } else { "links" })
            .codes(&["E001"])
            .plan(move |item: &CheckItem| {
                let probe = match fs {
                    true => Probe::FsRead {
                        root: Root::Project,
                        path: item.text.clone(),
                        range,
                    },
                    false => Probe::HttpGet {
                        url: item.text.clone(),
                        range,
                    },
                };
                PlannedItem::probes(vec![probe])
            })
            .judge(told);
    });
}

fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    described(&mut c);
    c.capability(Capability::http("links").from_values().why("to ask"));
    c.capability(
        Capability::fs("files")
            .ops(&[FsAccess::Read])
            .why("to read"),
    );
    reader(&mut c, "tail", true, ByteRange::Suffix { len: 2 });
    reader(&mut c, "span", true, ByteRange::Span { offset: 1, len: 3 });
    reader(&mut c, "tail_url", false, ByteRange::Suffix { len: 4 });
    reader(&mut c, "head_url", false, ByteRange::Prefix { len: 4 });
    c.kind("service", |k| {
        k.keyword("service");
        for field in ["tail", "span", "tail_url", "head_url"] {
            k.field(field, |f| {
                f.field_type(FieldType::String).check(field);
            });
        }
        k.field("tails", |f| {
            f.field_type(FieldType::StringList).check("tail_url");
        });
    });
    c
}

fn scene(http: FakeHttp, spec: &str) -> Scene {
    Scene::with(http, spec, &[EXT], InProcessRuntime::new().with(extension))
}

fn told_in(scene: &Scene, mode: EffectsMode) -> Vec<String> {
    let found = scene.checked(mode, &MemoryConsent::default()).diagnostics();
    coded(&found, "pin.probes/E001")
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a suffix read serves a file's last bytes, where they start and the file's length"
)]
fn a_suffix_read_serves_the_last_bytes_and_the_files_length() {
    let spec = "service a \"A\" {\n  tail \"a.txt\"\n  span \"a.txt\"\n}\n";
    let scene = scene(FakeHttp::new(), spec);
    scene.dir.write("a.txt", "hello");
    let told = told_in(&scene, EffectsMode::Cached);
    assert_eq!(told, ["3/5:lo", "1/5:ell"]);
}

const SMALL: &str = "https://a.example/small.pdf";
const PARTIAL: &str = "https://a.example/partial.pdf";
const LARGE: &str = "https://a.example/large.pdf";
const PAST: &str = "https://a.example/past.pdf";

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a 200 to a suffix request is sliced when small and range:ignored when large"
)]
fn a_200_to_a_suffix_request_is_sliced_when_small_and_range_ignored_when_large() {
    let http = FakeHttp::new()
        .route(SMALL, 200, "0123456789")
        .route_with(
            PARTIAL,
            206,
            vec![("content-range", "bytes 6-9/10")],
            "6789",
        )
        .route(LARGE, 200, vec![b'x'; 2 * 1024 * 1024])
        .route_with(PAST, 416, vec![("content-range", "bytes */10")], "");
    let spec = format!(
        "service a \"A\" {{\n  tails [\"{SMALL}\", \"{PARTIAL}\", \"{LARGE}\", \"{PAST}\"]\n}}\n"
    );
    let scene = scene(http, &spec);
    let told = told_in(&scene, EffectsMode::Online);
    assert_eq!(
        told,
        [
            "ok 6/Some(10):6789",
            "ok 6/Some(10):6789",
            "range:ignored 0/None:",
            "status:416 0/Some(10):",
        ]
    );
    let asked: Vec<String> = scene
        .http
        .requests()
        .iter()
        .map(|request| request.headers[0].value.clone())
        .collect();
    assert_eq!(asked, ["bytes=-4"; 4]);
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a prefix and a suffix of one URL are two journal records"
)]
fn a_prefix_and_a_suffix_of_one_file_are_two_journal_records() {
    let spec = format!("service a \"A\" {{\n  tail_url \"{SMALL}\"\n  head_url \"{SMALL}\"\n}}\n");
    let scene = scene(FakeHttp::new().route(SMALL, 200, "0123456789"), &spec);
    let live = told_in(&scene, EffectsMode::Online);
    assert_eq!(live, ["ok 6/Some(10):6789", "ok 0/None:0123"]);
    assert_eq!(scene.http.requests().len(), 2);
    assert_eq!(told_in(&scene, EffectsMode::Replay), live);
    assert_eq!(
        scene.http.requests().len(),
        2,
        "both answered from the journal"
    );
}
