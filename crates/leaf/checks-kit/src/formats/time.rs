//! Dates and durations: the RFC 3339 profile of ISO 8601, and ISO 8601 durations. jiff decides the calendar; RFC 3339 is
//! narrower than what it reads, so its shape is checked first.

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::fmt::temporal::SpanParser;

use crate::Invalid;

/// Which forms a date value may take.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DateForm {
    Date,
    DateTime,
    #[default]
    Any,
}

fn shaped(text: &str, shape: &[u8]) -> bool {
    text.len() == shape.len()
        && text.bytes().zip(shape).all(|(byte, want)| match want {
            b'd' => byte.is_ascii_digit(),
            want => byte == *want,
        })
}

fn date_time_shaped(text: &str) -> bool {
    let Some((date, time)) = text.split_at_checked(10) else {
        return false;
    };
    if !shaped(date, b"dddd-dd-dd") {
        return false;
    }
    let Some(time) = time.strip_prefix('T') else {
        return false;
    };
    let Some((clock, rest)) = time.split_at_checked(8) else {
        return false;
    };
    if !shaped(clock, b"dd:dd:dd") {
        return false;
    }
    let rest = match rest.strip_prefix('.') {
        Some(fraction) => {
            let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
            if digits == 0 {
                return false;
            }
            &fraction[digits..]
        }
        None => rest,
    };
    rest == "Z" || (rest.len() == 6 && (shaped(rest, b"+dd:dd") || shaped(rest, b"-dd:dd")))
}

/// A date (`2024-02-29`) or a date-time with its offset (`2024-02-29T12:00:00Z`), that the calendar holds.
pub fn iso_date(text: &str, form: DateForm) -> Result<(), Invalid> {
    let date = shaped(text, b"dddd-dd-dd");
    let date_time = date_time_shaped(text);
    let result = match (form, date, date_time) {
        (DateForm::Date | DateForm::Any, true, _) => text.parse::<Date>().map(|_| ()),
        (DateForm::DateTime | DateForm::Any, _, true) => text.parse::<Timestamp>().map(|_| ()),
        (DateForm::Date, _, _) => return Err(Invalid::new("write a date as YYYY-MM-DD")),
        (DateForm::DateTime, _, _) => {
            return Err(Invalid::new("write a date-time as YYYY-MM-DDThh:mm:ssZ"));
        }
        (DateForm::Any, _, _) => {
            return Err(Invalid::new(
                "write a date as YYYY-MM-DD or a date-time as YYYY-MM-DDThh:mm:ssZ",
            ));
        }
    };
    result.map_err(|error| Invalid::new(error.to_string()))
}

/// An ISO 8601 duration: `P`, then years, months, weeks and days, then `T` and hours, minutes and seconds. jiff also
/// reads a sign, lower case and its own friendly form, none of which the standard has.
pub fn iso_duration(text: &str) -> Result<(), Invalid> {
    if !text.starts_with('P') || text.bytes().any(|byte| byte.is_ascii_lowercase()) {
        return Err(Invalid::new("an ISO 8601 duration looks like P1DT2H"));
    }
    SpanParser::new()
        .parse_span(text)
        .map(|_| ())
        .map_err(|error| Invalid::new(error.to_string()))
}
