use specforge_code_facts::coverage_facts;
use specforge_extension_sdk::prelude::Fact;

use super::*;

/// `cargo llvm-cov --lcov` form, hand-written to llvm-cov's `CoverageExporterLcov`: crate `demo` at `/project`.
const LLVM_COV: &str = include_str!("../fixtures/llvm-cov.info");
/// c8 (istanbul lcovonly) form, hand-written: relative `SF`, package `board` at `/project`.
const C8: &str = include_str!("../fixtures/c8.info");
/// geninfo(1) LCOV 2.2 form, hand-written: `llvm-cov.info`'s counts with `FNL`/`FNA` groups, `TN`, `VER`, `#`.
const LCOV2: &str = include_str!("../fixtures/lcov2.info");
/// The served `item` and `package` facts a coverage collector receives, as JSON.
const ITEMS: &str = include_str!("../fixtures/items.json");
/// Recorded with c8 12.0.0 (`--reporter=lcovonly`) on a one-file project, unchanged.
const C8_12: &str = include_str!("../fixtures/c8-12.0.0.info");
/// The served facts of the recorded project: `src/board.js`'s items where the file writes them.
const C8_12_ITEMS: &str = include_str!("../fixtures/c8-12.0.0.items.json");

fn served() -> Vec<Fact> {
    serde_json::from_str(ITEMS).expect("the served facts parse")
}

#[test]
fn a_recorded_c8_12_report_mints_coverage_counting_each_line_once() {
    let files = read(C8_12, "/project").expect("the report reads").files;
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file, "src/board.js");
    let served: Vec<Fact> = serde_json::from_str(C8_12_ITEMS).expect("the served facts parse");
    let facts = coverage_facts(&files, &served);
    let recorded = C8_12.lines().filter(|l| l.starts_with("DA:")).count() as u64;
    let counted: u64 = facts
        .iter()
        .map(|f| f.fields["lines_found"].as_u64().expect("lines_found"))
        .sum();
    assert_eq!(counted, recorded);
    assert!(
        facts.iter().all(|f| f.fields.contains_key("item")),
        "{facts:?}"
    );
    insta::assert_json_snapshot!("c8_12", facts);
}

#[test]
fn an_llvm_cov_report_mints_coverage_citing_its_items() {
    let Tracefile { files, outside } = read(LLVM_COV, "/project").expect("the report reads");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file, "src/lib.rs");
    assert_eq!(outside, Vec::<String>::new());
    insta::assert_json_snapshot!("llvm_cov", coverage_facts(&files, &served()));
}

#[test]
fn an_llvm_cov_report_reads_every_record() {
    let files = read(LLVM_COV, "/project").expect("the report reads").files;
    let file = &files[0];
    assert_eq!(file.functions.len(), 5);
    assert_eq!(
        file.functions[0],
        Function {
            line: 3,
            name: "_RNvCs4fqI2P2rA04_4demo3add".to_string(),
            hits: 4
        }
    );
    assert_eq!(file.lines.len(), 16);
    assert_eq!(
        file.branches,
        [
            Branch {
                line: 29,
                block: "0".to_string(),
                branch: "0".to_string(),
                taken: Some(0)
            },
            Branch {
                line: 29,
                block: "0".to_string(),
                branch: "1".to_string(),
                taken: Some(3)
            },
        ]
    );
}

#[test]
fn a_c8_report_mints_coverage_citing_its_js_items() {
    let files = read(C8, "/project").expect("the report reads").files;
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file, "src/board.js");
    let facts = coverage_facts(&files, &served());
    assert!(facts
        .iter()
        .any(|f| f.id == "cov:js:board/src/board.js::createBoard"));
    insta::assert_json_snapshot!("c8", facts);
}

#[test]
fn an_lcov2_report_reads_the_counts_of_its_fn_twin() {
    let files = read(LCOV2, "/project").expect("the report reads").files;
    let groups: Vec<(usize, &str, u64)> = files[0]
        .functions
        .iter()
        .map(|f| (f.line, f.name.as_str(), f.hits))
        .collect();
    assert_eq!(
        groups,
        [
            (3, "demo::add", 4),
            (12, "<demo::Board>::len", 2),
            (22, "demo::tests::adds", 1),
            (27, "demo::helper::<u8>", 3),
            (27, "demo::helper::<(u8, u16)>", 0),
        ]
    );
    let facts = coverage_facts(&files, &served());
    let twin = coverage_facts(
        &read(LLVM_COV, "/project").expect("the twin reads").files,
        &served(),
    );
    assert_eq!(facts, twin);
    insta::assert_json_snapshot!("lcov2", facts);
}

#[test]
fn an_fna_with_no_fnl_and_an_fnda_with_no_fn_are_kept_at_line_0() {
    let text = "SF:src/a.rs\nFNA:7,1,lost\nFNDA:2,orphan\nFN:4,late\nFNDA:1,late\nend_of_record\n";
    let files = read(text, "/project").expect("the report reads").files;
    let functions: Vec<(usize, &str, u64)> = files[0]
        .functions
        .iter()
        .map(|f| (f.line, f.name.as_str(), f.hits))
        .collect();
    assert_eq!(
        functions,
        [(0, "lost", 1), (0, "orphan", 2), (4, "late", 1)]
    );
}

#[test]
fn the_nth_fnda_of_a_repeated_name_pairs_with_its_nth_fn() {
    let functions = |text: &str| -> Vec<(usize, String, u64)> {
        read(text, "/project").expect("the report reads").files[0]
            .functions
            .iter()
            .map(|f| (f.line, f.name.clone(), f.hits))
            .collect()
    };
    let owned = |rows: &[(usize, &str, u64)]| -> Vec<(usize, String, u64)> {
        rows.iter()
            .map(|(l, n, h)| (*l, n.to_string(), *h))
            .collect()
    };
    // Two impls' `new`: the second FNDA is the second function's, whatever the first one's count.
    let text = "SF:src/a.rs\nFN:3,new\nFN:10,new\nFN:20,other\nFNDA:0,new\nFNDA:5,new\nFNDA:1,other\nend_of_record\n";
    assert_eq!(
        functions(text),
        owned(&[(3, "new", 0), (10, "new", 5), (20, "other", 1)])
    );
    // FNDA first: each takes the line of the FN of its rank; one FNDA more than FNs stays at line 0.
    let text =
        "SF:src/a.rs\nFNDA:2,new\nFNDA:7,new\nFNDA:9,new\nFN:3,new\nFN:10,new\nend_of_record\n";
    assert_eq!(
        functions(text),
        owned(&[(3, "new", 2), (10, "new", 7), (0, "new", 9)])
    );
}

#[test]
fn a_bad_report_is_an_error_naming_its_line() {
    // (report, what the error names)
    let bad: [(&str, &str); 13] = [
        ("garbage", "line 1:"),
        ("TN:\nDA:1,1\n", "line 2:"),
        ("SF:src/a.rs\nDA:x,1\nend_of_record\n", "line 2:"),
        ("SF:src/a.rs\nBRDA:3,0,0\nend_of_record\n", "line 2:"),
        ("SF:src/a.rs\nDA:1,1\n", "line 1:"),
        ("SF:/elsewhere/a.rs\nDA:x,1\nend_of_record\n", "line 2:"),
        ("SF:src/a.rs\nSF:src/b.rs\nend_of_record\n", "line 2:"),
        ("end_of_record\n", "line 1:"),
        ("SF:src/a.rs\nFN:x,add\nend_of_record\n", "line 2:"),
        ("SF:src/a.rs\nFNDA:1\nend_of_record\n", "line 2:"),
        ("SF:src/a.rs\n\nFNA:0,1\nend_of_record\n", "line 3:"),
        ("SF:src/a.rs\nDA:1,-1\nend_of_record\n", "line 2:"),
        (
            "SF:src/a.rs\nDA:1,18446744073709551616\nend_of_record\n",
            "line 2:",
        ),
    ];
    for (text, named) in bad {
        match read(text, "/project") {
            Err(error) => assert!(error.contains(named), "{text:?}: {error}"),
            Ok(files) => panic!("{text:?} read as {files:?}"),
        }
    }
    assert!(read("lower:case", "/project").is_err());
}

#[test]
fn an_error_naming_an_open_file_shows_it_bounded_and_escaped() {
    let file = format!("src/\u{1b}]0;{}.rs", "a".repeat(5_000));
    for text in [
        format!("SF:{file}\nDA:1,1\n"),
        format!("SF:{file}\nSF:src/b.rs\nend_of_record\n"),
    ] {
        let error = read(&text, "/project").expect_err("refused");
        assert!(
            !error.contains('\u{1b}') && error.contains("\\u{1b}"),
            "{error}"
        );
        assert!(error.len() < 400, "{} bytes", error.len());
    }
}

#[test]
fn an_sf_path_is_trimmed_and_an_empty_one_has_its_own_error() {
    for text in [
        "SF:src/a.rs \nend_of_record\n",
        "SF: src/a.rs\r\t\nend_of_record\n",
    ] {
        let files = read(text, "/project").expect("the report reads").files;
        assert_eq!(files[0].file, "src/a.rs", "{text:?}");
    }
    for text in ["SF:\nend_of_record\n", "SF: \t\nend_of_record\n"] {
        let error = read(text, "/project").expect_err("no path");
        assert!(
            error.contains("line 1:") && error.contains("names no file"),
            "{text:?}: {error}"
        );
        assert!(!error.contains("outside"), "{error}");
    }
}

#[test]
fn crlf_line_endings_read_like_lf() {
    let crlf = LLVM_COV.replace('\n', "\r\n");
    assert_eq!(read(&crlf, "/project"), read(LLVM_COV, "/project"));
}

#[test]
fn a_huge_report_reads_in_linear_time_and_keeps_every_count_exactly() {
    let mut text = String::from("SF:src/a.rs\n");
    for n in 1..=50_000 {
        text.push_str(&format!("FN:{n},f{n}\n"));
    }
    for n in 1..=50_000 {
        text.push_str(&format!("FNDA:{n},f{n}\n"));
    }
    text.push_str("DA:1,18446744073709551615\nend_of_record\n");
    let started = std::time::Instant::now();
    let files = read(&text, "/project").expect("the report reads");
    // Linear pairing reads it in milliseconds; an FNDA scan over the FN list takes minutes.
    assert!(started.elapsed().as_secs() < 5, "{:?}", started.elapsed());
    let files = files.files;
    assert_eq!(files[0].functions.len(), 50_000);
    assert_eq!(files[0].functions[49_999].hits, 50_000);
    assert_eq!(files[0].lines, [(1, u64::MAX)]);
}

#[test]
fn an_empty_text_has_no_file() {
    assert_eq!(read("", "/project"), Ok(Tracefile::default()));
    assert_eq!(read("\n\n", "/project"), Ok(Tracefile::default()));
}

#[test]
fn an_outside_path_is_skipped_counted_once_and_mints_nothing() {
    let text = "SF:src/lib.rs\nDA:1,1\nend_of_record\n\
                SF:/elsewhere/a.rs\nDA:1,4\nend_of_record\n\
                SF:../../etc/passwd\nFN:1,f\nFNDA:1,f\nend_of_record\n\
                SF:/elsewhere/a.rs\nDA:2,4\nend_of_record\n";
    let read = read(text, "/project").expect("the report reads");
    assert_eq!(
        read.files,
        [CoverageFile {
            file: "src/lib.rs".to_string(),
            lines: vec![(1, 1)],
            ..CoverageFile::default()
        }]
    );
    assert_eq!(read.outside, ["/elsewhere/a.rs", "../../etc/passwd"]);
    let only = read_outside("SF:/elsewhere/a.rs\nend_of_record\n");
    assert_eq!(
        (only.files.len(), &only.outside[..]),
        (0, &["/elsewhere/a.rs".to_string()][..])
    );
}

#[test]
fn an_outside_path_is_counted_escaped_and_bounded() {
    let path = format!("/elsewhere/\u{1b}]0;{}.rs", "a".repeat(5_000));
    let read = read_outside(&format!("SF:{path}\nend_of_record\n"));
    let [shown] = &read.outside[..] else {
        panic!("one outside path: {:?}", read.outside);
    };
    assert!(
        !shown.contains('\u{1b}') && shown.contains("\\u{1b}"),
        "{shown}"
    );
    assert!(shown.len() < 200, "{} bytes", shown.len());
}

/// `text` read at `/project`, which must succeed.
fn read_outside(text: &str) -> Tracefile {
    read(text, "/project").expect("the report reads")
}
