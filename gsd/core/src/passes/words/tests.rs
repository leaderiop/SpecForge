use super::*;

/// The four cases of the matcher before it had two boundaries: JavaScript's reading keeps today's answers for
/// `gsd/E312`, and the Unicode reading answers the first two the other way ("caf" is no word of "café", "élève" is a
/// word of "l'Élève"). Neither stems: "user" is no word of "Users".
#[test]
fn the_four_cases_read_as_javascript_and_as_unicode() {
    let cases = [
        ("le café est servi", "caf"),
        ("Gérer l'Élève", "élève"),
        ("the user-id", "user"),
        ("Users sign in", "user"),
    ];
    let read = |boundary| -> Vec<bool> {
        cases
            .iter()
            .map(|(haystack, needle)| has_word(haystack, needle, boundary))
            .collect()
    };
    assert_eq!(read(Boundary::Js), [true, false, true, false]);
    assert_eq!(read(Boundary::Unicode), [false, true, true, false]);
}

#[test]
fn a_phrase_matches_whole_across_line_breaks_and_case() {
    assert!(has_word(
        "the user account page",
        "User\n  Account",
        Boundary::Unicode
    ));
    assert!(has_word(
        "the User\n  account page",
        "user account",
        Boundary::Unicode
    ));
    assert!(!has_word(
        "the user accounts page",
        "user account",
        Boundary::Unicode
    ));
}

#[test]
fn a_needle_edge_that_is_no_word_character_needs_no_boundary() {
    assert!(has_word("we write C++.", "C++", Boundary::Unicode));
    assert!(has_word("C++", "C", Boundary::Unicode));
    // The needle's first character is a word character: "C" inside "XC++" is no word.
    assert!(!has_word("XC++", "C++", Boundary::Unicode));
}

#[test]
fn a_combining_mark_stays_in_its_word() {
    assert!(!has_word("le cafe\u{301}", "cafe", Boundary::Unicode));
    assert!(has_word("le cafe\u{301}", "cafe\u{301}", Boundary::Unicode));
    // No normalisation: a composed "é" is not its decomposed spelling.
    assert!(!has_word("le café", "cafe\u{301}", Boundary::Unicode));
}

#[test]
fn an_overlapping_occurrence_counts() {
    assert!(has_word("xa a a", "a a", Boundary::Unicode));
}

#[test]
fn an_empty_needle_matches_nothing() {
    for boundary in [Boundary::Js, Boundary::Unicode] {
        for haystack in ["", " ", "a task"] {
            assert!(
                !has_word(haystack, "", boundary),
                "{boundary:?} {haystack:?}"
            );
        }
    }
    assert!(!has_word("a task", " \n ", Boundary::Unicode));
}

#[test]
fn the_glossary_folds_whitespace_runs_and_lowercases() {
    assert_eq!(folded("  To-Do\n  List "), "to-do list");
    assert_eq!(folded("ÉLÈVE"), "élève");
    assert_eq!(folded("STRASSE"), "strasse");
}
