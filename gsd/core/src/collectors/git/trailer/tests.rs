//! The shape [`trailer`] reads: one `<scope>/<task>` value, every segment of the same characters (ADR 0380).

use super::*;

fn values(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| v.to_string()).collect()
}

#[test]
fn a_trailers_values_are_read() {
    assert_eq!(trailer(&values(&["01-01/T1"])), Trailer::Named("01-01/T1"));
    assert_eq!(trailer(&[]), Trailer::None);
    // Repeats and blanks count once.
    assert_eq!(
        trailer(&values(&[" 01-01/T1 ", "", "01-01/T1"])),
        Trailer::Named("01-01/T1")
    );
}

#[test]
fn a_value_with_a_tab_a_unit_or_record_separator_a_space_or_no_slash_is_unreadable() {
    for value in [
        "01-01/\tT1",
        "01-01/T1\u{1f}x",
        "01-01/T1\u{0}",
        "01 01/T1",
        "01-01",
        "/T1",
        "01-01/",
        "01-01/T\u{7f}",
        "1\t2\tforged.js",
        // The full path keeps every segment to the same characters: no empty segment, no rooted key, no `:`.
        "v1.0//01-01/T1",
        "/v1.0/01/01-01/T1",
        "v1.0/01/01-01/",
        "v1.0/01 /01-01/T1",
        "v1.0/\t01/01-01/T1",
        "v1.0/01/01-01/T1\u{1f}x",
        "v1.0\u{1e}/01/01-01/T1",
        "v1.0:x/01/01-01/T1",
        "v1.0/01/01-01/T\u{7f}",
        "v1.0/01/01-01/T1/",
        "//T1",
    ] {
        assert_eq!(trailer(&values(&[value])), Trailer::Unreadable, "{value:?}");
    }
    // Two different values stay unreadable, whatever their form.
    assert_eq!(
        trailer(&values(&["v1.0/01/01-01/T1", "01-01/T1"])),
        Trailer::Unreadable
    );
}

#[test]
fn a_value_naming_its_task_by_full_path_is_read() {
    // The task part after the last `/`, the scope before it: the plan as a subject's scope writes it, or the full id.
    // Any depth is a value; the graph decides whether it names a task.
    for value in [
        "01-01/T1",
        "v1.0/01/01-01/T1",
        "a/b/c/d/e/T1",
        "01-01/T1/T2",
    ] {
        assert_eq!(
            trailer(&values(&[value])),
            Trailer::Named(value),
            "{value:?}"
        );
    }
    assert_eq!(
        trailer(&values(&[" v1.0/01/01-01/T1 ", "v1.0/01/01-01/T1"])),
        Trailer::Named("v1.0/01/01-01/T1")
    );
}

#[test]
fn two_different_values_are_unreadable() {
    assert_eq!(
        trailer(&values(&["01-01/T1", "01-01/T2"])),
        Trailer::Unreadable
    );
}
