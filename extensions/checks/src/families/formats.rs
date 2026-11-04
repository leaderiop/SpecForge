//! The formats family: twenty notations, each decided by its own standard from the text alone.

use specforge_checks_kit::formats::{self as kit, CodeForm, DateForm};
use specforge_extension_sdk::prelude::*;

use super::{flag, well_formed, word, Args, TEXT};

const SWITCH: &str = "families.formats";
const DATE_FORM: Vocabulary = Vocabulary::new(&["date", "datetime", "any"]);
const CODE_FORM: Vocabulary = Vocabulary::new(&["alpha2", "alpha3", "numeric", "any"]);
const IP_VERSION: Vocabulary = Vocabulary::new(&["4", "6"]);

fn code_form(args: &Args) -> CodeForm {
    match word(args, "form") {
        Some("alpha2") => CodeForm::Alpha2,
        Some("alpha3") => CodeForm::Alpha3,
        Some("numeric") => CodeForm::Numeric,
        _ => CodeForm::Any,
    }
}

fn date_form(args: &Args) -> DateForm {
    match word(args, "form") {
        Some("date") => DateForm::Date,
        Some("datetime") => DateForm::DateTime,
        _ => DateForm::Any,
    }
}

fn no_args(_: &mut CheckerBuilder) {}

fn bool_arg(name: &'static str) -> impl FnOnce(&mut CheckerBuilder) {
    move |k| {
        k.arg(name, |f| {
            f.field_type(FieldType::Bool);
        });
    }
}

fn form_arg(forms: &'static Vocabulary) -> impl FnOnce(&mut CheckerBuilder) {
    move |k| {
        k.arg("form", |f| {
            f.vocabulary(forms);
        });
    }
}

pub(super) fn contribute(c: &mut ContributionsBuilder) {
    well_formed(c, SWITCH, "email", TEXT, no_args, |t, _| kit::email(t));
    well_formed(c, SWITCH, "semver", TEXT, bool_arg("prerelease"), |t, a| {
        kit::semver(t, flag(a, "prerelease", true))
    });
    well_formed(c, SWITCH, "semver_range", TEXT, no_args, |t, _| {
        kit::semver_range(t)
    });
    well_formed(c, SWITCH, "spdx", TEXT, bool_arg("deprecated"), |t, a| {
        kit::spdx(t, flag(a, "deprecated", false))
    });
    let dated = &["string", "string_list", "date"];
    well_formed(
        c,
        SWITCH,
        "iso_date",
        dated,
        form_arg(&DATE_FORM),
        |t, a| kit::iso_date(t, date_form(a)),
    );
    well_formed(c, SWITCH, "iso_duration", TEXT, no_args, |t, _| {
        kit::iso_duration(t)
    });
    well_formed(c, SWITCH, "cron", TEXT, bool_arg("seconds"), |t, a| {
        kit::cron(t, flag(a, "seconds", false))
    });
    well_formed(c, SWITCH, "regex", TEXT, no_args, |t, _| kit::regex(t));
    let uuid_version = |k: &mut CheckerBuilder| {
        k.arg("version", |f| {
            f.field_type(FieldType::Integer).minimum(1).maximum(8);
        });
    };
    well_formed(c, SWITCH, "uuid", TEXT, uuid_version, |t, a| {
        let version = a.get("version").and_then(serde_json::Value::as_u64);
        kit::uuid(t, version.and_then(|v| u8::try_from(v).ok()))
    });
    well_formed(c, SWITCH, "mime", TEXT, no_args, |t, _| kit::mime(t));
    well_formed(c, SWITCH, "bcp47", TEXT, no_args, |t, _| kit::bcp47(t));
    well_formed(c, SWITCH, "iso3166", TEXT, form_arg(&CODE_FORM), |t, a| {
        kit::iso3166(t, code_form(a))
    });
    well_formed(c, SWITCH, "iso4217", TEXT, form_arg(&CODE_FORM), |t, a| {
        kit::iso4217(t, code_form(a))
    });
    well_formed(c, SWITCH, "iana_tz", TEXT, bool_arg("canonical"), |t, a| {
        kit::iana_tz(t, flag(a, "canonical", false))
    });
    well_formed(c, SWITCH, "e164", TEXT, no_args, |t, _| kit::e164(t));
    let ip_version = |k: &mut CheckerBuilder| {
        k.arg("version", |f| {
            f.vocabulary(&IP_VERSION);
        });
    };
    well_formed(c, SWITCH, "ip", TEXT, ip_version, |t, a| {
        kit::ip(t, word(a, "version").and_then(|v| v.parse().ok()))
    });
    well_formed(c, SWITCH, "cidr", TEXT, bool_arg("strict"), |t, a| {
        kit::cidr(t, flag(a, "strict", true))
    });
    well_formed(c, SWITCH, "hostname", TEXT, bool_arg("wildcard"), |t, a| {
        kit::hostname(t, flag(a, "wildcard", false))
    });
    let numbered = &["string", "string_list", "integer"];
    well_formed(c, SWITCH, "port", numbered, bool_arg("zero"), |t, a| {
        kit::port(t, flag(a, "zero", false))
    });
    well_formed(c, SWITCH, "hex_color", TEXT, no_args, |t, _| {
        kit::hex_color(t)
    });
}
