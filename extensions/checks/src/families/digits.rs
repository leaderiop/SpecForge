//! The digits family: the check-digit algorithms, each from the text alone. A refusal names the digit it expected.

use specforge_checks_kit::digits::{self as kit, iso7064};
use specforge_extension_sdk::prelude::*;

use super::{well_formed, TEXT};

const SWITCH: &str = "families.digits";

pub(super) fn contribute(c: &mut ContributionsBuilder) {
    let none = |_: &mut CheckerBuilder| {};
    well_formed(c, SWITCH, "luhn", TEXT, none, |t, _| kit::luhn(t));
    well_formed(c, SWITCH, "mod11", TEXT, none, |t, _| iso7064::mod11_2(t));
    well_formed(c, SWITCH, "mod97", TEXT, none, |t, _| iso7064::mod97_10(t));
    well_formed(c, SWITCH, "gtin", TEXT, none, |t, _| kit::gtin(t));
    well_formed(c, SWITCH, "isbn", TEXT, none, |t, _| kit::isbn(t));
    well_formed(c, SWITCH, "iban", TEXT, none, |t, _| kit::iban(t));
}
