//! The five families: what each may make the host do and the setting that switches it.

use serde_json::{Map, Value};
use specforge_checks_kit::Invalid;
use specforge_extension_sdk::prelude::*;

use crate::codes::*;

mod digits;
pub(crate) mod files;
mod formats;
pub(crate) mod links;

/// The scopes the families' probes run under. Formats and digits read only the text, so they have none.
pub(crate) const FILES: &str = "files";
pub(crate) const LINKS: &str = "links";
const GIT: &str = "git";

/// A family's switch and the codes only that family reports. E103 is shared, so no switch gates it.
struct Family {
    switch: &'static str,
    gates: &'static [&'static str],
    describe: &'static str,
}

const FAMILIES: [Family; 5] = [
    Family {
        switch: "families.files",
        gates: &[E016, E096, W176],
        describe: "Check the files and directories the values name",
    },
    Family {
        switch: "families.formats",
        gates: &[],
        describe:
            "Check that values are well formed: dates, versions, licences, addresses and the like",
    },
    Family {
        switch: "families.digits",
        gates: &[],
        describe: "Check the check digits of the identifiers values carry",
    },
    Family {
        switch: "families.links",
        gates: &[
            E091, E092, E093, E104, E112, W172, W173, W178, W181, W182, W183,
        ],
        describe: "Check that the URLs values name answer",
    },
    Family {
        switch: "families.git",
        gates: &[E106, W180],
        describe: "Check the commits, refs and tracked paths values name",
    },
];

pub(crate) fn contribute(c: &mut ContributionsBuilder) {
    c.capability(
        Capability::fs(FILES)
            .ops(&[FsAccess::Stat, FsAccess::Read, FsAccess::Walk])
            .why("check the files and directories the values name"),
    );
    c.capability(
        Capability::http(LINKS)
            .from_values()
            .schemes(&["https", "http"])
            .methods(&[HttpMethod::Head, HttpMethod::Get])
            .why("check that the URLs the values name answer"),
    );
    c.capability(
        Capability::git(GIT).why("check the commits, refs and tracked paths the values name"),
    );
    for family in &FAMILIES {
        c.setting(family.switch, |s| {
            s.bool()
                .default(true)
                .describe(family.describe)
                .gates(family.gates);
        });
    }
    files::contribute(c);
    formats::contribute(c);
    digits::contribute(c);
    links::contribute(c);
}

/// A checker's arguments, as the host admitted them.
pub(crate) type Args = Map<String, Value>;

/// The field types a checker of text reads.
const TEXT: &[&str] = &["string", "string_list"];

fn flag(args: &Args, name: &str, otherwise: bool) -> bool {
    args.get(name).and_then(Value::as_bool).unwrap_or(otherwise)
}

fn word<'a>(args: &'a Args, name: &str) -> Option<&'a str> {
    args.get(name).and_then(Value::as_str)
}

/// A checker the text alone decides: a refusal is E103, on the part the kit points at, and it needs no scope. An empty
/// value is the kernel's `required` to judge.
fn well_formed(
    c: &mut ContributionsBuilder,
    switch: &str,
    name: &str,
    types: &[&str],
    args: impl FnOnce(&mut CheckerBuilder),
    check: impl Fn(&str, &Args) -> Result<(), Invalid> + 'static,
) {
    c.checker(name, |k| {
        args(k);
        k.applies_to(types)
            .codes(&[E103])
            .when(switch)
            .plan(move |item| {
                if item.text.is_empty() {
                    return PlannedItem::default();
                }
                let finding = check(&item.text, &item.args).err().map(finding_of);
                PlannedItem::findings(finding.into_iter().collect())
            });
    });
}

fn finding_of(invalid: Invalid) -> CheckFinding {
    let finding = CheckFinding::new(E103, invalid.reason);
    match invalid.at {
        Some(range) => finding.at(range.start, range.end),
        None => finding,
    }
}
