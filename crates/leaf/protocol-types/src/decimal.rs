//! The value of a `number`: an exact decimal of at most 15 significant digits (ADR 0361).
//!
//! Fifteen is the precision a binary64 round-trips (C's `DBL_DIG`, RFC 7493 §2.2): every decimal within the cap maps
//! to a distinct `f64`, and printing that `f64` shortest gives the decimal back. So the `.spec` word, the graph's JSON
//! number, the field text a pass reads, the `f64` a guest or a JSON client reads and a declared bound all hold one
//! value. [`Decimal::parse`] is the one reader of a decimal word, and [`Decimal::to_json`] / [`Decimal::from_json`]
//! the one way between a decimal and a JSON number.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An exact decimal, `coefficient × 10^exponent`, normalized so that one value has one form: the coefficient has no
/// trailing zero digit, zero is `0e0` and there is no negative zero. Equality and order are by value (`1.5` = `1.50`
/// = `15e-1`); the written word is not part of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Decimal {
    coefficient: i64,
    exponent: i16,
}

/// Why a text or a JSON number is no [`Decimal`]; its `Display` is the remedy E061 and every refusal write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecimalError {
    /// Not `[-+]?D+(\.D+)?([eE][-+]?D+)?`: the text as given.
    NotADecimal(String),
    /// More than [`Decimal::MAX_DIGITS`] significant digits.
    TooManyDigits,
    /// Outside binary64's normal range (an adjusted exponent beyond ±307): the text as given.
    OutOfRange(String),
}

impl fmt::Display for DecimalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecimalError::NotADecimal(text) => write!(f, "'{text}' is no decimal number"),
            DecimalError::TooManyDigits => write!(
                f,
                "a number keeps at most {} significant digits",
                Decimal::MAX_DIGITS
            ),
            DecimalError::OutOfRange(text) => {
                write!(f, "'{text}' is outside the range a number holds")
            }
        }
    }
}

impl std::error::Error for DecimalError {}

/// The greatest adjusted exponent (`exponent + digits − 1`) a decimal holds, and its negation the least: within it a
/// value is a normal binary64 (`1e-307 ..= 9.99…e307`).
const MAX_ADJUSTED: i64 = 307;

impl Decimal {
    /// The most significant digits a decimal keeps.
    pub const MAX_DIGITS: u32 = 15;

    /// Zero.
    pub const ZERO: Decimal = Decimal {
        coefficient: 0,
        exponent: 0,
    };

    /// Reads a decimal word, `[-+]?D+(\.D+)?([eE][-+]?D+)?` with `D` an ASCII digit: `0.75`, `-0.25`, `007.5`, `1.50`,
    /// `1.5e3`, `2E-4`, `"+1.5"`. Leading and trailing zeros read as the value they write. Anything else is
    /// [`DecimalError::NotADecimal`]; more than 15 significant digits is [`DecimalError::TooManyDigits`]; a value
    /// outside binary64's normal range is [`DecimalError::OutOfRange`].
    pub fn parse(text: &str) -> Result<Decimal, DecimalError> {
        let not = || DecimalError::NotADecimal(text.to_string());
        let bytes = text.as_bytes();
        let mut at = 0;
        let negative = match bytes.first() {
            Some(b'-') => {
                at = 1;
                true
            }
            Some(b'+') => {
                at = 1;
                false
            }
            _ => false,
        };
        let digits_from = |at: usize| {
            bytes[at..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count()
        };
        let whole = digits_from(at);
        if whole == 0 {
            return Err(not());
        }
        let mut digits: Vec<u8> = bytes[at..at + whole].to_vec();
        at += whole;
        let mut exponent: i64 = 0;
        if bytes.get(at) == Some(&b'.') {
            let fraction = digits_from(at + 1);
            if fraction == 0 {
                return Err(not());
            }
            digits.extend_from_slice(&bytes[at + 1..at + 1 + fraction]);
            exponent -= fraction as i64;
            at += 1 + fraction;
        }
        if matches!(bytes.get(at), Some(b'e' | b'E')) {
            at += 1;
            let sign = match bytes.get(at) {
                Some(b'-') => {
                    at += 1;
                    -1
                }
                Some(b'+') => {
                    at += 1;
                    1
                }
                _ => 1,
            };
            let written = digits_from(at);
            if written == 0 {
                return Err(not());
            }
            // Saturates: an exponent this large is out of range for any coefficient but zero.
            let value = bytes[at..at + written].iter().fold(0i64, |n, d| {
                n.saturating_mul(10).saturating_add(i64::from(d - b'0'))
            });
            exponent = exponent.saturating_add(sign * value);
            at += written;
        }
        if at != bytes.len() {
            return Err(not());
        }
        let first = digits.iter().position(|d| *d != b'0');
        let Some(first) = first else {
            return Ok(Decimal::ZERO);
        };
        let last = digits
            .iter()
            .rposition(|d| *d != b'0')
            .expect("a non-zero digit");
        exponent = exponent.saturating_add((digits.len() - 1 - last) as i64);
        let significant = &digits[first..=last];
        if significant.len() > Decimal::MAX_DIGITS as usize {
            return Err(DecimalError::TooManyDigits);
        }
        let adjusted = exponent.saturating_add(significant.len() as i64 - 1);
        if !(-MAX_ADJUSTED..=MAX_ADJUSTED).contains(&adjusted) {
            return Err(DecimalError::OutOfRange(text.to_string()));
        }
        let magnitude = significant
            .iter()
            .fold(0i64, |n, d| n * 10 + i64::from(d - b'0'));
        Ok(Decimal {
            coefficient: if negative { -magnitude } else { magnitude },
            exponent: i16::try_from(exponent).expect("within ±321 by the range check"),
        })
    }

    /// Reads a JSON number: an integer within the cap, or an `f64` whose shortest round-trip text has at most 15
    /// significant digits (`0.1 + 0.2` is [`DecimalError::TooManyDigits`]).
    pub fn from_json(n: &serde_json::Number) -> Result<Decimal, DecimalError> {
        match n.as_f64() {
            Some(x) if n.is_f64() => Decimal::try_from(x),
            _ => Decimal::parse(&n.to_string()),
        }
    }

    /// The JSON number of the value: an integer when the value is whole and an `i64` or `u64` holds it, otherwise
    /// the binary64 whose shortest text is the decimal.
    pub fn to_json(self) -> serde_json::Number {
        if self.is_whole() {
            let whole = 10i128
                .checked_pow(self.exponent as u32)
                .and_then(|scale| scale.checked_mul(i128::from(self.coefficient)));
            if let Some(whole) = whole {
                if let Ok(n) = i64::try_from(whole) {
                    return n.into();
                }
                if let Ok(n) = u64::try_from(whole) {
                    return n.into();
                }
            }
        }
        serde_json::Number::from_f64(self.to_f64()).expect("a decimal within range is a finite f64")
    }

    /// The binary64 nearest the value, which is exact within the cap: its shortest text is the decimal.
    pub fn to_f64(self) -> f64 {
        format!("{}e{}", self.coefficient, self.exponent)
            .parse()
            .expect("a coefficient and an exponent are an f64 literal")
    }

    /// Whether the value has no fraction.
    pub fn is_whole(self) -> bool {
        self.exponent >= 0
    }

    /// Whether the value is below zero.
    pub fn is_negative(self) -> bool {
        self.coefficient < 0
    }

    /// The count of the coefficient's digits, 1 for zero.
    fn digits(self) -> i64 {
        let mut n = self.coefficient.unsigned_abs();
        let mut count = 1;
        while n >= 10 {
            n /= 10;
            count += 1;
        }
        count
    }

    /// The magnitude as (adjusted exponent, coefficient padded to 15 digits), which orders non-zero magnitudes.
    fn magnitude(self) -> (i64, u64) {
        let digits = self.digits();
        let padded = self.coefficient.unsigned_abs()
            * 10u64.pow((Decimal::MAX_DIGITS as i64 - digits) as u32);
        (i64::from(self.exponent) + digits - 1, padded)
    }
}

impl fmt::Display for Decimal {
    /// The canonical text: plain notation, no exponent, no trailing fraction zero, `-` for a negative value
    /// (`0.0002`, `1500`, `-2.25`). It is what Rust's `{}` prints for [`Decimal::to_f64`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.coefficient.unsigned_abs().to_string();
        let sign = if self.is_negative() { "-" } else { "" };
        if self.exponent >= 0 {
            let zeros = "0".repeat(self.exponent as usize);
            return f.pad(&format!("{sign}{digits}{zeros}"));
        }
        let fraction = self.exponent.unsigned_abs() as usize;
        let text = if digits.len() > fraction {
            let (whole, part) = digits.split_at(digits.len() - fraction);
            format!("{sign}{whole}.{part}")
        } else {
            format!("{sign}0.{}{digits}", "0".repeat(fraction - digits.len()))
        };
        f.pad(&text)
    }
}

impl FromStr for Decimal {
    type Err = DecimalError;

    fn from_str(text: &str) -> Result<Decimal, DecimalError> {
        Decimal::parse(text)
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Decimal) -> Ordering {
        let sign = |d: &Decimal| d.coefficient.signum();
        sign(self).cmp(&sign(other)).then_with(|| match sign(self) {
            0 => Ordering::Equal,
            1 => self.magnitude().cmp(&other.magnitude()),
            _ => other.magnitude().cmp(&self.magnitude()),
        })
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Decimal) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<i32> for Decimal {
    fn from(n: i32) -> Decimal {
        Decimal::try_from(i64::from(n)).expect("an i32 has at most 10 digits")
    }
}

impl TryFrom<i64> for Decimal {
    type Error = DecimalError;

    /// A whole number of at most 15 significant digits (`10^18` is one digit).
    fn try_from(n: i64) -> Result<Decimal, DecimalError> {
        Decimal::parse(&n.to_string())
    }
}

impl TryFrom<f64> for Decimal {
    type Error = DecimalError;

    /// The decimal of the `f64`'s shortest round-trip text (`95.0` is 95); `NaN` and the infinities are no decimal.
    fn try_from(x: f64) -> Result<Decimal, DecimalError> {
        Decimal::parse(&format!("{x}"))
    }
}

impl From<Decimal> for serde_json::Number {
    fn from(d: Decimal) -> serde_json::Number {
        d.to_json()
    }
}

impl Serialize for Decimal {
    /// A plain JSON number ([`Decimal::to_json`]).
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_json().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Decimal {
    /// A JSON number [`Decimal::from_json`] takes; a string is refused.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Decimal, D::Error> {
        let n = serde_json::Number::deserialize(deserializer)?;
        Decimal::from_json(&n).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
