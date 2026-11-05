//! The digits family: each algorithm's published vectors, and the digit a refusal names.

use serde_json::json;

use super::{clean, e103, e103_at, finding};

#[test]
fn digits_luhn() {
    clean("luhn", "79927398713", json!({}));
    e103("luhn", "79927398710", json!({}));
}

#[test]
fn digits_mod11() {
    clean("mod11", "0000-0002-1694-233X", json!({}));
    e103("mod11", "0000-0002-1694-2330", json!({}));
}

#[test]
fn digits_mod97() {
    clean("mod97", "123482", json!({}));
    e103("mod97", "123400", json!({}));
}

#[test]
fn digits_gtin() {
    clean("gtin", "4006381333931", json!({}));
    e103("gtin", "4006381333932", json!({}));
}

#[test]
fn digits_isbn() {
    clean("isbn", "0-306-40615-2", json!({}));
    clean("isbn", "9780306406157", json!({}));
    e103("isbn", "0306406153", json!({}));
}

#[test]
fn digits_iban() {
    clean("iban", "GB82 WEST 1234 5698 7654 32", json!({}));
    e103("iban", "GB82 WEST 1234 5698 7654 33", json!({}));
}

#[test]
fn a_digit_finding_names_the_expected_digit() {
    let found = finding("luhn", "79927398710", json!({})).expect("a wrong digit");
    assert!(found.message.contains("expected 3"), "{}", found.message);
    assert_eq!(e103_at("luhn", "79927398710", json!({})), [10, 11]);
}

#[test]
fn digits_carry_their_switch_and_need_no_scope() {
    let declaration = super::declaration();
    let digits: Vec<_> = declaration
        .checkers
        .iter()
        .filter(|checker| checker.when == ["families.digits"])
        .collect();
    assert_eq!(digits.len(), 6);
    assert!(digits.iter().all(|checker| checker.scopes.is_empty()));
}
