//! [`CATALOG`], the catalog tables merged at compile time.

use crate::CodeEntry;
use crate::catalog::TABLE;
use crate::catalog_checks::TABLE as CHECKS;

/// Both tables' length.
const LENGTH: usize = TABLE.len() + CHECKS.len();

/// Whether code `a` sorts before code `b`.
const fn before(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut at = 0;
    while at < a.len() && at < b.len() {
        if a[at] != b[at] {
            return a[at] < b[at];
        }
        at += 1;
    }
    a.len() < b.len()
}

/// Both tables, each sorted, as one sorted table.
const fn merged() -> [CodeEntry; LENGTH] {
    let mut all = [TABLE[0]; LENGTH];
    let (mut left, mut right, mut at) = (0, 0, 0);
    while at < LENGTH {
        let take_right = left == TABLE.len()
            || (right < CHECKS.len() && before(CHECKS[right].code, TABLE[left].code));
        if take_right {
            all[at] = CHECKS[right];
            right += 1;
        } else {
            all[at] = TABLE[left];
            left += 1;
        }
        at += 1;
    }
    all
}

/// Every catalogued code, sorted by code.
pub const CATALOG: &[CodeEntry] = &merged();
