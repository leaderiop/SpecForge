//! Software notations: SemVer 2.0.0, Cargo's `VersionReq`, SPDX expressions, the crontab, the `regex` crate's syntax,
//! RFC 9562 and RFC 6838. Each crate holds its standard's grammar.

use croner::parser::{CronParser, Seconds, Year};
use spdx::{Expression, ParseMode};

use crate::Invalid;

/// A SemVer 2.0.0 version; a pre-release only when `prerelease`.
pub fn semver(text: &str, prerelease: bool) -> Result<(), Invalid> {
    let version =
        ::semver::Version::parse(text).map_err(|error| Invalid::new(error.to_string()))?;
    if !prerelease && !version.pre.is_empty() {
        let from = text.find('-').unwrap_or(0);
        return Err(Invalid::at(
            "a pre-release is not allowed here",
            from..from + version.pre.len() + 1,
        ));
    }
    Ok(())
}

/// A version requirement in Cargo's `VersionReq` grammar, which is the one this checker names; npm's differs.
pub fn semver_range(text: &str) -> Result<(), Invalid> {
    ::semver::VersionReq::parse(text)
        .map(|_| ())
        .map_err(|error| Invalid::new(error.to_string()))
}

/// An SPDX 2.3 license expression over the crate's list; a deprecated identifier only when `deprecated`.
pub fn spdx(text: &str, deprecated: bool) -> Result<(), Invalid> {
    let mode = ParseMode {
        allow_deprecated: deprecated,
        ..ParseMode::STRICT
    };
    Expression::parse_mode(text, mode)
        .map(|_| ())
        .map_err(|error| Invalid::at(error.reason.to_string(), error.span))
}

/// The version of the SPDX list [`spdx`] reads.
pub const SPDX_LIST_VERSION: &str = ::spdx::identifiers::VERSION;

/// The month and weekday names a crontab may write in a field.
const CRON_NAMES: [&str; 19] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC", "SUN",
    "MON", "TUE", "WED", "THU", "FRI", "SAT",
];

/// A crontab line's schedule: five fields, or six with `seconds` first. croner also reads Quartz's `L`, `W`, `#` and
/// `?` and the `@daily` nicknames, which the crontab format does not have, so they are refused before it.
pub fn cron(text: &str, seconds: bool) -> Result<(), Invalid> {
    let mut at = 0;
    for field in text.split(' ') {
        let span = at..at + field.len();
        let mut rest = field.to_ascii_uppercase();
        for name in CRON_NAMES {
            rest = rest.replace(name, "");
        }
        if !rest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || b"*,-/".contains(&byte))
        {
            return Err(Invalid::at(
                "a crontab field holds numbers, names, `*`, `,`, `-` and `/`",
                span,
            ));
        }
        at += field.len() + 1;
    }
    CronParser::builder()
        .seconds(if seconds {
            Seconds::Required
        } else {
            Seconds::Disallowed
        })
        .year(Year::Disallowed)
        .build()
        .parse(text)
        .map(|_| ())
        .map_err(|error| Invalid::new(error.to_string()))
}

/// A pattern the `regex` crate compiles: exactly what the kernel's `matches` accepts.
pub fn regex(text: &str) -> Result<(), Invalid> {
    ::regex::Regex::new(text).map(|_| ()).map_err(|error| {
        let message = error.to_string();
        // The crate's message repeats the pattern and draws a caret over it; its last line is the reason.
        let reason = message.lines().last().unwrap_or(&message);
        Invalid::new(reason.strip_prefix("error: ").unwrap_or(reason))
    })
}

/// An RFC 9562 UUID in its hyphenated text form, of `version` when one is named.
pub fn uuid(text: &str, version: Option<u8>) -> Result<(), Invalid> {
    // `try_parse` also reads the simple, braced and URN forms.
    if text.len() != 36 {
        return Err(Invalid::new("a UUID is 36 characters: 8-4-4-4-12"));
    }
    let parsed = ::uuid::Uuid::try_parse(text).map_err(|error| Invalid::new(error.to_string()))?;
    match version {
        Some(version) if parsed.get_version_num() != usize::from(version) => Err(Invalid::at(
            format!(
                "this is a version {} UUID, not {version}",
                parsed.get_version_num()
            ),
            14..15,
        )),
        _ => Ok(()),
    }
}

/// An RFC 6838 media type, with its parameters if any.
pub fn mime(text: &str) -> Result<(), Invalid> {
    text.parse::<::mime::Mime>()
        .map(|_| ())
        .map_err(|error| Invalid::new(error.to_string()))
}
