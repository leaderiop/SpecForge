//! Ported from gsd-core v1.19.0 tests/complexity-trigger.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! Every case of the analyzer, the leak surface, the properties, the file selection and the threshold/jump evaluation:
//! the same source text, the same expected scores and the same candidates (D-r41-15).
//!
//! Left out, because they test what this port does not have, by name: `treatsMissingBaselineAsEmpty`,
//! `doesNotClobberMalformedBaselineOnReadFailure`, `rejectsWrongShapedBaseline`, `anchorsBaselineOnFirstObservation`,
//! `doesNotAdvanceAnchorOnPlainEvaluate`, `freezesBaselineWhileProposalUntriaged`, `reanchorsBaselineOnDisposition`,
//! `detectsIncrementalCreepAcrossRuns`, `prunesBaselineForDeletedFile`, `leavesNoPartialBaselineWhenWriteFails`,
//! `cleansUpTempFileWhenRenameFails`, `degradesReadBaselineOnDeniedRead` (the baseline file: SpecForge keeps the baseline
//! in the disposed proposals, `refactor.rs`), `locksReasonEnumKeys`, `locksVerdictEnumKeys`, the constants test (the
//! JavaScript module's exports), and the three proposal-file tests (`parseProposal` and the frontmatter fence: the
//! proposal is an entity here). The ordering vector `ordersCandidatesDeterministically` keeps its sources.

use std::collections::BTreeMap;

use super::*;

/// The headline vectors of upstream's suite in one place; the rest of this file is the rest of the suite.
#[test]
fn complexity_scores_match_upstream_vectors() {
    let thirteen = lines(&[
        "function f(x) {",
        "  if (x === 1) { return 1; }",
        "  else if (x === 2) { return 2; }",
        "  for (let i = 0; i < 1; i++) { g(i); }",
        "  for (const y of x) { g(y); }",
        "  for (const k in x) { g(k); }",
        "  let n = 0;",
        "  do { n += 1; } while (n < 1);",
        "  switch (x) {",
        "    case 1: g(1); break;",
        "    default: g(0);",
        "  }",
        "  try { g(x); } catch (e) { g(e); }",
        "  const a = x && 1;",
        "  const b = x || 1;",
        "  const c = x ? 1 : 2;",
        "  return a + b + c;",
        "}",
    ]);
    assert_eq!(scores(&thirteen), [13]);
    assert_eq!(
        scores("function f(x) {\n  const s = `value: ${x ? 1 : 2}`;\n}"),
        [2]
    );
    assert_eq!(scores("function f(a, b, c) {\n  return a / b / c;\n}"), [1]);
    assert_eq!(
        evaluate(
            &[file("a.js", &[("f", 1, 16)])],
            &baseline(&[]),
            THRESHOLD,
            JUMP_DELTA
        )[0]
        .reasons,
        ["threshold"]
    );
}

fn scores(source: &str) -> Vec<u32> {
    analyze_source(source)
        .expect("parseable")
        .iter()
        .map(|f| f.score)
        .collect()
}

fn one(source: &str) -> u32 {
    let found = scores(source);
    assert_eq!(found.len(), 1, "{source}");
    found[0]
}

fn lines(parts: &[&str]) -> String {
    parts.join("\n")
}

// -- counting ---------------------------------------------------------------------------------------------------

#[test]
fn scores_flat_function_as_one() {
    let f = analyze_source(&lines(&["function f() {", "  return 1;", "}"])).expect("ok");
    assert_eq!(f.len(), 1);
    assert_eq!(
        (f[0].name.as_str(), f[0].start_line, f[0].score),
        ("f", 1, 1)
    );
}

#[test]
fn scores_single_if_as_two() {
    assert_eq!(
        one(&lines(&[
            "function f(x) {",
            "  if (x) {",
            "    return 1;",
            "  }",
            "}"
        ])),
        2
    );
}

#[test]
fn counts_each_decision_construct_once() {
    let source = lines(&[
        "function f(x) {",
        "  if (x === 1) { return 1; }",
        "  else if (x === 2) { return 2; }",
        "  for (let i = 0; i < 1; i++) { g(i); }",
        "  for (const y of x) { g(y); }",
        "  for (const k in x) { g(k); }",
        "  let n = 0;",
        "  do { n += 1; } while (n < 1);",
        "  switch (x) {",
        "    case 1: g(1); break;",
        "    default: g(0);",
        "  }",
        "  try { g(x); } catch (e) { g(e); }",
        "  const a = x && 1;",
        "  const b = x || 1;",
        "  const c = x ? 1 : 2;",
        "  return a + b + c;",
        "}",
    ]);
    assert_eq!(one(&source), 13);
}

#[test]
fn does_not_count_bare_else() {
    let source = lines(&[
        "function f(x) {",
        "  if (x) { return 1; }",
        "  else { return 2; }",
        "}",
    ]);
    assert_eq!(one(&source), 2);
}

#[test]
fn does_not_count_switch_default() {
    let source = lines(&[
        "function f(x) {",
        "  switch (x) {",
        "    case 1: return 1;",
        "    default: return 0;",
        "  }",
        "}",
    ]);
    assert_eq!(one(&source), 2);
}

#[test]
fn does_not_conflate_length_with_complexity() {
    let mut parts = vec!["function f() {".to_string()];
    parts.extend((0..200).map(|i| format!("  const v{i} = {i};")));
    parts.push("  return 0;".to_string());
    parts.push("}".to_string());
    assert_eq!(one(&parts.join("\n")), 1);
}

#[test]
fn scores_flat_switch_by_case_count_pinning_known_bias() {
    let mut parts = vec!["function f(x) {".to_string(), "  switch (x) {".to_string()];
    parts.extend((1..=12).map(|i| format!("    case {i}: return {i};")));
    parts.extend([
        "  }".to_string(),
        "  return 0;".to_string(),
        "}".to_string(),
    ]);
    assert_eq!(one(&parts.join("\n")), 13);
}

#[test]
fn attributes_scores_to_each_function() {
    let source = lines(&[
        "function a() {",
        "  return 1;",
        "}",
        "",
        "function b(x) {",
        "  if (x) {",
        "    return 1;",
        "  }",
        "  return 0;",
        "}",
    ]);
    let f = analyze_source(&source).expect("ok");
    assert_eq!(f.len(), 2);
    assert_eq!(
        (f[0].name.as_str(), f[0].start_line, f[0].score),
        ("a", 1, 1)
    );
    assert_eq!(
        (f[1].name.as_str(), f[1].start_line, f[1].score),
        ("b", 5, 2)
    );
}

#[test]
fn does_not_double_count_nested_function() {
    let source = lines(&[
        "function outer() {",
        "  function inner(x) {",
        "    if (x) {",
        "      return 1;",
        "    }",
        "    return 0;",
        "  }",
        "  return inner;",
        "}",
    ]);
    let f = analyze_source(&source).expect("ok");
    assert_eq!(f.len(), 2);
    let score = |name: &str| f.iter().find(|x| x.name == name).expect(name).score;
    assert_eq!(
        score("outer"),
        1,
        "the nested if must not double-count into outer"
    );
    assert_eq!(score("inner"), 2);
}

#[test]
fn detects_all_function_forms() {
    let source = lines(&[
        "const arrow = (x) => x + 1;",
        "const obj = {",
        "  method(x) {",
        "    return x;",
        "  },",
        "};",
        "async function asyncFn() {",
        "  return 1;",
        "}",
        "function* genFn() {",
        "  yield 1;",
        "}",
    ]);
    let found = scores(&source);
    assert_eq!(
        found.len(),
        4,
        "arrow, method shorthand, async function and generator"
    );
    assert!(found.iter().all(|s| *s == 1));
}

// -- the leak surface -------------------------------------------------------------------------------------------

#[test]
fn ignores_decision_keyword_in_comments_strings_and_templates() {
    for inside in [
        "  // if (x) { return 1; }",
        "  /* if (x) { return 1 } && y */",
        "  const s = \"if (x) { return 1; }\";",
        "  const s = `if (x) { return 1; }`;",
    ] {
        assert_eq!(
            one(&lines(&["function f() {", inside, "  return 0;", "}"])),
            1,
            "{inside}"
        );
    }
}

#[test]
fn ignores_alternation_inside_regex_literal() {
    assert_eq!(
        one(&lines(&["function f(s) {", "  return /a|b/.test(s);", "}"])),
        1
    );
}

#[test]
fn does_not_mistake_division_for_regex_literal() {
    assert_eq!(
        one(&lines(&[
            "function f(a, b, c) {",
            "  return a / b / c;",
            "}"
        ])),
        1
    );
}

#[test]
fn does_not_count_optional_chaining_or_nullish_coalescing() {
    assert_eq!(one(&lines(&["function f(a) {", "  return a?.b;", "}"])), 1);
    assert_eq!(
        one(&lines(&["function f(x, y) {", "  return x ?? y;", "}"])),
        1
    );
}

#[test]
fn handles_escaped_quote_without_swallowing_code() {
    let source = lines(&[
        "function f(x) {",
        "  const s = 'it\\'s fine';",
        "  if (x) { return s; }",
        "  return null;",
        "}",
    ]);
    assert_eq!(one(&source), 2);
}

#[test]
fn counts_branch_inside_template_interpolation() {
    let source = lines(&[
        "function f(x) {",
        "  const s = `value: ${x ? 1 : 2}`;",
        "  return s;",
        "}",
    ]);
    assert_eq!(one(&source), 2);
}

#[test]
fn refuses_to_score_unterminated_literal() {
    for source in [
        lines(&["function f() {", "  const s = \"never closed;", "}"]),
        lines(&["function f() {", "  const s = `never closed;", "}"]),
        lines(&["function f() {", "  /* never closed", "  return 1;", "}"]),
    ] {
        assert_eq!(analyze_source(&source), Err(Unparseable), "{source}");
    }
}

#[test]
fn produces_identical_scores_for_crlf_and_lf() {
    let lf = lines(&[
        "function f(x) {",
        "  if (x) {",
        "    return 1;",
        "  } else if (x === 2) {",
        "    return 2;",
        "  }",
        "  return 0;",
        "}",
        "",
        "function g(y) {",
        "  return y && y || y ? 1 : 0;",
        "}",
    ]);
    let crlf = lf.replace('\n', "\r\n");
    assert_eq!(analyze_source(&crlf), analyze_source(&lf));
    assert!(analyze_source(&lf).is_ok());
}

#[test]
fn returns_empty_result_for_empty_whitespace_and_functionless_files() {
    assert_eq!(analyze_source("").expect("ok"), Vec::new());
    assert_eq!(analyze_source("   \n\t\n   \n").expect("ok"), Vec::new());
    let top_level = lines(&["const x = 1;", "if (x) {", "  console.log(x);", "}"]);
    assert_eq!(analyze_source(&top_level).expect("ok"), Vec::new());
}

#[test]
fn analyzes_large_file_within_bounds() {
    let chunk =
        |i: usize| format!("function f{i}(x) {{\n  if (x) {{ return 1; }}\n  return 0;\n}}");
    let source: String = (0..20_000).map(chunk).collect::<Vec<_>>().join("\n\n");
    assert!(source.len() > 1_000_000);
    let found = scores(&source);
    assert_eq!(found.len(), 20_000);
    assert!(found.iter().all(|s| *s == 2));
}

// -- properties (the fast-check runs, with a seeded generator) --------------------------------------------------

/// A xorshift generator: the same strings on every run.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A string of up to 300 characters: mostly the ones that matter to a scanner, some printable, some not.
    fn string(&mut self) -> String {
        const PIECES: &[&str] = &[
            "if (x) { return 1; }",
            "&&",
            "||",
            "?",
            ":",
            "//",
            "/*",
            "*/",
            "`",
            "${",
            "}",
            "{",
            "(",
            ")",
            "'",
            "\"",
            "\\",
            "/",
            "\n",
            " ",
            "case 1:",
            "function f() {",
            "=>",
            "[",
            "]",
            "é",
            "😀",
            "\t",
            ".",
            "a",
            "0",
        ];
        let len = self.next() % 60;
        (0..len)
            .map(|_| PIECES[(self.next() % PIECES.len() as u64) as usize])
            .collect()
    }
}

const BASE: &str = "function f(x) {\n  if (x) { return 1; }\n  return 0;\n}\n";

#[test]
fn property_strip_never_manufactures_decision_points() {
    let base: Vec<u32> = scores(BASE);
    let mut seeded = Seeded(42);
    for _ in 0..300 {
        let source = seeded.string();
        let chars: Vec<char> = source.chars().collect();
        if let Ok(stripped) = strip::strip_literals(&chars) {
            assert!(stripped.len() <= chars.len(), "{source:?}");
        }
        if let Ok(functions) = analyze_source(&source) {
            assert!(functions.iter().all(|f| f.score >= 1), "{source:?}");
        }
        let comment_safe = source.replace('*', " ");
        let with_comment = analyze_source(&format!("{BASE}\n/* {comment_safe} */\n"));
        assert_eq!(
            with_comment
                .expect("a sanitized comment is parseable")
                .iter()
                .map(|f| f.score)
                .collect::<Vec<_>>(),
            base,
            "{source:?}"
        );
        let string_safe = source.replace(['"', '\\', '\r', '\n'], " ");
        let with_string = analyze_source(&format!("{BASE}\nconst __s = \"{string_safe}\";\n"));
        assert_eq!(
            with_string
                .expect("a sanitized string is parseable")
                .iter()
                .map(|f| f.score)
                .collect::<Vec<_>>(),
            base,
            "{source:?}"
        );
    }
}

#[test]
fn property_comments_and_strings_are_score_neutral() {
    let bases = [
        lines(&[
            "function f(x) {",
            "  if (x) { return 1; }",
            "  return 0;",
            "}",
        ]),
        lines(&["function g(x, y) {", "  return x && y;", "}"]),
        lines(&[
            "function h(x) {",
            "  for (let i = 0; i < x; i++) { g(i); }",
            "}",
        ]),
    ];
    let appenders: [fn(&str) -> String; 3] = [
        |s| format!("{s}\n// if (x) {{ return 1; }} && y || z ? 1 : 2\n"),
        |s| format!("{s}\n/* if (x) {{ return 1 }} && y */\n"),
        |s| format!("{s}\nconst __s = \"if (x) {{ return 1; }} && y || z ? 1 : 2\";\n"),
    ];
    for base in &bases {
        for append in appenders {
            assert_eq!(scores(base), scores(&append(base)), "{base}");
        }
    }
}

// -- file selection ---------------------------------------------------------------------------------------------

#[test]
fn file_selection() {
    for ext in ANALYZABLE_EXTENSIONS {
        assert!(is_analyzable_path(&format!("src/foo{ext}")), "{ext}");
    }
    for p in [
        "docs/readme.md",
        "package.json",
        "ci.yml",
        "notes.txt",
        "Makefile",
    ] {
        assert!(!is_analyzable_path(p), "{p}");
    }
    assert!(!is_analyzable_path("tests/foo.test.cjs"));
    assert!(!is_analyzable_path("tests\\foo.test.cjs"));
    assert!(!is_analyzable_path("gsd-core/bin/lib/foo.cjs"));
    assert!(!is_analyzable_path("gsd-core\\bin\\lib\\foo.cjs"));
}

// -- threshold / jump evaluation --------------------------------------------------------------------------------

fn file(name: &str, functions: &[(&str, usize, i64)]) -> Analyzed {
    Analyzed {
        file: name.to_string(),
        functions: Ok(functions
            .iter()
            .map(|(n, line, score)| FunctionScore {
                name: n.to_string(),
                start_line: *line,
                end_line: line + 2,
                score: u32::try_from(*score).expect("a score"),
            })
            .collect()),
    }
}

fn baseline(entries: &[(&str, i64)]) -> BTreeMap<String, i64> {
    entries.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

fn run(analyzed: &[Analyzed], base: &BTreeMap<String, i64>) -> Vec<Candidate> {
    evaluate(analyzed, base, THRESHOLD, JUMP_DELTA)
}

#[test]
fn threshold_is_strictly_greater() {
    let t = THRESHOLD;
    for score in [t - 1, t] {
        assert_eq!(
            run(&[file("a.js", &[("f", 1, score)])], &baseline(&[])),
            Vec::new(),
            "{score}"
        );
    }
    let found = run(&[file("a.js", &[("f", 1, t + 1)])], &baseline(&[]));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].reasons, ["threshold"]);
    assert_eq!((found[0].baseline, found[0].delta), (None, None));
}

#[test]
fn jump_is_strictly_greater() {
    let (t, d) = (THRESHOLD, JUMP_DELTA);
    let score = t - 5;
    let analyzed = [file("a.js", &[("f", 1, score)])];
    for before in [score - (d - 1), score - d] {
        assert_eq!(
            run(&analyzed, &baseline(&[("a.js::f", before)])),
            Vec::new(),
            "{before}"
        );
    }
    let found = run(&analyzed, &baseline(&[("a.js::f", score - (d + 1))]));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].reasons, ["jump"]);
}

#[test]
fn emits_a_single_candidate_carrying_both_reasons() {
    let score = THRESHOLD + 1;
    let found = run(
        &[file("a.js", &[("f", 1, score)])],
        &baseline(&[("a.js::f", score - (JUMP_DELTA + 1))]),
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].reasons, ["threshold", "jump"]);
}

#[test]
fn does_not_trigger_when_complexity_decreased() {
    assert_eq!(
        run(
            &[file("a.js", &[("f", 1, 10)])],
            &baseline(&[("a.js::f", 20)])
        ),
        Vec::new()
    );
}

#[test]
fn does_not_treat_missing_baseline_as_zero() {
    let found = run(&[file("a.js", &[("f", 1, THRESHOLD + 1)])], &baseline(&[]));
    assert_eq!(found[0].baseline, None);
    assert_eq!(
        found[0].delta, None,
        "no baseline: the delta is not the score"
    );
    assert_eq!(
        found[0].reasons,
        ["threshold"],
        "no jump without a baseline"
    );
}

#[test]
fn orders_candidates_deterministically() {
    let analyzed = [
        file("b.js", &[("y", 1, 20)]),
        file("a.js", &[("x", 1, 20), ("z", 10, 18)]),
    ];
    let found = evaluate(&analyzed, &baseline(&[("a.js::x", 10)]), 15, 100);
    let ids: Vec<String> = found
        .iter()
        .map(|c| format!("{}::{}", c.file, c.name))
        .collect();
    assert_eq!(
        ids,
        ["a.js::x", "b.js::y", "a.js::z"],
        "score desc, delta desc (none last), file asc, name asc"
    );
}

#[test]
fn breaks_candidate_ties_stably() {
    let analyzed = [file("a.js", &[("bravo", 20, 20), ("alpha", 5, 20)])];
    let found = evaluate(&analyzed, &baseline(&[]), 15, 100);
    assert_eq!(
        found.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        ["alpha", "bravo"]
    );
}

#[test]
fn a_non_positive_threshold_falls_back_to_the_default() {
    let analyzed = [file("a.js", &[("f", 1, THRESHOLD + 1)])];
    for bad in [0, -5] {
        assert_eq!(
            evaluate(&analyzed, &baseline(&[]), bad, bad).len(),
            1,
            "{bad}"
        );
    }
}

#[test]
fn an_unparseable_file_is_skipped() {
    let skipped = Analyzed {
        file: "a.js".to_string(),
        functions: Err(Unparseable),
    };
    assert_eq!(run(&[skipped], &baseline(&[])), Vec::new());
}
