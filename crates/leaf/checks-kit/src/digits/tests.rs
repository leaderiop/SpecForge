use super::*;
use crate::tables::iban::{LENGTHS, SOURCE};

fn reason(result: Result<(), Invalid>) -> String {
    result.expect_err("refused").reason
}

#[test]
fn luhn_reads_the_published_vector_and_names_the_expected_digit() {
    assert_eq!(luhn("79927398713"), Ok(()));
    assert_eq!(luhn("7992 7398-713"), Ok(()));
    let refused = luhn("79927398710").unwrap_err();
    assert_eq!(refused.reason, "the check digit is 0, expected 3");
    assert_eq!(refused.at, Some(10..11));
}

#[test]
fn a_letter_is_refused_on_its_own_bytes() {
    assert_eq!(luhn("79a").unwrap_err().at, Some(2..3));
    // A multi-byte character keeps the range on a character boundary.
    assert_eq!(luhn("7é9").unwrap_err().at, Some(1..3));
    assert_eq!(reason(luhn("")), "there are no digits");
    assert_eq!(reason(luhn(" - ")), "there are no digits");
}

#[test]
fn mod11_2_reads_an_orcid() {
    assert_eq!(iso7064::mod11_2("0000-0002-1825-0097"), Ok(()));
    assert_eq!(iso7064::mod11_2("0000-0002-1694-233X"), Ok(()));
    let refused = iso7064::mod11_2("0000-0002-1825-0098").unwrap_err();
    assert_eq!(refused.reason, "the check digit is 8, expected 7");
    assert_eq!(refused.at, Some(18..19));
    assert!(iso7064::mod11_2("00X0-0002-1825-0097").is_err());
}

#[test]
fn mod97_10_reads_numbers_and_letters_and_names_both_digits() {
    // "1234" followed by its check digits: 98 - (123400 mod 97) = 82.
    assert_eq!(iso7064::mod97_10("123482"), Ok(()));
    let refused = iso7064::mod97_10("123400").unwrap_err();
    assert_eq!(refused.reason, "the check digit is 00, expected 82");
    assert_eq!(refused.at, Some(4..6));
    // An ISO 17442 LEI reads letters too.
    assert_eq!(iso7064::mod97_10("5493001KJTIIGC8Y1R12"), Ok(()));
}

#[test]
fn mod97_10_holds_a_value_too_long_for_any_integer() {
    let long = format!("{}82", "1234".repeat(100_000));
    // The remainder is kept under 97 as it goes, so no length overflows it.
    assert!(reason(iso7064::mod97_10(&long)).starts_with("the check digit is 82, expected"));
}

#[test]
fn gtin_reads_8_and_13_digits() {
    assert_eq!(gtin("4006381333931"), Ok(()));
    assert_eq!(gtin("96385074"), Ok(()));
    assert_eq!(
        reason(gtin("4006381333932")),
        "the check digit is 2, expected 1"
    );
    // A check digit that holds on a length no GTIN has.
    assert_eq!(gs1::mod10("17"), Ok(()));
    assert_eq!(
        reason(gtin("17")),
        "a GTIN has 8, 12, 13 or 14 digits, this one has 2"
    );
}

#[test]
fn isbn_reads_10_and_13() {
    assert_eq!(isbn("0306406152"), Ok(()));
    assert_eq!(isbn("0-306-40615-2"), Ok(()));
    assert_eq!(isbn("9780306406157"), Ok(()));
    assert_eq!(isbn("978-0-306-40615-7"), Ok(()));
    assert_eq!(isbn("080442957X"), Ok(()));
    assert_eq!(
        reason(isbn("0306406153")),
        "the check digit is 3, expected 2"
    );
    assert_eq!(
        reason(isbn("9780306406158")),
        "the check digit is 8, expected 7"
    );
    assert_eq!(
        reason(isbn("1234567890123")),
        "an ISBN-13 starts with 978 or 979"
    );
    assert_eq!(
        reason(isbn("12345")),
        "an ISBN has 10 or 13 digits, this one has 5"
    );
    assert_eq!(isbn("03X6406152").unwrap_err().at, Some(2..3));
}

#[test]
fn iban_reads_the_registered_length_and_mod_97() {
    assert_eq!(iban("GB82 WEST 1234 5698 7654 32"), Ok(()));
    assert_eq!(iban("GB82WEST12345698765432"), Ok(()));
    let refused = iban("GB83 WEST 1234 5698 7654 32").unwrap_err();
    assert_eq!(refused.reason, "the check digits are 83, expected 82");
    assert_eq!(refused.at, Some(2..4));
    assert_eq!(
        reason(iban("GB82 WEST 1234 5698 7654")),
        "an IBAN of GB has 22 characters, this one has 20"
    );
    assert_eq!(iban("ZZ82 WEST").unwrap_err().at, Some(0..2));
    assert_eq!(iban("gb82west12345698765432").unwrap_err().at, Some(0..1));
    assert!(iban("GB").is_err());
}

#[test]
fn iban_table_source_is_pinned() {
    assert_eq!(
        SOURCE.standard,
        "ISO 13616 IBAN registry (SWIFT, its registration authority)"
    );
    assert_eq!(SOURCE.retrieved, "2026-11-04");
    assert_eq!(LENGTHS.len(), 105);
    assert_eq!(
        crate::tables::fingerprint(LENGTHS.iter().map(|r| format!("{r:?}"))),
        5390832029336228839,
        "the table changed: update its SOURCE and this pin"
    );
    assert!(
        LENGTHS.windows(2).all(|pair| pair[0].0 < pair[1].0),
        "sorted"
    );
    assert_eq!(registry::length_of("GB"), Some(22));
    assert_eq!(registry::length_of("NO"), Some(15));
    assert_eq!(registry::length_of("ZZ"), None);
}
