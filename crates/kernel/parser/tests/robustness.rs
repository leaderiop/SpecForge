//! What the parser does with text a user is still typing.

use proptest::test_runner::TestRunner;
use specforge_parser::lex::{MAX_NESTING, nested_too_deep};
use specforge_parser::testing::{
    corpus, corpus_file, corpus_path, edited_corpus, property_config, rendezvous, rendezvous_pick,
};
use specforge_parser::{FieldValue, Previous, parse, parse_incremental};
use specforge_test_macros::test as specforge_test;

#[specforge_test(
    behavior = "recover_from_syntax_errors",
    verify = "parsing any text yields a file and never panics"
)]
fn parsing_any_edit_of_the_corpus_yields_a_file() {
    let mut runner = TestRunner::new(property_config());
    let result = runner.run(&edited_corpus(), |case| {
        // A panic is a failure that proptest shrinks to the few bytes that matter.
        let file = parse(&case.edited, "p.spec");
        proptest::prop_assert!(
            file.errors.iter().all(|e| e.span.start_line >= 1),
            "an error without a line"
        );
        Ok(())
    });
    if let Err(failure) = result {
        panic!("{failure}");
    }
}

#[specforge_test(
    invariant = "incremental_correctness",
    verify = "a file parsed with its previous text and tree reads as the same file parsed cold"
)]
fn a_reparse_reads_as_a_cold_parse() {
    let mut runner = TestRunner::new(property_config());
    let result = runner.run(&edited_corpus(), |case| {
        let (_, tree) = parse_incremental(&case.original, "p.spec", None);
        let tree = tree.expect("a corpus file has a tree");
        let previous = Previous {
            text: &case.original,
            tree: &tree,
        };
        let (reparsed, _) = parse_incremental(&case.edited, "p.spec", Some(previous));
        let cold = parse(&case.edited, "p.spec");
        let read =
            |file: &specforge_parser::SpecFile| format!("{:?}{:?}", file.entities, file.errors);
        proptest::prop_assert!(
            read(&reparsed) == read(&cold),
            "the reparse differs from a cold parse"
        );
        Ok(())
    });
    if let Err(failure) = result {
        panic!("{failure}");
    }
}

#[specforge_test(
    behavior = "recover_from_syntax_errors",
    verify = "a syntax error quoting a long first line cuts it at a character boundary"
)]
fn a_long_error_line_is_cut_at_a_character_boundary() {
    // The 57th byte falls inside '─' (3 bytes): the cut used to panic.
    let line = "ref rust.fn:x// ── Property Entity Kind ──────────────────────";
    let file = parse(&format!("{line}\n"), "a.spec");
    let quoted = file
        .errors
        .iter()
        .filter_map(|e| e.message.strip_prefix("syntax error: unexpected '"))
        .filter_map(|m| m.strip_suffix("...'"))
        .next()
        .unwrap_or_else(|| panic!("an error quoting the cut line: {:?}", file.errors));
    assert!(quoted.len() <= 57, "{quoted:?}");
    assert!(
        line.starts_with(quoted),
        "{quoted:?} is a prefix of the line"
    );
}

/// `text` parsed on a thread with an editor's stack (tokio's 2 MiB).
fn parsed_on_a_small_stack(text: String) -> specforge_parser::SpecFile {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || parse(&text, "deep.spec"))
        .unwrap()
        .join()
        .unwrap()
}

#[specforge_test(
    behavior = "recover_from_syntax_errors",
    verify = "brackets nested past 32 levels are one parse error, and no tree is read"
)]
fn brackets_nested_past_the_limit_are_one_parse_error() {
    let parens = format!(
        "invariant deep_x \"Deep\" {{\n  expression expr {{\n    {}x < 1{}\n  }}\n}}\n",
        "(".repeat(1000),
        ")".repeat(1000)
    );
    let blocks = format!(
        "thing deep_x \"Deep\" {{\n  meta {} v \"x\" {}\n}}\n",
        "k {".repeat(3000),
        "}".repeat(3000)
    );
    for text in [parens, blocks] {
        let file = parsed_on_a_small_stack(text);
        assert_eq!(file.errors.len(), 1, "{:?}", file.errors);
        assert!(
            file.errors[0]
                .message
                .starts_with("brackets nested more than 32 deep"),
            "{:?}",
            file.errors
        );
    }
}

#[specforge_test(
    behavior = "recover_from_syntax_errors",
    verify = "a control or bidirectional character in the text is one parse error at it, and no tree is read"
)]
fn a_control_character_in_the_text_is_one_parse_error_at_it() {
    // `§` stands for the character under test.
    let held = [
        (
            "a title",
            "feature f1 \"T§t\" {\n  problem \"p\"\n}\n",
            '\u{1b}',
        ),
        (
            "a value",
            "feature f1 \"T\" {\n  problem \"p§\"\n}\n",
            '\u{1b}',
        ),
        (
            "a triple value",
            "feature f1 \"T\" {\n  problem \"\"\"\n    p§\n  \"\"\"\n}\n",
            '\u{1b}',
        ),
        (
            "a comment",
            "feature f1 \"T\" {\n  problem \"p\" // x§y\n}\n",
            '\u{7}',
        ),
        (
            "between tokens",
            "feature f1 \"T\" {\n  problem \"p\" §\n}\n",
            '\u{1b}',
        ),
        (
            "an id",
            "feature f§1 \"T\" {\n  problem \"p\"\n}\n",
            '\u{202e}',
        ),
        ("a C1", "feature f1 \"T§\" {\n}\n", '\u{9b}'),
        ("a line separator", "feature f1 \"T§\" {\n}\n", '\u{2028}'),
        ("a NUL", "feature f1 \"T§\" {\n}\n", '\0'),
        ("a DEL", "feature f1 \"T§\" {\n}\n", '\u{7f}'),
        ("a lone CR", "feature f1 \"T§\" {\n}\n", '\r'),
    ];
    for (what, template, c) in held {
        let at = template.find('§').unwrap();
        let source = template.replace('§', &c.to_string());
        let (file, tree) = parse_incremental(&source, "c.spec", None);
        assert!(tree.is_none(), "{what}: a tree was read");
        assert_eq!(file.errors.len(), 1, "{what}: {:?}", file.errors);
        assert!(file.entities.is_empty(), "{what}");
        let error = &file.errors[0];
        let line = source[..at].matches('\n').count() + 1;
        let col = at - source[..at].rfind('\n').map_or(0, |n| n + 1) + 1;
        assert_eq!(
            (error.span.start_line, error.span.start_col),
            (line, col),
            "{what}"
        );
        assert!(error.message.contains("U+"), "{what}: {}", error.message);
        assert!(!error.message.chars().any(char::is_control), "{what}");
    }
    for (what, source) in [
        (
            "a tab in a value",
            "feature f1 \"T\" {\n  problem \"a\tb\"\n}\n",
        ),
        (
            "a CRLF file",
            "feature f1 \"T\" {\r\n  problem \"p\"\r\n}\r\n",
        ),
        ("a no-break space", "feature f1 \"T\u{a0}\" {\n}\n"),
        ("a leading BOM", "\u{feff}feature f1 \"T\" {\n}\n"),
    ] {
        let (file, tree) = parse_incremental(source, "c.spec", None);
        assert!(
            tree.is_some() && file.errors.is_empty(),
            "{what}: {:?}",
            file.errors
        );
    }
}

#[test]
fn the_nesting_limit_counts_brackets_outside_strings_and_comments() {
    let nested = |n: usize| format!("{}x{}", "(".repeat(n), ")".repeat(n));
    assert_eq!(nested_too_deep(&nested(MAX_NESTING)), None);
    // The 33rd opener is the 33rd byte.
    assert_eq!(nested_too_deep(&nested(MAX_NESTING + 1)), Some(MAX_NESTING));
    // `(`, `[` and `{` count together.
    let mixed = "([{".repeat(11);
    assert_eq!(nested_too_deep(&mixed), Some(MAX_NESTING));
    // Brackets in strings and comments are no brackets.
    let quoted = format!("\"{}\"", "(".repeat(100));
    let triple = format!("\"\"\"{}\"\"\"", "{".repeat(100));
    let comment = format!("// {}\n", "[".repeat(100));
    for text in [quoted, triple, comment] {
        assert_eq!(nested_too_deep(&text), None, "{text:?}");
    }
    // A closer never takes the depth below zero: a stray run of them is not credit.
    let stray = format!("{}{}", ")".repeat(100), nested(MAX_NESTING));
    assert_eq!(nested_too_deep(&stray), None);
    let stray = format!("{}{}", ")".repeat(100), nested(MAX_NESTING + 1));
    assert_eq!(nested_too_deep(&stray), Some(100 + MAX_NESTING));
}

#[test]
fn a_source_nested_past_the_limit_reports_the_first_bracket_past_it() {
    let text = format!("x {}\n", "(".repeat(MAX_NESTING + 5));
    let file = parse(&text, "deep.spec");
    assert_eq!(file.errors.len(), 1, "{:?}", file.errors);
    let error = &file.errors[0];
    assert!(
        error
            .message
            .starts_with("brackets nested more than 32 deep")
    );
    // `x ` is two bytes: the 33rd bracket is column 2 + 33.
    assert_eq!(
        (error.span.start_line, error.span.start_col),
        (1, 2 + MAX_NESTING + 1)
    );
}

#[specforge_test(
    behavior = "recover_from_syntax_errors",
    verify = "a reference holding a colon written bare is one E120 that says to quote it, and no E001 or E003 for it"
)]
fn a_bare_colon_reference_in_a_list_is_one_e120_spanning_it() {
    let file = parse(
        "truth T1 \"One\" {\n  proof [\"ok\", js:a.test.js, run:x, b]\n}\n",
        "a.spec",
    );
    let found: Vec<_> = file
        .errors
        .iter()
        .map(|e| (e.code.to_string(), e.span.start_col, e.span.end_col))
        .collect();
    assert_eq!(
        found,
        [("E120".to_string(), 16, 28), ("E120".to_string(), 30, 35)],
        "{:?}",
        file.errors
    );
    assert_eq!(
        file.errors[0].message,
        "a reference holding ':' is written quoted: \"js:a.test.js\""
    );
    assert_eq!(
        file.errors[0].suggestion.as_deref(),
        Some("write \"js:a.test.js\"")
    );
    // No word of a bare reference stays in the value: nothing links as `js`.
    let Some(FieldValue::List(items)) = file.entities[0].fields.get("proof") else {
        panic!("{:?}", file.entities[0].fields);
    };
    let texts: Vec<_> = items.iter().map(|item| item.text.as_str()).collect();
    assert_eq!(texts, ["ok", "b"]);
}

#[test]
fn a_bare_colon_reference_as_a_single_value_is_one_e120() {
    for (source, text) in [
        ("truth T1 \"One\" {\n  proof js:p1\n}\n", "js:p1"),
        (
            "truth T1 \"One\" {\n  proof js:a.test.js#adds\n}\n",
            "js:a.test.js#adds",
        ),
    ] {
        let file = parse(source, "a.spec");
        assert_eq!(file.errors.len(), 1, "{source}: {:?}", file.errors);
        let error = &file.errors[0];
        assert!(error.code.matches("E120"), "{error:?}");
        assert_eq!(
            (error.span.start_col, error.span.end_col),
            (9, 9 + text.len())
        );
        assert!(error.message.ends_with(&format!("\"{text}\"")));
        // The field the grammar read for the scheme word, and the ones its recovery made up, are gone.
        assert!(
            file.entities[0].fields.get("proof").is_none(),
            "{:?}",
            file.entities[0].fields
        );
    }
}

#[test]
fn a_bare_scheme_kind_ref_is_still_a_provider_ref() {
    let file = parse(
        "truth T1 \"One\" {\n  proof [jira.issue:PROJ-1]\n}\n",
        "a.spec",
    );
    assert!(file.errors.is_empty(), "{:?}", file.errors);
    let Some(FieldValue::List(items)) = file.entities[0].fields.get("proof") else {
        panic!("{:?}", file.entities[0].fields);
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].text, "jira.issue:PROJ-1");
    // Spaced around its colon, a list is no bare reference: the grammar's own error stands.
    let file = parse("truth T1 \"One\" {\n  proof [js : p1]\n}\n", "a.spec");
    assert!(
        file.errors.iter().all(|e| e.code.matches("E001")),
        "{:?}",
        file.errors
    );
}

#[test]
fn a_corpus_case_picks_its_file_by_the_files_own_path() {
    // A score is the key and the file's relative path alone: no position in the corpus, no checkout, no platform.
    let root = specforge_repo_root::repo_root();
    assert_eq!(
        corpus_path(&root.join("spec").join("x.spec")),
        "spec/x.spec"
    );
    assert_eq!(rendezvous(7, "spec/x.spec"), 0xccaf_a0ca_a4d5_77c7);
    let paths: Vec<String> = corpus().iter().map(|(path, _)| corpus_path(path)).collect();
    let reversed: Vec<&String> = paths.iter().rev().collect();
    for key in keys(512) {
        let picked = &paths[rendezvous_pick(key, &paths)];
        assert_eq!(reversed[rendezvous_pick(key, &reversed)], picked);
        assert_eq!(&corpus_path(&corpus_file(key).0), picked);
    }
}

#[test]
fn a_corpus_file_removed_moves_only_the_keys_it_won() {
    let paths: Vec<String> = corpus().iter().map(|(path, _)| corpus_path(path)).collect();
    let gone = paths.len() / 2;
    let kept: Vec<&String> = paths
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != gone)
        .map(|(_, p)| p)
        .collect();
    for key in keys(2048) {
        let before = rendezvous_pick(key, &paths);
        if before != gone {
            assert_eq!(
                kept[rendezvous_pick(key, &kept)],
                &paths[before],
                "key {key}"
            );
        }
    }
}

#[test]
fn every_corpus_file_is_as_likely() {
    // Eight paths, 64 draws each on average: a ring by arc length gives some two or three times others' share.
    let paths: Vec<String> = (0..8)
        .map(|i| format!("spec/behaviors/f{i}.spec"))
        .collect();
    let mut wins = [0usize; 8];
    for key in keys(512) {
        wins[rendezvous_pick(key, &paths)] += 1;
    }
    assert!(wins.iter().all(|&n| (32..=96).contains(&n)), "{wins:?}");
}

/// `n` drawn keys, spread over the whole `u64` range as proptest's would be.
fn keys(n: u64) -> impl Iterator<Item = u64> {
    (0..n).map(|i| i.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ (i << 7))
}
