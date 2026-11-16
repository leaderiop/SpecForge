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

#[test]
fn the_fold_collapses_whitespace_runs_and_lowercases() {
    assert_eq!(folded("  To-Do\n  List "), "to-do list");
    assert_eq!(folded("ÉLÈVE"), "élève");
    assert_eq!(folded("STRASSE"), "strasse");
}
