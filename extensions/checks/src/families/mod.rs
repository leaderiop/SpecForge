//! The families: what each may make the host do and the setting that switches it.

use serde_json::{Map, Value};
use specforge_checks_kit::Invalid;
use specforge_extension_sdk::prelude::*;

use crate::codes::*;

mod digits;
pub(crate) mod files;
mod formats;
mod git;
pub(crate) mod links;
mod media;
mod registries;
mod structured;

/// A family: the setting that switches it, the scope its probes run under (none when it reads only the text), the codes
/// only it reports, and what the setting says. E103, E016 and W176 are shared, as are the network's W172, W173, W181
/// and W182, so no switch gates them.
pub(crate) struct Family {
    pub(crate) switch: &'static str,
    pub(crate) scope: Option<&'static str>,
    gates: &'static [&'static str],
    describe: &'static str,
}

pub(crate) const FILES: Family = Family {
    switch: "families.files",
    scope: Some("files"),
    gates: &[E096],
    describe: "Check the files and directories the values name",
};
pub(crate) const STRUCTURED: Family = Family {
    switch: "families.structured",
    scope: Some("files"),
    gates: &[E097, E098],
    describe: "Check that JSON, YAML and TOML files parse, and that pointers and Markdown anchors into files exist",
};
pub(crate) const FORMATS: Family = Family {
    switch: "families.formats",
    scope: None,
    gates: &[],
    describe:
        "Check that values are well formed: dates, versions, licences, addresses and the like",
};
pub(crate) const DIGITS: Family = Family {
    switch: "families.digits",
    scope: None,
    gates: &[],
    describe: "Check the check digits of the identifiers values carry",
};
pub(crate) const LINKS: Family = Family {
    switch: "families.links",
    scope: Some("links"),
    gates: &[E091, E092, E093, E104, E112, W178, W183],
    describe: "Check that the URLs values name answer",
};
pub(crate) const MEDIA: Family = Family {
    switch: "families.media",
    scope: Some("media-remote"),
    gates: &[E099, E113, W177],
    describe:
        "Check that images, SVGs, PDFs and media files are what they say, within their limits",
};
pub(crate) const GIT: Family = Family {
    switch: "families.git",
    scope: Some("git"),
    gates: &[E106, W180],
    describe: "Check the commits, refs and tracked paths values name",
};

pub(crate) const REGISTRIES: Family = Family {
    switch: "families.registries",
    scope: Some("registries"),
    gates: &[E105, W179],
    describe: "Check that packages, DOIs and ORCID iDs exist, and the advisories and licences of the packages",
};

const FAMILIES: [&Family; 8] = [
    &FILES,
    &FORMATS,
    &DIGITS,
    &LINKS,
    &GIT,
    &STRUCTURED,
    &MEDIA,
    &REGISTRIES,
];

/// A family's scope, which it declares.
fn scope_of(family: &Family) -> &'static str {
    family.scope.expect("a family with probes has a scope")
}

pub(crate) fn contribute(c: &mut ContributionsBuilder) {
    c.capability(
        Capability::fs(scope_of(&FILES))
            .ops(&[FsAccess::Stat, FsAccess::Read, FsAccess::Walk])
            .why("check the files and directories the values name"),
    );
    c.capability(
        Capability::http(scope_of(&LINKS))
            .from_values()
            .schemes(&["https", "http"])
            .methods(&[HttpMethod::Head, HttpMethod::Get])
            .why("check that the URLs the values name answer"),
    );
    c.capability(
        Capability::git(scope_of(&GIT))
            .why("check the commits, refs and tracked paths the values name"),
    );
    c.capability(
        Capability::http(scope_of(&MEDIA))
            .from_values()
            .schemes(&["https"])
            .methods(&[HttpMethod::Get])
            .why("read the head or the tail of the remote images, SVGs, PDFs and media files the values name"),
    );
    c.capability(
        Capability::http(scope_of(&REGISTRIES))
            .call_kinds(&[Operation::CheckValues])
            .hosts(&registries::HOSTS)
            .schemes(&["https"])
            .methods(&[HttpMethod::Get, HttpMethod::Post])
            .max_bytes(registries::ANSWER)
            .why("check that packages, DOIs and ORCID iDs exist, and read the advisories and licences of the packages"),
    );
    for family in FAMILIES {
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
    git::contribute(c);
    structured::contribute(c);
    media::contribute(c);
    registries::contribute(c);
}

/// A checker's arguments as the host admitted them, read by type: one that is missing, or of another type, is absent.
#[derive(Clone, Copy)]
pub(crate) struct Args<'a>(&'a Map<String, Value>);

impl<'a> Args<'a> {
    fn get(self, name: &str) -> Option<&'a Value> {
        self.0.get(name)
    }

    fn flag(self, name: &str, otherwise: bool) -> bool {
        self.get(name).and_then(Value::as_bool).unwrap_or(otherwise)
    }

    fn word(self, name: &str) -> Option<&'a str> {
        self.get(name).and_then(Value::as_str)
    }

    /// A string list's strings.
    fn words(self, name: &str) -> Vec<&'a str> {
        self.get(name)
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    /// A count: an integer of at least 0.
    fn count(self, name: &str) -> Option<u64> {
        self.get(name).and_then(Value::as_u64)
    }

    fn integer(self, name: &str) -> Option<i64> {
        self.get(name).and_then(Value::as_i64)
    }

    /// An argument of a closed vocabulary, as its variant.
    fn choice<W: Word>(self, name: &str) -> Option<W> {
        let word = self.word(name)?;
        let at = W::VOCABULARY.values().iter().position(|w| *w == word)?;
        W::ALL.get(at).copied()
    }

    /// Where a bare path starts: the `root` argument, which the host admits only as a root's name, else the project.
    /// The host reads it the same way (`CheckRegistry::default_root`).
    fn root(self) -> Root {
        Root::named_by(self.0, "root").unwrap_or(Root::Project)
    }
}

/// A closed set of words an argument takes, one variant each: `VOCABULARY` declares them, and `ALL` holds the variants
/// in the same order, so a word is parsed once, by its place.
pub(crate) trait Word: Copy + 'static {
    const VOCABULARY: Vocabulary;
    const ALL: &'static [Self];
}

/// The argument `name`, one word of `W`.
fn choice_arg<W: Word>(name: &'static str) -> impl FnOnce(&mut CheckerBuilder) {
    move |k| {
        k.arg(name, |f| {
            f.vocabulary(&W::VOCABULARY);
        });
    }
}

/// The field types a checker of text reads.
const TEXT: &[&str] = &["string", "string_list"];

/// A checker that takes no argument.
fn no_args(_: &mut CheckerBuilder) {}

/// A path value under its `root` argument, with `args`.
fn path_shape(args: impl FnOnce(&mut CheckerBuilder)) -> impl FnOnce(&mut CheckerBuilder) {
    move |k| {
        k.arg("root", |f| {
            f.field_type(FieldType::String);
        });
        args(k);
        k.value(ValueForm::Path).root_arg("root");
    }
}

/// A checker of `family` over text, its other arguments and forms set by `shape`. `plan` reads a value's text: the
/// probes it needs, or the finding that stops them. `judge` reads what they answered. An empty value is the kernel's
/// `required` to judge, and a value whose probes were not answered is not judged.
fn checker(
    c: &mut ContributionsBuilder,
    family: &Family,
    name: &str,
    codes: &[&str],
    shape: impl FnOnce(&mut CheckerBuilder),
    plan: impl Fn(&str, Args) -> Result<Vec<Probe>, CheckFinding> + 'static,
    judge: impl Fn(&str, Args, &[ProbeOutcome]) -> Vec<CheckFinding> + 'static,
) {
    let judge = move |text: &str, args: Args, outcomes: &[ProbeOutcome], _: Option<&Value>| {
        Judged::Findings(judge(text, args, outcomes))
    };
    rounds_checker(c, family, name, codes, shape, plan, judge);
}

/// [`checker`], whose `judge` may answer more probes for another round instead of findings, handed back the `carry` it
/// asked for with every round's outcomes so far.
fn rounds_checker(
    c: &mut ContributionsBuilder,
    family: &Family,
    name: &str,
    codes: &[&str],
    shape: impl FnOnce(&mut CheckerBuilder),
    plan: impl Fn(&str, Args) -> Result<Vec<Probe>, CheckFinding> + 'static,
    judge: impl Fn(&str, Args, &[ProbeOutcome], Option<&Value>) -> Judged + 'static,
) {
    c.checker(name, |k| {
        k.applies_to(TEXT);
        shape(k);
        if let Some(scope) = family.scope {
            k.scope(scope);
        }
        k.codes(codes)
            .when(family.switch)
            .plan(move |item| match plan(&item.text, Args(&item.args)) {
                _ if item.text.is_empty() => PlannedItem::default(),
                Ok(probes) if probes.is_empty() => PlannedItem::default(),
                Ok(probes) => PlannedItem::probes(probes),
                Err(found) => PlannedItem::findings(vec![found]),
            })
            .judge_rounds(move |item| match item.outcomes.as_slice() {
                [] => Judged::Findings(Vec::new()),
                outcomes => judge(&item.text, Args(&item.args), outcomes, item.carry.as_ref()),
            });
    });
}

/// A checker the text alone decides: a refusal is E103, on the part the kit points at, and it needs no scope.
fn well_formed(
    c: &mut ContributionsBuilder,
    family: &Family,
    name: &str,
    types: &[&str],
    args: impl FnOnce(&mut CheckerBuilder),
    check: impl Fn(&str, Args) -> Result<(), Invalid> + 'static,
) {
    let shape = |k: &mut CheckerBuilder| {
        args(k);
        k.applies_to(types);
    };
    let plan = move |text: &str, args: Args| {
        check(text, args)
            .map(|()| Vec::new())
            .map_err(|invalid| finding_of(E103, invalid))
    };
    checker(c, family, name, &[E103], shape, plan, |_, _, _| Vec::new());
}

/// A finding of `code` for what the kit refused, on the part it points at.
fn finding_of(code: &str, invalid: Invalid) -> CheckFinding {
    let finding = CheckFinding::new(code, invalid.reason);
    match invalid.at {
        Some(range) => finding.at(range.start, range.end),
        None => finding,
    }
}
