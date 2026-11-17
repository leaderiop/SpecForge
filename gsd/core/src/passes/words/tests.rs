use super::*;

/// JavaScript's reading, for `gsd/E312`: ASCII word characters, ASCII letters folded. "caf" is a word of "café" (the
/// `é` is no word character), "élève" is no word of "l'Élève" (non-ASCII letters do not fold), and nothing stems:
/// "user" is no word of "Users".
#[test]
fn the_four_cases_read_as_javascript() {
    let cases = [
        ("le café est servi", "caf"),
        ("Gérer l'Élève", "élève"),
        ("the user-id", "user"),
        ("Users sign in", "user"),
    ];
    let read: Vec<bool> = cases
        .iter()
        .map(|(haystack, needle)| has_word(haystack, needle))
        .collect();
    assert_eq!(read, [true, false, true, false]);
}

#[test]
fn an_empty_needle_matches_nothing() {
    for haystack in ["", " ", "a task"] {
        assert!(!has_word(haystack, ""), "{haystack:?}");
    }
}

/// The exact-case reading: the byte offset of each whole-word occurrence, at the same JavaScript boundary. `-`, `(`,
/// `.` and `,` are no word characters; a letter, a digit or `_` beside the id is.
#[test]
fn word_starts_gives_each_whole_word_at_its_byte_offset() {
    let text = "D-01 and (D-01), XD-01 D-010 _D-01 D-01-x D-01.";
    assert_eq!(word_starts(text, "D-01"), [0, 10, 35, 42]);
    // A multi-byte character before the id moves its byte offset, not its reading.
    assert_eq!(word_starts("é D-01", "D-01"), [3]);
    // An occurrence that overlaps a rejected one is still read, as a regex reads it.
    assert_eq!(word_starts("xa-a-a", "a-a"), [3]);
}

#[test]
fn word_starts_reads_the_case_as_written() {
    assert!(word_starts("see d-01", "D-01").is_empty());
    assert!(word_starts("D-01", "").is_empty());
    // The folding reading still folds.
    assert!(has_word("see d-01", "D-01"));
    assert!(has_word("xa-a-A", "A-a"));
}

#[test]
fn the_fold_collapses_whitespace_runs_and_lowercases() {
    assert_eq!(folded("  To-Do\n  List "), "to-do list");
    assert_eq!(folded("ÉLÈVE"), "élève");
    assert_eq!(folded("STRASSE"), "strasse");
}
