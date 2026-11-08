//! [`CATALOG`], the catalog tables (`catalog.rs`, `catalog_checks.rs`, `catalog_registry.rs`) merged at compile time.

use crate::CodeEntry;
use crate::catalog::TABLE;
use crate::catalog_checks::TABLE as CHECKS;
use crate::catalog_registry::TABLE as REGISTRY;

/// The core and checks tables' length.
const LENGTH: usize = TABLE.len() + CHECKS.len();
/// Every table's length.
const ALL: usize = LENGTH + REGISTRY.len();

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

/// Two tables, each sorted, as one sorted table of their `N` entries.
const fn merge<const N: usize>(first: &[CodeEntry], second: &[CodeEntry]) -> [CodeEntry; N] {
    let mut all = [first[0]; N];
    let (mut left, mut right, mut at) = (0, 0, 0);
    while at < N {
        let take_right = left == first.len()
            || (right < second.len() && before(second[right].code, first[left].code));
        if take_right {
            all[at] = second[right];
            right += 1;
        } else {
            all[at] = first[left];
            left += 1;
        }
        at += 1;
    }
    all
}

/// Every table, each sorted, as one sorted table.
const fn merged() -> [CodeEntry; ALL] {
    let core_and_checks: [CodeEntry; LENGTH] = merge(TABLE, CHECKS);
    merge(&core_and_checks, REGISTRY)
}

/// Every catalogued code, sorted by code.
pub const CATALOG: &[CodeEntry] = &merged();
