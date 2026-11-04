//! Places, currencies and languages: RFC 5646, ISO 3166-1, ISO 4217, the tz database and ITU-T E.164. A list is read from
//! its pinned table; a language tag is read for its form alone, with no subtag registry.

use language_tags::LanguageTag;

use crate::Invalid;
use crate::tables::{
    e164 as calling, iana_tz as zones, iso3166 as countries, iso4217 as currencies,
};

/// Which of a code's forms a value may take.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CodeForm {
    Alpha2,
    Alpha3,
    Numeric,
    #[default]
    Any,
}

/// A well-formed BCP 47 tag (RFC 5646).
pub fn bcp47(text: &str) -> Result<(), Invalid> {
    LanguageTag::parse(text)
        .map(|_| ())
        .map_err(|error| Invalid::new(error.to_string()))
}

/// An ISO 3166-1 country code.
pub fn iso3166(text: &str, form: CodeForm) -> Result<(), Invalid> {
    let found = match form {
        CodeForm::Alpha2 => countries::is_alpha2(text),
        CodeForm::Alpha3 => countries::is_alpha3(text),
        CodeForm::Numeric => countries::is_numeric(text),
        CodeForm::Any => {
            countries::is_alpha2(text) || countries::is_alpha3(text) || countries::is_numeric(text)
        }
    };
    found
        .then_some(())
        .ok_or_else(|| Invalid::new("this is no ISO 3166-1 country code in that form"))
}

/// An ISO 4217 currency code in use: alpha-3 or numeric. A two-letter form does not exist, and finds nothing.
pub fn iso4217(text: &str, form: CodeForm) -> Result<(), Invalid> {
    let found = match form {
        CodeForm::Alpha3 => currencies::is_alpha(text),
        CodeForm::Numeric => currencies::is_numeric(text),
        CodeForm::Alpha2 => false,
        CodeForm::Any => currencies::is_alpha(text) || currencies::is_numeric(text),
    };
    found
        .then_some(())
        .ok_or_else(|| Invalid::new("this is no ISO 4217 currency code in use, in that form"))
}

/// A name of the tz database; when `canonical`, not a name that links to another.
pub fn iana_tz(text: &str, canonical: bool) -> Result<(), Invalid> {
    if zones::is_canonical(text) {
        return Ok(());
    }
    if zones::is_link(text) {
        return if canonical {
            Err(Invalid::new(
                "this name links to another; write the zone's own name",
            ))
        } else {
            Ok(())
        };
    }
    Err(Invalid::new("the tz database has no such name"))
}

/// An E.164 number's structure: `+`, an assigned country calling code, at most 15 digits. A country's own numbering
/// plan is jurisdictional, and left to packs.
pub fn e164(text: &str) -> Result<(), Invalid> {
    let Some(digits) = text.strip_prefix('+') else {
        return Err(Invalid::new("an E.164 number starts with `+`"));
    };
    if let Some(at) = digits.find(|ch: char| !ch.is_ascii_digit()) {
        let length = digits[at..].chars().next().map_or(1, char::len_utf8);
        return Err(Invalid::at("this is not a digit", at + 1..at + 1 + length));
    }
    if digits.len() > 15 {
        return Err(Invalid::new(format!(
            "an E.164 number has at most 15 digits, this one has {}",
            digits.len()
        )));
    }
    let Some(code) = calling::code_of(digits) else {
        return Err(Invalid::new("this country calling code is not assigned"));
    };
    if digits.len() == code.len() {
        return Err(Invalid::new(
            "there is no number after the country calling code",
        ));
    }
    Ok(())
}
