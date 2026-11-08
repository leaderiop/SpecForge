//! GSD's quick id: `YYMMDD-xxx`, six digits, a dash, three base-36 digits (`src/init.cts`, lowercase).

/// A quick id.
pub(crate) struct QuickId(String);

impl QuickId {
    /// How many ids a day has: three base-36 digits.
    pub(crate) const PER_DAY: u64 = 36 * 36 * 36;

    /// `name` as a quick id, if it is one.
    pub(crate) fn parse(name: &str) -> Option<Self> {
        let bytes = name.as_bytes();
        let is_one = bytes.len() == 10
            && bytes[..6].iter().all(u8::is_ascii_digit)
            && bytes[6] == b'-'
            && bytes[7..]
                .iter()
                .all(|b| b.is_ascii_digit() || b.is_ascii_lowercase());
        is_one.then(|| Self(name.to_string()))
    }

    /// The `n`th id of `day` (`yymmdd`), `n` wrapping at a day's end.
    pub(crate) fn nth(day: &str, n: u64) -> Self {
        let n = n % Self::PER_DAY;
        let digits: String = [n / (36 * 36), n / 36 % 36, n % 36]
            .iter()
            .map(|digit| char::from_digit(*digit as u32, 36).expect("a digit under 36"))
            .collect();
        Self(format!("{day}-{digits}"))
    }

    /// Whether this is an id of `day` (`yymmdd`).
    pub(crate) fn is_of(&self, day: &str) -> bool {
        self.0.split('-').next() == Some(day)
    }

    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::QuickId;

    #[test]
    fn a_quick_id_is_six_digits_a_dash_and_three_base_36_digits() {
        for good in ["261028-a1b", "000000-000", "991231-zzz"] {
            assert!(QuickId::parse(good).is_some(), "{good}");
        }
        for bad in [
            "261028-A1B",
            "26102-a1bc",
            "261028-a1",
            "261028_a1b",
            "03-01",
        ] {
            assert!(QuickId::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn the_nth_id_of_a_day_counts_in_base_36_and_wraps() {
        let nth = |n| QuickId::nth("261105", n).into_string();
        assert_eq!(nth(0), "261105-000");
        assert_eq!(nth(36 + 11), "261105-01b");
        assert_eq!(nth(QuickId::PER_DAY - 1), "261105-zzz");
        assert_eq!(nth(QuickId::PER_DAY), "261105-000");
        assert!(
            QuickId::parse(&nth(7)).is_some_and(|id| id.is_of("261105") && !id.is_of("261106"))
        );
    }
}
