//! Static tables, each naming the source it was read from (ADR 0338 D7). A table is refreshed from its source before
//! every release, with its `SOURCE` and that `SOURCE`'s pin in one commit (`docs/agents/revendor.md`).

#[cfg(feature = "digits")]
pub mod iban;

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
