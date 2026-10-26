//! An instant a registry answers in (ADR 0265).

use serde::{Deserialize, Serialize};

/// An instant a registry answers in, RFC 3339 (a token's expiry), read where the reply is read: nothing downstream
/// parses or prints its text (ADR 0265). Written back as RFC 3339 (`+00:00` for UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Timestamp(chrono::DateTime<chrono::FixedOffset>);

impl Timestamp {
    /// `text` as an RFC 3339 instant; the reason (`{text:?} is not an RFC 3339 timestamp`) when not.
    pub fn parse(text: &str) -> Result<Timestamp, String> {
        chrono::DateTime::parse_from_rfc3339(text)
            .map(Timestamp)
            .map_err(|_| format!("{text:?} is not an RFC 3339 timestamp"))
    }

    /// `at` as a timestamp.
    pub fn of(at: chrono::DateTime<chrono::Utc>) -> Timestamp {
        Timestamp(at.fixed_offset())
    }

    /// Whole days from `now` to it (negative once past, truncated toward zero).
    pub fn days_after(&self, now: chrono::DateTime<chrono::Utc>) -> i64 {
        (self.0.with_timezone(&chrono::Utc) - now).num_days()
    }

    /// Whether `now` is after it.
    pub fn is_past(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        now > self.0
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_rfc3339())
    }
}

impl TryFrom<String> for Timestamp {
    type Error = String;

    fn try_from(text: String) -> Result<Timestamp, String> {
        Timestamp::parse(&text)
    }
}

impl From<Timestamp> for String {
    fn from(timestamp: Timestamp) -> String {
        timestamp.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn a_timestamp_is_rfc_3339_and_writes_back_as_rfc_3339() {
        let z = Timestamp::parse("2027-01-01T00:00:00Z").unwrap();
        assert_eq!(z.to_string(), "2027-01-01T00:00:00+00:00");
        let offset = Timestamp::parse("2027-01-01T00:00:00+02:00").unwrap();
        assert_eq!(offset.to_string(), "2027-01-01T00:00:00+02:00");
        for bad in ["", "2027-01-01", "next tuesday", "soon\u{1b}]0;x\u{7}"] {
            let why = Timestamp::parse(bad).unwrap_err();
            assert!(why.contains("not an RFC 3339 timestamp"), "{why}");
        }
        let now = chrono::Utc.with_ymd_and_hms(2026, 12, 29, 0, 0, 0).unwrap();
        assert_eq!(z.days_after(now), 3);
        assert!(!z.is_past(now));
        let later = chrono::Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 1).unwrap();
        assert!(z.is_past(later));
        assert_eq!(z.days_after(later), 0, "truncated toward zero");
        assert_eq!(Timestamp::of(now).to_string(), "2026-12-29T00:00:00+00:00");
    }
}
