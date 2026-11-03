//! The one clock of the effect plane (ADR 0337 D3, D5): when a record was observed and when it stops answering, in
//! milliseconds since the Unix epoch. A clock before the epoch is an error, never a silent 0.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// An instant, in milliseconds since the Unix epoch.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct UnixMillis(pub u64);

/// The clock reads an instant before the Unix epoch, or one past what milliseconds in a `u64` hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockError;

impl std::fmt::Display for ClockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the system clock is before the Unix epoch or out of range")
    }
}

impl std::error::Error for ClockError {}

impl UnixMillis {
    /// The last instant: what [`Self::now_or_end`] answers when the clock cannot be read.
    pub const END: UnixMillis = UnixMillis(u64::MAX);

    /// `time` in milliseconds since the epoch.
    pub fn of(time: SystemTime) -> Result<Self, ClockError> {
        let since = time.duration_since(UNIX_EPOCH).map_err(|_| ClockError)?;
        u64::try_from(since.as_millis())
            .map(UnixMillis)
            .map_err(|_| ClockError)
    }

    /// Now.
    pub fn now() -> Result<Self, ClockError> {
        Self::of(SystemTime::now())
    }

    /// Now; [`Self::END`] when the clock cannot be read, so no record is fresh by a clock that cannot vouch for it.
    pub fn now_or_end() -> Self {
        Self::now().unwrap_or(Self::END)
    }

    /// `self` and `span` later, held at [`Self::END`].
    pub fn plus(self, span: Duration) -> Self {
        UnixMillis(
            self.0
                .saturating_add(u64::try_from(span.as_millis()).unwrap_or(u64::MAX)),
        )
    }

    /// How long from `self` until `later`; zero when `later` is not after it.
    pub fn until(self, later: UnixMillis) -> Duration {
        Duration::from_millis(later.0.saturating_sub(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clock_before_the_epoch_is_an_error_not_zero() {
        let before = UNIX_EPOCH - Duration::from_secs(1);
        assert_eq!(UnixMillis::of(before), Err(ClockError));
        assert_eq!(
            UnixMillis::of(UNIX_EPOCH + Duration::from_millis(5)),
            Ok(UnixMillis(5))
        );
        assert_eq!(
            UnixMillis(1).plus(Duration::from_secs(1)),
            UnixMillis(1_001)
        );
        assert_eq!(
            UnixMillis::END.plus(Duration::from_secs(1)),
            UnixMillis::END
        );
        assert_eq!(UnixMillis(10).until(UnixMillis(4)), Duration::ZERO);
    }
}
