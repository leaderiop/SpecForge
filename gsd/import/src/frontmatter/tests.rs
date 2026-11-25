use super::*;

fn front(text: &str) -> Result<Located<Value>, Refused> {
    let (front, _) = split(text);
    read(&front.expect("closed").expect("a frontmatter"))
}

fn s(line: usize, text: &str) -> Located<Value> {
    Located {
        line,
        value: Value::Str(text.to_string()),
    }
}

fn refused_at(text: &str) -> (usize, String) {
    let r = front(text).expect_err("refused");
    (r.line, r.reason)
}

#[test]
fn split_keeps_the_body_at_its_lines() {
    let text = "---\na: 1\n---\n# Title\n";
    let (front, start) = split(text);
    let front = front.expect("closed").expect("a frontmatter");
    assert_eq!((front.lines, front.line, start), (vec!["a: 1"], 2, 3));
    assert_eq!(body(text, start), "\n\n\n# Title\n");
    assert_eq!(split("# No front\n---\n"), (Ok(None), 0));
}

#[test]
fn an_unclosed_frontmatter_is_refused_at_line_one() {
    let (front, start) = split("---\na: 1\n# Title\n");
    let refusal = front.expect_err("refused");
    assert_eq!((refusal.line, start), (1, 0));
    assert!(refusal.reason.contains("not closed"), "{}", refusal.reason);
}

#[test]
fn scalars_are_never_typed_and_keep_their_lines() {
    let map = front(
        "---\nphase: \"01\"\nplan: 01\nauto: true\nempty:\nq: 'it''s'\nc: text # a note\n---\n",
    )
    .expect("read");
    assert_eq!(map.line, 2);
    let Value::Map(entries) = &map.value else {
        panic!("a map")
    };
    let got: Vec<(&str, &Located<Value>)> = entries.iter().map(|(k, v)| (k.as_str(), v)).collect();
    assert_eq!(
        got,
        [
            ("phase", &s(2, "01")),
            ("plan", &s(3, "01")),
            ("auto", &s(4, "true")),
            ("empty", &s(5, "")),
            ("q", &s(6, "it's")),
            ("c", &s(7, "text")),
        ]
    );
    let escaped = front("---\nv: \"a \\\"b\\\" \\\\ c\"\n---\n").expect("read");
    assert_eq!(escaped.value.get("v"), Some(&s(2, "a \"b\" \\ c")));
}

#[test]
fn flow_lists_and_maps_hold_scalars() {
    let map =
        front("---\nfiles: [package.json, \"a, b\", 'c']\nnone: []\nm: {a: 1, b: \"x y\"}\n---\n")
            .expect("read");
    assert_eq!(
        map.value.get("files").map(|v| &v.value),
        Some(&Value::List(vec![
            s(2, "package.json"),
            s(2, "a, b"),
            s(2, "c")
        ]))
    );
    assert_eq!(
        map.value.get("none").map(|v| &v.value),
        Some(&Value::List(vec![]))
    );
    assert_eq!(
        map.value.get("m").map(|v| &v.value),
        Some(&Value::Map(vec![
            ("a".into(), s(4, "1")),
            ("b".into(), s(4, "x y"))
        ]))
    );
}

#[test]
fn a_plans_must_haves_read_as_lists_of_maps() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../parity/fixtures/taskboard/.planning/phases/01-core/01-01-PLAN.md"
    ))
    .expect("the fixture");
    let map = front(&text).expect("read");
    assert_eq!(map.value.get("phase"), Some(&s(2, "01-core")));
    assert_eq!(
        map.value.get("files_modified").map(|v| &v.value),
        Some(&Value::List(vec![
            s(7, "package.json"),
            s(7, "src/tasks.js"),
            s(7, "test/tasks.test.js")
        ]))
    );
    let must = &map.value.get("must_haves").expect("must_haves").value;
    let truths = must.get("truths").expect("truths");
    assert_eq!(
        truths.value,
        Value::List(vec![s(
            12,
            "A task added to a board has the next integer id and keeps its title"
        )])
    );
    let artifact = match &must.get("artifacts").expect("artifacts").value {
        Value::List(items) => items[0].clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(artifact.line, 14);
    assert_eq!(artifact.value.get("path"), Some(&s(14, "src/tasks.js")));
    assert_eq!(
        artifact.value.get("contains"),
        Some(&s(16, "export function addTask"))
    );
    let link = match &must.get("key_links").expect("key_links").value {
        Value::List(items) => items[0].clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(link.value.get("via"), Some(&s(20, "import { addTask }")));
}

#[test]
fn the_fixtures_security_and_state_frontmatters_read() {
    let fixture = |path: &str| {
        std::fs::read_to_string(format!(
            "{}/../parity/fixtures/{path}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("the fixture")
    };
    let security = front(&fixture("vault/.planning/phases/01-core/01-SECURITY.md")).expect("read");
    assert_eq!(security.value.get("phase"), Some(&s(2, "01")));
    assert_eq!(security.value.get("created"), Some(&s(8, "2026-10-30")));
    let state = front(&fixture("taskboard/.planning/STATE.md")).expect("read");
    assert_eq!(state.value.get("current_phase"), Some(&s(3, "02")));
    let progress = &state.value.get("progress").expect("progress").value;
    assert_eq!(progress.get("percent"), Some(&s(16, "33")));
}

#[test]
fn nested_maps_and_compact_lists_read() {
    let map = front("---\nprogress:\n  total: 4\n  done: 1\nlist:\n- a\n- b\nafter: x\n---\n")
        .expect("read");
    assert_eq!(
        map.value.get("progress").map(|v| &v.value),
        Some(&Value::Map(vec![
            ("total".into(), s(3, "4")),
            ("done".into(), s(4, "1"))
        ]))
    );
    assert_eq!(
        map.value.get("list").map(|v| &v.value),
        Some(&Value::List(vec![s(6, "a"), s(7, "b")]))
    );
    assert_eq!(map.value.get("after"), Some(&s(8, "x")));
}

#[test]
fn block_scalars_fold_and_chomp() {
    let text = "---\nlit: |\n  one\n    two\n\nclip: |-\n  a\n  b\nkeep: |+\n  k\n\nfold: >\n  a\n  b\n\n  c\nend: x\n---\n";
    let map = front(text).expect("read");
    let get = |k: &str| {
        map.value
            .get(k)
            .and_then(|v| v.value.as_str())
            .map(str::to_string)
    };
    assert_eq!(get("lit").as_deref(), Some("one\n  two\n"));
    assert_eq!(get("clip").as_deref(), Some("a\nb"));
    assert_eq!(get("keep").as_deref(), Some("k\n\n"));
    assert_eq!(get("fold").as_deref(), Some("a b\nc\n"));
    assert_eq!(get("end").as_deref(), Some("x"));
}

#[test]
fn every_unread_yaml_feature_is_refused_at_its_line() {
    for (text, line, why) in [
        ("---\na: &x 1\n---\n", 2, "anchor"),
        ("---\na: 1\nb: *x\n---\n", 3, "alias"),
        ("---\na: !tag 1\n---\n", 2, "tag"),
        ("---\nb:\n  <<: {a: 1}\n---\n", 3, "merge"),
        ("---\na: 1\n...\nb: 2\n---\n", 3, "second YAML document"),
        ("---\na: 1\n--- b\n---\n", 3, "second YAML document"),
        ("---\na:\n\tb: 1\n---\n", 3, "tab"),
        ("---\na: 1\na: 2\n---\n", 3, "twice"),
        ("---\na: [1, [2]]\n---\n", 2, "inside a flow"),
        ("---\na: \"open\n---\n", 2, "past its line"),
    ] {
        let (at, reason) = refused_at(text);
        assert_eq!(at, line, "{text:?}: {reason}");
        assert!(reason.contains(why), "{text:?}: {reason}");
    }
}

#[test]
fn a_line_past_the_cap_is_refused() {
    let text = format!("---\na: {}\n---\n", "x".repeat(MAX_LINE));
    let (at, reason) = refused_at(&text);
    assert_eq!(at, 2);
    assert!(reason.contains("longer than"), "{reason}");
}

/// `a1:` holding `a2:` ... to `aN: x`: N maps nested.
fn nest(n: usize) -> String {
    let mut text = "---\n".to_string();
    for i in 1..n {
        text.push_str(&format!("{}a{i}:\n", " ".repeat(i - 1)));
    }
    text.push_str(&format!("{}a{n}: x\n---\n", " ".repeat(n - 1)));
    text
}

#[test]
fn nesting_stops_at_the_depth_cap() {
    assert_eq!(MAX_DEPTH, 8);
    assert!(front(&nest(MAX_DEPTH)).is_ok());
    let (at, reason) = refused_at(&nest(MAX_DEPTH + 1));
    // `a9` opens the ninth map, on the file's line 10.
    assert_eq!(at, MAX_DEPTH + 2, "{reason}");
    assert!(reason.contains("deeper than 8"), "{reason}");
    // A flow collection is one level more.
    let mut flow = nest(MAX_DEPTH);
    flow = flow.replace(&format!("a{MAX_DEPTH}: x"), &format!("a{MAX_DEPTH}: [x]"));
    assert!(refused_at(&flow).1.contains("deeper"));
}

#[test]
fn a_key_given_twice_is_found_in_linear_time() {
    // 200,000 keys: a scan of the earlier keys per key is 2e10 comparisons; a set is well under a second.
    let n = 200_000;
    let mut text = "---\n".to_string();
    for k in 0..n {
        text.push_str(&format!("k{k}: x\n"));
    }
    text.push_str("k7: again\n---\n");
    let started = std::time::Instant::now();
    let (at, reason) = refused_at(&text);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(
        (at, reason.as_str()),
        (n + 2, "the key `k7` is given twice")
    );
}

#[test]
fn an_empty_frontmatter_is_an_empty_map() {
    let (front, start) = split("---\n---\nbody\n");
    let map = read(&front.expect("closed").expect("a frontmatter")).expect("read");
    assert_eq!((map.value, start), (Value::Map(Vec::new()), 2));
}
