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

/// A mark or a joiner is part of the word it sits in, in any script: a Devanagari virama, a Persian zero-width
/// non-joiner and a zero-width joiner never open a word, and a Thai tone mark stays on its letter.
#[test]
fn a_mark_or_a_joiner_stays_in_its_word_in_every_script() {
    assert!(!has_word("क्या हाल", "या", Boundary::Unicode));
    assert!(has_word("क्या हाल", "क्या", Boundary::Unicode));
    assert!(!has_word("می\u{200c}خواهم", "می", Boundary::Unicode));
    assert!(!has_word("x\u{200d}y", "x", Boundary::Unicode));
    assert!(!has_word("ผู้ใช้", "ผู", Boundary::Unicode));
}

/// Han and Hiragana are a segment per character and a Katakana run is one word, so a word of them is found in running
/// text; Thai, segmented per character without a dictionary, finds a word inside a longer one.
#[test]
fn a_word_is_found_in_scripts_written_without_spaces() {
    assert!(has_word(
        "ユーザーがログインする",
        "ユーザー",
        Boundary::Unicode
    ));
    assert!(has_word(
        "ユーザーがログインする",
        "する",
        Boundary::Unicode
    ));
    assert!(!has_word(
        "ユーザーがログインする",
        "ユーザ",
        Boundary::Unicode
    ));
    assert!(has_word("用户登录系统", "用户", Boundary::Unicode));
    assert!(has_word("用户登录系统", "登录", Boundary::Unicode));
    assert!(has_word("ผู้ใช้งาน", "ผู้ใช้", Boundary::Unicode));
}

/// An apostrophe, straight or typographic, splits a word as French elision does; a full stop between two letters
/// joins them, as UAX #29 reads `e.g.` or a host name.
#[test]
fn an_apostrophe_splits_a_word_and_a_full_stop_between_letters_does_not() {
    assert!(has_word("Gérer l\u{2019}Élève", "élève", Boundary::Unicode));
    assert!(has_word("we can't", "can", Boundary::Unicode));
    assert!(!has_word("see example.com", "example", Boundary::Unicode));
    assert!(has_word("an example. Then", "example", Boundary::Unicode));
}
