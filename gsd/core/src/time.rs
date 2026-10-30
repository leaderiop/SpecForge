//! UTC times as RFC 3339 text, and back, by the days-from-civil algorithm (no dependency).
//!
//! Also ports gsd-core bin/lib modules: clock.cjs.

/// `seconds` since the epoch as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn utc(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// The days from 1970-01-01 to the civil date `year-month-day`.
fn days(year: i64, month: i64, day: i64) -> i64 {
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let year_of_era = y.rem_euclid(400);
    let shifted_month = (month + 9) % 12;
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Seconds since the epoch of `YYYY-MM-DDTHH:MM:SS[.fraction]Z`; `None` for any other spelling (`gsd/E107` reports
/// those on an authored time).
pub fn seconds(time: &str) -> Option<i64> {
    let body = time.strip_suffix('Z')?;
    let (date, clock) = body.split_once('T')?;
    let clock = clock.split_once('.').map_or(clock, |(whole, _)| whole);
    let number = |part: Option<&str>| part?.parse::<i64>().ok();
    let mut d = date.split('-');
    let (year, month, day) = (number(d.next())?, number(d.next())?, number(d.next())?);
    let mut c = clock.split(':');
    let (hour, minute, second) = (number(c.next())?, number(c.next())?, number(c.next())?);
    if d.next().is_some()
        || c.next().is_some()
        || !(1..=12).contains(&month)
        || !(1..=days_in(year, month)).contains(&day)
        || !(0..24).contains(&hour)
        || !(0..60).contains(&minute)
        || !(0..=60).contains(&second)
    {
        return None;
    }
    Some(days(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second)
}

fn days_in(year: i64, month: i64) -> i64 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_impossible_date_is_no_time() {
        assert_eq!(seconds("2026-02-31T00:00:00Z"), None);
        assert_eq!(seconds("2026-04-31T00:00:00Z"), None);
        assert_eq!(seconds("2100-02-29T00:00:00Z"), None);
        assert!(seconds("2024-02-29T00:00:00Z").is_some());
    }

    #[test]
    fn unix_seconds_become_utc_rfc3339() {
        assert_eq!(utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(utc(1_709_164_799), "2024-02-28T23:59:59Z");
        assert_eq!(utc(1_709_164_800), "2024-02-29T00:00:00Z");
        assert_eq!(utc(1_709_251_200), "2024-03-01T00:00:00Z");
        assert_eq!(utc(1_761_732_000), "2025-10-29T10:00:00Z");
        assert_eq!(utc(2_147_483_648), "2038-01-19T03:14:08Z");
        assert_eq!(utc(4_107_542_400), "2100-03-01T00:00:00Z");
        assert_eq!(utc(-1), "1969-12-31T23:59:59Z");
    }

    #[test]
    fn a_time_reads_back_to_its_seconds() {
        for s in [
            0,
            951_782_400,
            1_709_164_799,
            1_761_732_000,
            2_147_483_648,
            4_107_542_400,
            -1,
            -86_400 * 800,
        ] {
            assert_eq!(seconds(&utc(s)), Some(s), "{s}");
        }
        assert_eq!(
            seconds("2026-10-29T09:00:00.250Z"),
            seconds("2026-10-29T09:00:00Z")
        );
        for refused in [
            "2026-10-29",
            "2026-10-29T09:00:00",
            "2026-13-01T00:00:00Z",
            "2026-10-29T24:00:00Z",
            "x",
        ] {
            assert_eq!(seconds(refused), None, "{refused}");
        }
    }
}
