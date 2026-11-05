//! The formats family: twenty notations, each decided by its own standard from the text alone.

use specforge_checks_kit::formats::{self as kit, CodeForm, DateForm};
use specforge_extension_sdk::prelude::*;

use super::{choice_arg, no_args, well_formed, Word, FORMATS, TEXT};

const IP_VERSION: Vocabulary = Vocabulary::new(&["4", "6"]);

impl Word for DateForm {
    const VOCABULARY: Vocabulary = Vocabulary::new(&["date", "datetime", "any"]);
    const ALL: &'static [Self] = &[DateForm::Date, DateForm::DateTime, DateForm::Any];
}

impl Word for CodeForm {
    const VOCABULARY: Vocabulary = Vocabulary::new(&["alpha2", "alpha3", "numeric", "any"]);
    const ALL: &'static [Self] = &[
        CodeForm::Alpha2,
        CodeForm::Alpha3,
        CodeForm::Numeric,
        CodeForm::Any,
    ];
}

fn bool_arg(name: &'static str) -> impl FnOnce(&mut CheckerBuilder) {
    move |k| {
        k.arg(name, |f| {
            f.field_type(FieldType::Bool);
        });
    }
}

pub(super) fn contribute(c: &mut ContributionsBuilder) {
    well_formed(c, &FORMATS, "email", TEXT, no_args, |t, _| kit::email(t));
    well_formed(
        c,
        &FORMATS,
        "semver",
        TEXT,
        bool_arg("prerelease"),
        |t, a| kit::semver(t, a.flag("prerelease", true)),
    );
    well_formed(c, &FORMATS, "semver_range", TEXT, no_args, |t, _| {
        kit::semver_range(t)
    });
    well_formed(c, &FORMATS, "spdx", TEXT, bool_arg("deprecated"), |t, a| {
        kit::spdx(t, a.flag("deprecated", false))
    });
    let dated = &["string", "string_list", "date"];
    well_formed(
        c,
        &FORMATS,
        "iso_date",
        dated,
        choice_arg::<DateForm>("form"),
        |t, a| kit::iso_date(t, a.choice("form").unwrap_or_default()),
    );
    well_formed(c, &FORMATS, "iso_duration", TEXT, no_args, |t, _| {
        kit::iso_duration(t)
    });
    well_formed(c, &FORMATS, "cron", TEXT, bool_arg("seconds"), |t, a| {
        kit::cron(t, a.flag("seconds", false))
    });
    well_formed(c, &FORMATS, "regex", TEXT, no_args, |t, _| kit::regex(t));
    let uuid_version = |k: &mut CheckerBuilder| {
        k.arg("version", |f| {
            f.field_type(FieldType::Integer).minimum(1).maximum(8);
        });
    };
    well_formed(c, &FORMATS, "uuid", TEXT, uuid_version, |t, a| {
        kit::uuid(t, a.count("version").and_then(|v| u8::try_from(v).ok()))
    });
    well_formed(c, &FORMATS, "mime", TEXT, no_args, |t, _| kit::mime(t));
    well_formed(c, &FORMATS, "bcp47", TEXT, no_args, |t, _| kit::bcp47(t));
    well_formed(
        c,
        &FORMATS,
        "iso3166",
        TEXT,
        choice_arg::<CodeForm>("form"),
        |t, a| kit::iso3166(t, a.choice("form").unwrap_or_default()),
    );
    well_formed(
        c,
        &FORMATS,
        "iso4217",
        TEXT,
        choice_arg::<CodeForm>("form"),
        |t, a| kit::iso4217(t, a.choice("form").unwrap_or_default()),
    );
    well_formed(
        c,
        &FORMATS,
        "iana_tz",
        TEXT,
        bool_arg("canonical"),
        |t, a| kit::iana_tz(t, a.flag("canonical", false)),
    );
    well_formed(c, &FORMATS, "e164", TEXT, no_args, |t, _| kit::e164(t));
    let ip_version = |k: &mut CheckerBuilder| {
        k.arg("version", |f| {
            f.vocabulary(&IP_VERSION);
        });
    };
    well_formed(c, &FORMATS, "ip", TEXT, ip_version, |t, a| {
        kit::ip(t, a.word("version").and_then(|v| v.parse().ok()))
    });
    well_formed(c, &FORMATS, "cidr", TEXT, bool_arg("strict"), |t, a| {
        kit::cidr(t, a.flag("strict", true))
    });
    well_formed(
        c,
        &FORMATS,
        "hostname",
        TEXT,
        bool_arg("wildcard"),
        |t, a| kit::hostname(t, a.flag("wildcard", false)),
    );
    let numbered = &["string", "string_list", "integer"];
    well_formed(c, &FORMATS, "port", numbered, bool_arg("zero"), |t, a| {
        kit::port(t, a.flag("zero", false))
    });
    well_formed(c, &FORMATS, "hex_color", TEXT, no_args, |t, _| {
        kit::hex_color(t)
    });
}
