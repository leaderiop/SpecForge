//! The structured family: what each checker plans for its text and finds in canned reads, as the host would send them.

use serde_json::json;
use specforge_protocol_types::{
    ByteRange, CheckFinding, OutcomeClass, OutcomeData, Probe, ProbeOutcome, Root, StatEntry,
};
use specforge_test::prelude::*;

use super::{e103_at, finding, judged, planned};

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

fn read(bytes: &[u8]) -> ProbeOutcome {
    answer(
        OutcomeClass::Ok,
        OutcomeData::Bytes {
            bytes: bytes.to_vec(),
            start: 0,
            length: bytes.len() as u64,
            truncated: false,
        },
    )
}

fn found(checker: &str, text: &str, bytes: &[u8]) -> Vec<CheckFinding> {
    judged(checker, text, json!({}), read(bytes))
}

fn codes(findings: &[CheckFinding]) -> Vec<&str> {
    findings.iter().map(|f| f.code.as_str()).collect()
}

#[specforge_test(
    behavior = "checks_structured",
    verify = "json, yaml and toml read their file whole, up to 1 MiB, under its root"
)]
fn a_structured_checker_reads_its_file_under_its_root() {
    let plan = planned("yaml", "<spec>/ci.yml", json!({}));
    assert_eq!(
        plan.probes,
        [Probe::FsRead {
            root: Root::Project,
            path: "<spec>/ci.yml".into(),
            range: ByteRange::Whole { max: 1024 * 1024 },
        }]
    );
    let plan = planned(
        "markdown_anchor",
        "docs/a.md#install",
        json!({"root": "spec"}),
    );
    assert!(matches!(
        &plan.probes[..],
        [Probe::FsRead { root: Root::Spec, path, .. }] if path == "docs/a.md"
    ));
}

#[specforge_test(
    behavior = "checks_structured",
    verify = "a file that parses passes, a byte order mark included, and one that does not is E097 at its line"
)]
fn a_file_parses_or_is_e097_at_its_line() {
    assert!(found("json", "a.json", b"\xef\xbb\xbf{\"a\": [1]}").is_empty());
    assert!(found("yaml", "a.yml", b"a: [1]\n").is_empty());
    assert!(found("toml", "a.toml", b"a = 1\n").is_empty());
    let broken = found("json", "a.json", b"{\n  \"a\" 1\n}");
    assert_eq!(codes(&broken), ["E097"]);
    assert!(
        broken[0]
            .message
            .starts_with("'a.json' does not parse as JSON: ")
            && broken[0].message.ends_with(" at 2:7"),
        "{}",
        broken[0].message
    );
    let bytes = found("toml", "a.toml", b"a = \"\xff\"\n");
    assert!(
        bytes[0].message.contains("is not UTF-8 at 1:6"),
        "{bytes:?}"
    );
}

#[specforge_test(
    behavior = "checks_structured",
    verify = "a missing file is E016, and one over the read or no regular file is W176"
)]
fn a_file_that_is_not_read_whole_is_told_why() {
    let missing = answer(
        OutcomeClass::Missing,
        OutcomeData::Stat {
            entry: None,
            size: 0,
            near: Some("a.json".into()),
        },
    );
    let told = judged("json", "b.json", json!({}), missing);
    assert_eq!(codes(&told), ["E016"]);
    assert_eq!(told[0].help.as_deref(), Some("did you mean 'a.json'?"));
    let over = answer(
        OutcomeClass::Ok,
        OutcomeData::Bytes {
            bytes: b"{".to_vec(),
            start: 0,
            length: 3 << 20,
            truncated: true,
        },
    );
    let told = judged("pointer", "big.json#/a", json!({}), over);
    assert_eq!(codes(&told), ["W176"]);
    assert_eq!(told[0].at, Some([0, 8]));
    let dir = answer(
        OutcomeClass::Denied,
        OutcomeData::Stat {
            entry: Some(StatEntry::Dir),
            size: 0,
            near: None,
        },
    );
    assert_eq!(codes(&judged("toml", "d", json!({}), dir)), ["W176"]);
}

#[specforge_test(
    behavior = "checks_structured",
    verify = "a pointer that names nothing is E098 on its fragment, naming the deepest part that exists"
)]
fn a_pointer_is_resolved_in_the_files_tree() {
    let doc = b"a:\n  b: [x, y]\n";
    assert!(found("pointer", "c.yaml#/a/b/1", doc).is_empty());
    // An empty fragment, or none, points at the whole document.
    assert!(found("pointer", "c.yaml#", doc).is_empty());
    assert!(found("pointer", "c.yaml", doc).is_empty());
    let told = found("pointer", "c.yaml#/a/c", doc);
    assert_eq!(codes(&told), ["E098"]);
    assert_eq!(told[0].at, Some([6, 11]));
    assert_eq!(
        told[0].message,
        "'/a/c' names nothing in 'c.yaml': '/a' is the deepest part that exists"
    );
    let told = found("pointer", "c.yaml#/z", doc);
    assert!(told[0].message.contains("the document itself"), "{told:?}");
    let broken = found("pointer", "c.yaml#/a", b"a: [\n");
    assert_eq!(codes(&broken), ["E097"]);
}

#[specforge_test(
    behavior = "checks_structured",
    verify = "a pointer that is malformed is E103 on its token, and one whose format is unknown is E097"
)]
fn a_pointer_is_well_formed_and_its_format_known() {
    assert_eq!(e103_at("pointer", "a.json#/x~2", json!({})), [7, 11]);
    assert_eq!(e103_at("pointer", "a.json#x", json!({})), [7, 8]);
    let unknown = finding("pointer", "data.txt#/a", json!({})).unwrap();
    assert_eq!(unknown.code, "E097");
    assert!(planned("pointer", "data.txt#/a", json!({"format": "toml"}))
        .findings
        .is_empty());
}

#[specforge_test(
    behavior = "checks_structured",
    verify = "a markdown anchor that is no heading or html id is E098 on its fragment, listing the headings"
)]
fn a_markdown_anchor_is_a_heading_or_an_html_id() {
    let md = b"# Install\n## Use it\n```\n# not\n```\n<a name=\"old\"></a>\n";
    assert!(found("markdown_anchor", "a.md#install", md).is_empty());
    assert!(found("markdown_anchor", "a.md#use-it", md).is_empty());
    assert!(found("markdown_anchor", "a.md#old", md).is_empty());
    assert!(found("markdown_anchor", "a.md", md).is_empty());
    let told = found("markdown_anchor", "a.md#not", md);
    assert_eq!(codes(&told), ["E098"]);
    assert_eq!(told[0].at, Some([4, 8]));
    assert_eq!(told[0].message, "'a.md' has no anchor '#not'");
    assert_eq!(
        told[0].help.as_deref(),
        Some("its headings: #install, #use-it")
    );
    let many: String = (0..10).map(|n| format!("# h{n}\n")).collect();
    let told = found("markdown_anchor", "m.md#x", many.as_bytes());
    assert_eq!(
        told[0].help.as_deref(),
        Some("its headings: #h0, #h1, #h2, #h3, #h4, #h5, #h6, #h7 and 2 more")
    );
}
