//! Test support (D-r44-2): the cases of a vendored upstream suite, accounted both ways. Every `test(…)`/`it(…)` of the
//! suite is either ported (a table entry or a replayed recording runs it) or skipped with a reason, and every entry of
//! either list names a case of the suite. A list may not grow to make the test pass.

use std::collections::BTreeSet;

/// Why a case is not ported. Each reason starts with one of these: D-r44-2's six, and `diverges:`, the one amendment
/// (REVIEW-r44 M5): a deliberate difference that `gsd/parity/ACCEPTANCE.md` lists under "Known divergences".
const REASONS: [&str; 7] = [
    "git:",
    "env:",
    "fs:",
    "runtime:",
    "installer:",
    "security:",
    "diverges:",
];

/// The "Known divergences" section of ACCEPTANCE.md, which every `diverges: [tag]` skip must name by its tag.
const DIVERGENCES: &str = include_str!("../../../parity/ACCEPTANCE.md");

/// A case of a suite: the `describe` titles around a `test`, and its title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    pub path: String,
    pub title: String,
}

fn literal(rest: &str) -> Option<String> {
    let mut chars = rest.trim_start().chars();
    let quote = chars.next().filter(|c| matches!(c, '\'' | '"' | '`'))?;
    let mut out = String::new();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.extend(chars.next()),
            c if c == quote => return Some(out),
            c => out.push(c),
        }
    }
    None
}

/// The cases of a Node test suite, by indentation: a `describe` encloses the cases indented deeper than it.
pub fn cases(src: &str) -> Vec<Case> {
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut out = Vec::new();
    let lines: Vec<&str> = src.lines().collect();
    for (n, line) in lines.iter().enumerate() {
        let indent = line.len() - line.trim_start().len();
        let t = line.trim_start();
        let opener = [
            "describe(",
            "describe.skip(",
            "suite(",
            "test(",
            "test.skip(",
            "test.todo(",
            "it(",
            "it.skip(",
        ]
        .into_iter()
        .find(|o| t.starts_with(o));
        let Some(opener) = opener else { continue };
        let mut rest = &t[opener.len()..];
        if rest.trim().is_empty() {
            rest = lines.get(n + 1).copied().unwrap_or_default();
        }
        // A title that is not a literal is still a case (REVIEW-r44 M5): it is named by its expression and must be
        // ported or skipped like any other, never dropped.
        let title = literal(rest).unwrap_or_else(|| {
            let expr = rest.split([',', ')']).next().unwrap_or_default().trim();
            format!("<non-literal title: {expr}>")
        });
        while stack.last().is_some_and(|(i, _)| *i >= indent) {
            stack.pop();
        }
        if opener.starts_with("describe") || opener.starts_with("suite") {
            stack.push((indent, title));
        } else {
            let mut path: Vec<&str> = stack.iter().map(|(_, d)| d.as_str()).collect();
            path.push(&title);
            out.push(Case {
                path: path.join(" > "),
                title,
            });
        }
    }
    out
}

/// The literal runs of `pattern`, a static title in which each `${…}` stands for any text: one more than its `${…}`s.
fn literals(pattern: &str) -> Vec<String> {
    let mut parts: Vec<String> = vec![String::new()];
    let mut rest = pattern;
    while let Some(at) = rest.find("${") {
        parts.last_mut().expect("a part").push_str(&rest[..at]);
        let mut depth = 0;
        let mut end = rest.len();
        for (i, c) in rest[at + 1..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = at + 1 + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        parts.push(String::new());
        rest = &rest[end..];
    }
    parts.last_mut().expect("a part").push_str(rest);
    parts
}

/// Whether `text` is an instance of `pattern`, a static title in which each `${…}` stands for any text.
pub fn instance_of(pattern: &str, text: &str) -> bool {
    let parts = literals(pattern);
    let (first, last) = (&parts[0], &parts[parts.len() - 1]);
    if parts.len() == 1 {
        return pattern == text;
    }
    if !text.starts_with(first.as_str())
        || !text.ends_with(last.as_str())
        || text.len() < first.len() + last.len()
    {
        return false;
    }
    let mut at = first.len();
    let end = text.len() - last.len();
    for mid in &parts[1..parts.len() - 1] {
        match text[at..end].find(mid.as_str()) {
            Some(i) => at += i + mid.len(),
            None => return false,
        }
    }
    true
}

/// How many characters of `pattern` are literal: the more, the more specific.
fn specificity(pattern: &str) -> usize {
    literals(pattern).iter().map(String::len).sum()
}

/// Panics unless `ported` and `skipped` account for every case of `suite`, both ways. An entry of `ported` is a case's
/// title or its full path; an entry of `skipped` is a full path or a prefix of one (a `describe` block), with a reason.
pub fn account(suite: &str, ported: &[String], skipped: &[(&str, &str)]) {
    let all = cases(suite);
    assert!(!all.is_empty(), "the suite holds no case");
    for (entry, reason) in skipped {
        assert!(
            REASONS.iter().any(|r| reason.starts_with(r)),
            "skip '{entry}': the reason '{reason}' is none of {REASONS:?}"
        );
    }
    let section = DIVERGENCES
        .split("## Known divergences")
        .nth(1)
        .and_then(|s| s.split("\n## ").next())
        .expect("ACCEPTANCE.md has a Known divergences section");
    for (entry, reason) in skipped {
        if let Some(rest) = reason.strip_prefix("diverges: [") {
            let tag = rest.split(']').next().unwrap_or_default();
            assert!(
                section.contains(&format!("[{tag}]")),
                "skip '{entry}': the divergence [{tag}] is not listed under Known divergences in ACCEPTANCE.md"
            );
        } else {
            assert!(
                !reason.starts_with("diverges:"),
                "skip '{entry}': a `diverges:` reason starts with its [tag]"
            );
        }
    }
    let mut problems = Vec::new();
    let ported: BTreeSet<&str> = ported.iter().map(String::as_str).collect();
    for c in &all {
        let by_port = ported.contains(c.path.as_str()) || ported.contains(c.title.as_str());
        let by_skip = skipped
            .iter()
            .filter(|(p, _)| c.path.starts_with(p))
            .count();
        match (by_port, by_skip) {
            (true, 0) | (false, 1) => {}
            (false, 0) => problems.push(format!("neither ported nor skipped: {}", c.path)),
            (true, _) => problems.push(format!("ported and skipped: {}", c.path)),
            (false, _) => problems.push(format!("skipped twice: {}", c.path)),
        }
    }
    for p in &ported {
        if !all.iter().any(|c| c.path == *p || c.title == *p) {
            problems.push(format!("ported but not in the suite: {p}"));
        }
    }
    for (p, _) in skipped {
        if !all.iter().any(|c| c.path.starts_with(p)) {
            problems.push(format!("skipped but not in the suite: {p}"));
        }
    }
    assert!(
        problems.is_empty(),
        "{} problems:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// The static case a recorded call belongs to: the one whose title pattern matches, the most specific on a tie.
pub fn case_of<'a>(all: &'a [Case], path: &str) -> &'a Case {
    let mut found: Vec<&Case> = all.iter().filter(|c| instance_of(&c.path, path)).collect();
    found.sort_by_key(|c| std::cmp::Reverse(specificity(&c.path)));
    match found.as_slice() {
        [] => panic!("no case of the suite is '{path}'"),
        [one] => one,
        [a, b, ..] if specificity(&a.path) > specificity(&b.path) => a,
        many => panic!("'{path}' is an instance of {} cases: {many:?}", many.len()),
    }
}

#[test]
fn the_case_with_more_literal_text_wins() {
    let all = cases(
        "test(`blocks ${JSON.stringify(cmd)}`, () => {});\ntest(`blocks ${w} bash -c`, () => {});\n",
    );
    assert_eq!(
        case_of(&all, "blocks env -i bash -c").path,
        "blocks ${w} bash -c"
    );
    assert_eq!(
        case_of(&all, "blocks \"cat .env\"").path,
        "blocks ${JSON.stringify(cmd)}"
    );
}

#[test]
fn a_template_title_matches_its_instances() {
    assert!(instance_of(
        "blocks ${JSON.stringify(cmd)}",
        "blocks \"git -c x\""
    ));
    assert!(instance_of("a ${x} b ${y} c", "a 1 b 2 c"));
    assert!(!instance_of("a ${x} b", "a 1 c"));
    assert!(!instance_of("plain", "plain!"));
}

#[test]
fn the_accounting_reads_describe_blocks_and_titles() {
    let src = "describe('a', () => {\n  test('one', () => {});\n  describe('b', () => {\n    test(`two ${x}`, () => {});\n  });\n});\ntest('three', () => {});\n";
    let all: Vec<String> = cases(src).into_iter().map(|c| c.path).collect();
    assert_eq!(all, ["a > one", "a > b > two ${x}", "three"]);
    account(
        src,
        &["one".into(), "a > b > two ${x}".into()],
        &[("three", "env: a test of the account")],
    );
}

#[test]
#[should_panic(expected = "neither ported nor skipped: a > two")]
fn a_case_in_neither_list_fails() {
    let src = "describe('a', () => {\n  test('one', () => {});\n  test('two', () => {});\n});\n";
    account(src, &["one".into()], &[]);
}

#[test]
#[should_panic(expected = "skipped but not in the suite: z")]
fn a_skip_naming_no_case_fails() {
    let src = "test('one', () => {});\n";
    account(src, &["one".into()], &[("z", "env: nothing")]);
}

#[test]
fn a_title_that_is_not_a_literal_is_a_case() {
    let src = "for (const [desc] of cases) {\n  test(desc, () => {});\n}\ntest('one', () => {});\n";
    let all: Vec<String> = cases(src).into_iter().map(|c| c.path).collect();
    assert_eq!(all, ["<non-literal title: desc>", "one"]);
}
