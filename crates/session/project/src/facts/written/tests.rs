//! How places written in one text nest, and where every other place lands in the text they leave.

use super::*;

fn write(line: usize, from: usize, held: &str, now: &str) -> Write {
    Write {
        span: (line, from, line, from + held.len()),
        held: held.into(),
        now: now.into(),
    }
}

/// `text` with the outermost places of `writes` written: what a rename writes.
fn applied(text: &str, writes: &Writes) -> String {
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    for w in writes.outer().iter().rev() {
        let line = &mut lines[w.span.0 - 1];
        line.replace_range(w.span.1 - 1..w.span.3 - 1, &w.now);
    }
    lines.join("\n")
}

/// The text `span` covers in `text`.
fn read(text: &str, span: Span) -> &str {
    let line = text.split('\n').nth(span.0 - 1).unwrap();
    &line[span.1 - 1..span.3 - 1]
}

#[test]
fn a_place_nested_in_another_reads_its_name_inside_what_the_outer_writes() {
    // `v1/T1` read whole by one fact, its `T1` by another: renaming `v1/T1` writes `v1/Two` and `Two`.
    let text = "// covers v1/T1 end";
    let (outer, inner) = (write(1, 11, "v1/T1", "v1/Two"), write(1, 14, "T1", "Two"));

    let writes = Writes::of([inner.clone(), outer.clone()]).expect("they nest");

    assert_eq!(writes.outer(), std::slice::from_ref(&outer));
    let left = applied(text, &writes);
    assert_eq!(left, "// covers v1/Two end");
    assert_eq!(read(&left, writes.written(&outer.span).unwrap()), "v1/Two");
    assert_eq!(read(&left, writes.written(&inner.span).unwrap()), "Two");
    // Given in the other order, the same.
    let again = Writes::of([outer, inner.clone()]).unwrap();
    assert_eq!(again.written(&inner.span), writes.written(&inner.span));
}

#[test]
fn a_nested_place_is_found_by_the_text_after_it_or_else_before_it() {
    // The parent renamed: `w/m` writes `x/m`; the `m` nested at its end writes itself, found by what follows it.
    let text = "// covers w/m and w/m/k";
    let parent = [write(1, 11, "w/m", "x/m"), write(1, 13, "m", "m")];
    let writes = Writes::of(parent.clone()).expect("they nest");
    let left = applied(text, &writes);
    assert_eq!(read(&left, writes.written(&parent[1].span).unwrap()), "m");

    // A child moved with its parent: `w/m/k` writes `w/n/k`, and the nested `w/m` writes `w/n`, found by what precedes
    // it, the text after it (`/k`) kept too.
    let child = [write(1, 19, "w/m/k", "w/n/k"), write(1, 19, "w/m", "w/n")];
    let writes = Writes::of(child.clone()).expect("they nest");
    let left = applied(text, &writes);
    assert_eq!(left, "// covers w/m and w/n/k");
    assert_eq!(read(&left, writes.written(&child[1].span).unwrap()), "w/n");
}

#[test]
fn places_that_cross_or_disagree_leave_no_one_text() {
    // Crossing: `a/T1` and `T1/x` over `a/T1/x`.
    assert!(Writes::of([write(1, 1, "a/T1", "a/T2"), write(1, 3, "T1/x", "T2/x")]).is_none());
    // One span written two ways.
    assert!(Writes::of([write(1, 1, "T1", "T2"), write(1, 1, "T1", "T3")]).is_none());
    // Nested, the inner writing what the outer does not.
    assert!(Writes::of([write(1, 1, "v1/T1", "v1/T2"), write(1, 4, "T1", "T9")]).is_none());
    // Nested, the inner reading what the outer does not.
    assert!(Writes::of([write(1, 1, "v1/T1", "v1/T2"), write(1, 4, "T7", "T2")]).is_none());
    // The same write twice is one.
    let twice = Writes::of([write(1, 1, "T1", "T2"), write(1, 1, "T1", "T2")]).unwrap();
    assert_eq!(twice.outer().len(), 1);
}

#[test]
fn a_place_nothing_writes_lands_where_the_writes_leave_its_text() {
    let text = "// covers a/T1 x\n// next line";
    let writes = Writes::of([write(1, 11, "a/T1", "a/Longer")]).unwrap();
    let left = applied(text, &writes);

    // After it, before it, and containing it (a fact's own place).
    let after = writes.place((1, 16, 1, 17), true).unwrap();
    assert_eq!(read(&left, after), "x");
    assert_eq!(read(&left, writes.place((1, 1, 1, 3), true).unwrap()), "//");
    assert_eq!(
        read(&left, writes.place((1, 1, 1, 17), false).unwrap()),
        "// covers a/Longer x"
    );
    assert_eq!(writes.place((2, 1, 2, 3), true), Some((2, 1, 2, 3)));
    // Inside it, in the part the name written keeps: `a` names something the rename did not move.
    assert_eq!(
        read(&left, writes.place((1, 11, 1, 12), true).unwrap()),
        "a"
    );
    // A name the write changes: inside it, or holding it.
    assert_eq!(
        writes.place((1, 13, 1, 15), true),
        None,
        "`T1` is `Longer` now"
    );
    assert_eq!(
        writes.place((1, 11, 1, 15), true),
        None,
        "`a/T1` is `a/Longer` now"
    );
    assert_eq!(writes.place((1, 10, 1, 17), true), None);
}
