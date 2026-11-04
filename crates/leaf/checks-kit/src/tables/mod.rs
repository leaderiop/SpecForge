//! Static tables, each naming the source it was read from (ADR 0338 D7). A table is refreshed from its source before
//! every release, with its `SOURCE` and that `SOURCE`'s pin in one commit (`docs/agents/revendor.md`).

#[cfg(feature = "formats")]
pub mod e164;
#[cfg(feature = "formats")]
pub mod iana_tz;
#[cfg(feature = "digits")]
pub mod iban;
#[cfg(feature = "formats")]
pub mod iso3166;
#[cfg(feature = "formats")]
pub mod iso4217;

/// Where a table's rows come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableSource {
    /// The standard it holds rows of.
    pub standard: &'static str,
    /// The edition or release read.
    pub edition: &'static str,
    /// Where it was read from.
    pub url: &'static str,
    /// The day it was read, `YYYY-MM-DD`.
    pub retrieved: &'static str,
}

/// FNV-1a over a table's rows: a pin that fails on an edit of any row.
#[cfg(test)]
pub(crate) fn fingerprint(rows: impl IntoIterator<Item = String>) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for row in rows {
        for byte in row.bytes().chain(*b"\n") {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
}

#[cfg(all(test, feature = "formats"))]
mod tests;
