//! One function per format, each answering whether a text is well formed by its standard: the crate that holds the
//! standard's grammar where one exists, a pinned table for a list. Nothing here looks anything up in a registry.

mod locale;
mod net;
mod software;
mod time;

pub use locale::{CodeForm, bcp47, e164, iana_tz, iso3166, iso4217};
pub use net::{cidr, email, hex_color, hostname, ip, port};
pub use software::{SPDX_LIST_VERSION, cron, mime, regex, semver, semver_range, spdx, uuid};
pub use time::{DateForm, iso_date, iso_duration};

#[cfg(test)]
mod tests;
