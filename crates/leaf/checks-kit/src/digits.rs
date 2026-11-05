//! Check-digit algorithms. Each reads a value as it is printed, separators included; each refusal says which digit it
//! expected and points at the digit it found.

use std::ops::Range;

use crate::Invalid;
use crate::tables::iban as registry;

/// Each byte of `text` that is not one of `separators`, with where it is. A byte outside `allowed` is refused here, so
/// the arithmetic reads ASCII only.
fn written(
    text: &str,
    separators: &[u8],
    allowed: impl Fn(u8) -> bool,
) -> Result<Vec<(u8, usize)>, Invalid> {
    let mut kept = Vec::with_capacity(text.len());
    for (at, ch) in text.char_indices() {
        let byte = u8::try_from(u32::from(ch)).unwrap_or(0);
        if ch.is_ascii() && separators.contains(&byte) {
            continue;
        }
        if !ch.is_ascii() || !allowed(byte) {
            return Err(Invalid::at(
                format!("'{ch}' is not allowed here"),
                at..at + ch.len_utf8(),
            ));
        }
        kept.push((byte, at));
    }
    if kept.is_empty() {
        return Err(Invalid::new("there are no digits"));
    }
    Ok(kept)
}

fn digit_of(byte: u8) -> u32 {
    u32::from(byte - b'0')
}

/// The check digit(s) at the end of `kept`, as bytes of the text.
fn check_span(kept: &[(u8, usize)], count: usize) -> Range<usize> {
    let first = kept[kept.len() - count].1;
    let last = kept[kept.len() - 1].1;
    first..last + 1
}

fn wrong_check(expected: &str, kept: &[(u8, usize)], count: usize) -> Invalid {
    let found: String = kept[kept.len() - count..]
        .iter()
        .map(|(byte, _)| char::from(*byte))
        .collect();
    Invalid::at(
        format!("the check digit is {found}, expected {expected}"),
        check_span(kept, count),
    )
}

/// The Luhn check (ISO/IEC 7812-1): digits, with spaces or hyphens between them, the last being the check digit.
pub fn luhn(text: &str) -> Result<(), Invalid> {
    let kept = written(text, b" -", |byte| byte.is_ascii_digit())?;
    if kept.len() < 2 {
        return Err(Invalid::new("a Luhn number has at least two digits"));
    }
    let (check, payload) = kept.split_last().expect("at least two");
    let mut sum = 0;
    for (place, (byte, _)) in payload.iter().rev().enumerate() {
        let digit = digit_of(*byte);
        sum += match place % 2 {
            0 if digit > 4 => digit * 2 - 9,
            0 => digit * 2,
            _ => digit,
        };
        sum %= 10;
    }
    let expected = (10 - sum) % 10;
    if digit_of(check.0) == expected {
        Ok(())
    } else {
        Err(wrong_check(&expected.to_string(), &kept, 1))
    }
}

/// ISO/IEC 7064's pure systems.
pub mod iso7064 {
    use super::{Invalid, digit_of, written, wrong_check};

    /// MOD 11-2, the check behind ORCID and ISNI: digits, with spaces or hyphens between them, then a digit or `X`.
    pub fn mod11_2(text: &str) -> Result<(), Invalid> {
        let last = text
            .char_indices()
            .rfind(|(_, ch)| *ch != ' ' && *ch != '-')
            .map(|(at, _)| at);
        let kept = written(text, b" -", |byte| byte.is_ascii_digit() || byte == b'X')?;
        if let Some((byte, at)) = kept
            .iter()
            .find(|(byte, at)| *byte == b'X' && Some(*at) != last)
        {
            return Err(Invalid::at(
                format!("'{}' is only a check digit", char::from(*byte)),
                *at..*at + 1,
            ));
        }
        if kept.len() < 2 {
            return Err(Invalid::new("a MOD 11-2 number has at least two digits"));
        }
        let (check, payload) = kept.split_last().expect("at least two");
        let mut total = 0;
        for (byte, _) in payload {
            total = (total + digit_of(*byte)) * 2 % 11;
        }
        let expected = match (12 - total % 11) % 11 {
            10 => 'X',
            digit => char::from_digit(digit, 10).expect("under ten"),
        };
        if char::from(check.0) == expected {
            Ok(())
        } else {
            Err(wrong_check(&expected.to_string(), &kept, 1))
        }
    }

    /// MOD 97-10: digits and letters (a letter counts as 10 to 35, as IBAN and LEI read it), with spaces or hyphens
    /// between them, the last two being the check digits.
    pub fn mod97_10(text: &str) -> Result<(), Invalid> {
        let kept = written(text, b" -", |byte| byte.is_ascii_alphanumeric())?;
        if kept.len() < 3 {
            return Err(Invalid::new("a MOD 97-10 number has at least three digits"));
        }
        let bytes: Vec<u8> = kept.iter().map(|(byte, _)| *byte).collect();
        let (payload, check) = bytes.split_at(bytes.len() - 2);
        if check.iter().all(u8::is_ascii_digit) && super::remainder_97(&bytes) == 1 {
            Ok(())
        } else {
            Err(wrong_check(
                &format!("{:02}", super::check_digits_97(payload)),
                &kept,
                2,
            ))
        }
    }
}

/// The remainder of `bytes` by 97, a letter counting as its two-digit value.
fn remainder_97(bytes: &[u8]) -> u32 {
    let mut remainder = 0;
    for byte in bytes {
        remainder = match byte.to_ascii_uppercase() {
            letter @ b'A'..=b'Z' => (remainder * 100 + u32::from(letter - b'A') + 10) % 97,
            digit => (remainder * 10 + digit_of(digit)) % 97,
        };
    }
    remainder
}

/// The two check digits that follow `payload`.
fn check_digits_97(payload: &[u8]) -> u32 {
    98 - remainder_97(payload) * 100 % 97
}

/// GS1's mod-10 check digit.
pub mod gs1 {
    use super::{Invalid, digit_of, written, wrong_check};

    /// Digits, with spaces or hyphens between them, the last being the check digit.
    pub fn mod10(text: &str) -> Result<(), Invalid> {
        let kept = written(text, b" -", |byte| byte.is_ascii_digit())?;
        if kept.len() < 2 {
            return Err(Invalid::new("a GS1 number has at least two digits"));
        }
        let (check, payload) = kept.split_last().expect("at least two");
        let expected = check_digit(payload.iter().map(|(byte, _)| *byte));
        if digit_of(check.0) == expected {
            Ok(())
        } else {
            Err(wrong_check(&expected.to_string(), &kept, 1))
        }
    }

    /// The check digit of `payload`: from its right, weights 3 and 1 in turn.
    fn check_digit(payload: impl DoubleEndedIterator<Item = u8>) -> u32 {
        let mut sum = 0;
        for (place, byte) in payload.rev().enumerate() {
            sum = (sum + digit_of(byte) * if place % 2 == 0 { 3 } else { 1 }) % 10;
        }
        (10 - sum) % 10
    }
}

fn count_digits(text: &str) -> usize {
    text.bytes().filter(u8::is_ascii_digit).count()
}

/// A GTIN of 8, 12, 13 or 14 digits.
pub fn gtin(text: &str) -> Result<(), Invalid> {
    gs1::mod10(text)?;
    match count_digits(text) {
        8 | 12 | 13 | 14 => Ok(()),
        other => Err(Invalid::new(format!(
            "a GTIN has 8, 12, 13 or 14 digits, this one has {other}"
        ))),
    }
}

/// An ISBN-10 (weighted mod 11) or an ISBN-13 (978 or 979, then GS1's mod 10), with hyphens or spaces.
pub fn isbn(text: &str) -> Result<(), Invalid> {
    let kept = written(text, b" -", |byte| byte.is_ascii_digit() || byte == b'X')?;
    let x_at = kept.iter().position(|(byte, _)| *byte == b'X');
    match (kept.len(), x_at) {
        (10, None | Some(9)) => isbn10(&kept),
        (13, None) => {
            let bytes: Vec<u8> = kept.iter().map(|(byte, _)| *byte).collect();
            if !bytes.starts_with(b"978") && !bytes.starts_with(b"979") {
                return Err(Invalid::new("an ISBN-13 starts with 978 or 979"));
            }
            gs1::mod10(text)
        }
        (_, Some(at)) if at + 1 != kept.len() => Err(Invalid::at(
            "'X' is only an ISBN-10's check digit",
            kept[at].1..kept[at].1 + 1,
        )),
        (other, _) => Err(Invalid::new(format!(
            "an ISBN has 10 or 13 digits, this one has {other}"
        ))),
    }
}

fn isbn10(kept: &[(u8, usize)]) -> Result<(), Invalid> {
    let (check, payload) = kept.split_last().expect("ten");
    let mut sum = 0;
    for (place, (byte, _)) in payload.iter().enumerate() {
        sum += digit_of(*byte) * (10 - place as u32);
    }
    let expected = match (11 - sum % 11) % 11 {
        10 => 'X',
        digit => char::from_digit(digit, 10).expect("under ten"),
    };
    if char::from(check.0) == expected {
        Ok(())
    } else {
        Err(wrong_check(&expected.to_string(), kept, 1))
    }
}

/// An IBAN: its country's registered length, then MOD 97-10 over the number moved to read its check digits last. Spaces
/// are allowed between its characters, and its letters are upper case as the standard prints them.
pub fn iban(text: &str) -> Result<(), Invalid> {
    let kept = written(text, b" ", |byte| {
        byte.is_ascii_uppercase() || byte.is_ascii_digit()
    })?;
    if kept.len() < 4 {
        return Err(Invalid::new(
            "an IBAN starts with a country and check digits",
        ));
    }
    let bytes: Vec<u8> = kept.iter().map(|(byte, _)| *byte).collect();
    let country = std::str::from_utf8(&bytes[..2]).expect("ASCII");
    let span = kept[0].1..kept[1].1 + 1;
    if !bytes[..2].iter().all(u8::is_ascii_uppercase) {
        return Err(Invalid::at(
            "an IBAN starts with its country's two letters",
            span,
        ));
    }
    let Some(length) = registry::length_of(country) else {
        return Err(Invalid::at(format!("'{country}' has no IBAN"), span));
    };
    if bytes.len() != length {
        return Err(Invalid::new(format!(
            "an IBAN of {country} has {length} characters, this one has {}",
            bytes.len()
        )));
    }
    let digits = kept[2].1..kept[3].1 + 1;
    if !bytes[2..4].iter().all(u8::is_ascii_digit) {
        return Err(Invalid::at("an IBAN's check digits are digits", digits));
    }
    let mut moved = bytes[4..].to_vec();
    moved.extend_from_slice(&bytes[..4]);
    if remainder_97(&moved) == 1 {
        return Ok(());
    }
    let mut payload = bytes[4..].to_vec();
    payload.extend_from_slice(&bytes[..2]);
    Err(Invalid::at(
        format!(
            "the check digits are {}, expected {:02}",
            String::from_utf8_lossy(&bytes[2..4]),
            check_digits_97(&payload)
        ),
        digits,
    ))
}

#[cfg(test)]
mod tests;
