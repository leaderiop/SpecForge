use super::estimate_tokens;

#[test]
fn empty_text_costs_nothing() {
    assert_eq!(estimate_tokens(""), 0);
}

#[test]
fn a_word_run_is_one_token() {
    assert_eq!(estimate_tokens("hello world"), 2);
    assert_eq!(estimate_tokens("foo_bar baz"), 2);
    assert_eq!(estimate_tokens("é"), 1);
}

#[test]
fn json_punctuation_costs_half_rounded_up() {
    // two words, five structural characters: { " " : }
    assert_eq!(estimate_tokens(r#"{"a":1}"#), 2 + 5_usize.div_ceil(2));
    assert_eq!(estimate_tokens(r#"{"a":1}"#), 5);
}
