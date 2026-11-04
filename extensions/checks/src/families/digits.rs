//! The digits family: the check-digit algorithms, each from the text alone. A refusal names the digit it expected.

use specforge_checks_kit::digits::{self as kit, iso7064};
use specforge_extension_sdk::prelude::*;

use super::{no_args, well_formed, DIGITS, TEXT};

pub(super) fn contribute(c: &mut ContributionsBuilder) {
    well_formed(c, &DIGITS, "luhn", TEXT, no_args, |t, _| kit::luhn(t));
    well_formed(c, &DIGITS, "mod11", TEXT, no_args, |t, _| {
        iso7064::mod11_2(t)
    });
    well_formed(c, &DIGITS, "mod97", TEXT, no_args, |t, _| {
        iso7064::mod97_10(t)
    });
    well_formed(c, &DIGITS, "gtin", TEXT, no_args, |t, _| kit::gtin(t));
    well_formed(c, &DIGITS, "isbn", TEXT, no_args, |t, _| kit::isbn(t));
    well_formed(c, &DIGITS, "iban", TEXT, no_args, |t, _| kit::iban(t));
}
