//! The registries family: that a package, a DOI or an ORCID iD a value names exists, at the identifier's own public
//! resolver, and what the registries record of a package's release: its advisories and its licence. Every request goes
//! to one of seven fixed hosts, through the `registries` scope.

use serde_json::{json, Value};
use specforge_checks_kit::coord::{self, Coord, Release};
use specforge_checks_kit::ids;
use specforge_checks_kit::osv;
use specforge_checks_kit::ranges::{newest, newest_unyanked};
use specforge_extension_sdk::prelude::*;

use super::{checker, finding_of, no_args, on, rounds_checker, transport, Args, REGISTRIES};
use crate::codes::*;

/// The hosts the family reads: each identifier's own public resolver.
pub(super) const HOSTS: [&str; 7] = [
    "index.crates.io",
    "crates.io",
    "registry.npmjs.org",
    "pypi.org",
    "api.osv.dev",
    "doi.org",
    "pub.orcid.org",
];
/// The most of an answer read: an index of every release of a large package fits.
pub(super) const ANSWER: u64 = 16 * 1024 * 1024;
/// The codes a registry's answer reports.
const ANSWERS: [&str; 6] = [E103, E105, W172, W173, W181, W182];

/// A finding of `code` on the whole value.
fn get(url: String) -> Probe {
    Probe::HttpGet {
        url,
        range: ByteRange::Whole { max: ANSWER },
    }
}

/// The body of a registry's 2xx answer; else what the value is told: E105 for a 404 or a 410 (`gone`, the message),
/// W181, W182, W172 or W173 as [`transport`] says, W172 for one with no body. An answer cut at the cap leaves
/// the value unchecked: a large package's index is no fault of the project, nor of this machine's network.
fn answered<'o>(
    text: &str,
    outcome: &'o ProbeOutcome,
    gone: &dyn Fn() -> String,
) -> Result<&'o [u8], Judged> {
    let told = |code: &str, message: String| Err(Judged::Findings(vec![on(code, text, message)]));
    match (&outcome.class, &outcome.data) {
        (
            OutcomeClass::Ok,
            OutcomeData::Http {
                body,
                truncated: false,
                ..
            },
        ) => Ok(body),
        (OutcomeClass::Ok | OutcomeClass::Capped, OutcomeData::Http { .. }) => Err(Judged::Unchecked),
        (OutcomeClass::Status(404 | 410), _) => told(E105, gone()),
        (class, _) => match transport(text, outcome, "registries") {
            Some((code, message)) => told(code, message),
            // No body to read (a record that kept none, a range served whole): unchecked, never a pass.
            None => told(
                W172,
                format!("'{text}': the registry's answer has no body to read ({class:?}), so it is not checked"),
            ),
        },
    }
}

/// W172 for an answer that does not read as what the registry serves.
fn unread(text: &str, why: &str) -> CheckFinding {
    on(
        W172,
        text,
        format!("'{text}': the registry's answer does not read ({why})"),
    )
}

/// The value as a package, or E103 on the part that is wrong.
fn package_of(text: &str) -> Result<Coord, CheckFinding> {
    coord::parse(text).map_err(|invalid| finding_of(E103, invalid))
}

/// What the index answered of `coord`: the newest release its requirement admits; else what the value is told: E105
/// when there is no such package or no release meets it, naming the newest that is not yanked.
fn resolved(text: &str, coord: &Coord, outcome: &ProbeOutcome) -> Result<Release, Judged> {
    let gone = || {
        format!(
            "'{text}': {} has no package '{}'",
            coord.ecosystem.as_str(),
            coord.name
        )
    };
    let body = answered(text, outcome, &gone)?;
    let releases = coord::releases(coord.ecosystem, body)
        .map_err(|why| Judged::Findings(vec![unread(text, &why)]))?;
    if let Some(release) = newest(coord.ecosystem, &coord.requirement, &releases) {
        return Ok(release.clone());
    }
    let newest = match newest_unyanked(coord.ecosystem, &releases) {
        Some(release) => format!("the newest release not yanked is {}", release.version),
        None => "every release is yanked".to_string(),
    };
    let finding = on(
        E105,
        text,
        format!(
            "'{text}': no release of '{}' meets its requirement, yanked ones left out: {newest}",
            coord.name
        ),
    );
    Err(Judged::Findings(vec![match &coord.requirement_at {
        Some(at) => finding.at(at.start, at.end),
        None => finding,
    }]))
}

fn package_plan(text: &str, _: Args) -> Result<Vec<Probe>, CheckFinding> {
    Ok(vec![get(package_of(text)?.index_url())])
}

fn package_judge(text: &str, _: Args, outcomes: &[ProbeOutcome]) -> Judged {
    let Ok(coord) = package_of(text) else {
        return Judged::Findings(Vec::new());
    };
    resolved(text, &coord, &outcomes[0])
        .err()
        .unwrap_or(Judged::Findings(Vec::new()))
}

/// A checker that resolves the package in its first round, then asks `second` of the release it resolved to, and
/// judges that answer with `judge`.
fn release_checker(
    c: &mut ContributionsBuilder,
    name: &str,
    shape: impl FnOnce(&mut CheckerBuilder),
    second: fn(&Coord, &str) -> Probe,
    judge: fn(&str, Args, &Coord, &str, &ProbeOutcome) -> Judged,
) {
    let mut codes = ANSWERS.to_vec();
    codes.push(W179);
    rounds_checker(
        c,
        &REGISTRIES,
        name,
        &codes,
        shape,
        package_plan,
        move |text, args, outcomes, carry| {
            let Ok(coord) = package_of(text) else {
                return Judged::Findings(Vec::new());
            };
            match (outcomes, carry.and_then(Value::as_str)) {
                ([index], None) => match resolved(text, &coord, index) {
                    Ok(release) => {
                        let probe = second(&coord, &release.version);
                        Judged::more(vec![probe], json!(release.version))
                    }
                    Err(told) => told,
                },
                // A later round is sent only its own answer: the release rides in the carry.
                ([answer], Some(version)) => judge(text, args, &coord, version, answer),
                _ => Judged::Findings(Vec::new()),
            }
        },
    );
}

fn osv_query(coord: &Coord, version: &str) -> Probe {
    Probe::HttpPostQuery {
        url: osv::QUERY_URL.to_string(),
        json_body: osv::query(coord.ecosystem, &coord.name, version),
    }
}

fn advisory_judge(
    text: &str,
    _: Args,
    coord: &Coord,
    version: &str,
    outcome: &ProbeOutcome,
) -> Judged {
    let gone = || format!("'{text}': OSV answered that its query is gone");
    let body = match answered(text, outcome, &gone) {
        Ok(body) => body,
        Err(told) => return told,
    };
    let advisories = match osv::advisories(body) {
        Ok(advisories) => advisories,
        Err(why) => return vec![unread(text, &why)].into(),
    };
    advisories
        .into_iter()
        .map(|advisory| {
            let aliases = match advisory.aliases.as_slice() {
                [] => String::new(),
                aliases => format!(" ({})", aliases.join(", ")),
            };
            let fixed = advisory.fixed.map_or_else(
                || "no fixed version is named".to_string(),
                |v| format!("fixed in {v}"),
            );
            on(
                W179,
                text,
                format!(
                    "'{text}': {} {version} has the advisory {}{aliases}; {fixed}",
                    coord.name, advisory.id
                ),
            )
        })
        .collect::<Vec<_>>()
        .into()
}

fn release_record(coord: &Coord, version: &str) -> Probe {
    get(coord.release_url(version))
}

fn licence_judge(
    text: &str,
    args: Args,
    coord: &Coord,
    version: &str,
    outcome: &ProbeOutcome,
) -> Judged {
    let gone = || {
        format!(
            "'{text}': the registry has no record of {} {version}",
            coord.name
        )
    };
    let body = match answered(text, outcome, &gone) {
        Ok(body) => body,
        Err(told) => return told,
    };
    let stated = match coord::licence(coord.ecosystem, body) {
        Ok(stated) => stated,
        Err(why) => return vec![unread(text, &why)].into(),
    };
    let unstated = || {
        vec![on(
            W179,
            text,
            format!("'{text}': {} {version} states no SPDX licence", coord.name),
        )]
    };
    let allow = args.words("allow");
    let findings = match stated
        .as_deref()
        .map(|stated| coord::licence_admitted(stated, &allow))
    {
        None | Some(None) => unstated(),
        Some(Some(true)) => Vec::new(),
        Some(Some(false)) => vec![on(
            W179,
            text,
            format!(
                "'{text}': {} {version} is licensed {}, which allow does not admit",
                coord.name,
                stated.unwrap_or_default()
            ),
        )
        .with_help(format!("allow: {}", allow.join(", ")))],
    };
    findings.into()
}

fn doi_judge(text: &str, _: Args, outcomes: &[ProbeOutcome]) -> Judged {
    let Ok(doi) = ids::doi(text) else {
        return Judged::Findings(Vec::new());
    };
    let gone = || format!("'{text}': the DOI '{doi}' is not registered");
    let body = match answered(text, &outcomes[0], &gone) {
        Ok(body) => body,
        Err(told) => return told,
    };
    let findings = match ids::doi_registered(body) {
        Ok(true) => Vec::new(),
        Ok(false) => vec![on(E105, text, gone())],
        Err(why) => vec![unread(text, &why)],
    };
    findings.into()
}

fn orcid_judge(text: &str, _: Args, outcomes: &[ProbeOutcome]) -> Judged {
    let Ok(id) = ids::orcid(text) else {
        return Judged::Findings(Vec::new());
    };
    let gone = || format!("'{text}': ORCID has no record of '{id}'");
    answered(text, &outcomes[0], &gone)
        .err()
        .unwrap_or(Judged::Findings(Vec::new()))
}

pub(super) fn contribute(c: &mut ContributionsBuilder) {
    checker(
        c,
        &REGISTRIES,
        "package",
        &ANSWERS,
        no_args,
        package_plan,
        package_judge,
    );
    release_checker(c, "advisory", no_args, osv_query, advisory_judge);
    release_checker(
        c,
        "license_of",
        |k| {
            k.arg("allow", |f| {
                f.field_type(FieldType::StringList);
            });
        },
        release_record,
        licence_judge,
    );
    checker(
        c,
        &REGISTRIES,
        "doi",
        &ANSWERS,
        no_args,
        |text, _| {
            let doi = ids::doi(text).map_err(|invalid| finding_of(E103, invalid))?;
            Ok(vec![get(ids::doi_url(&doi))])
        },
        doi_judge,
    );
    checker(
        c,
        &REGISTRIES,
        "orcid",
        &ANSWERS,
        no_args,
        |text, _| {
            let id = ids::orcid(text).map_err(|invalid| finding_of(E103, invalid))?;
            Ok(vec![get(ids::orcid_url(&id))])
        },
        orcid_judge,
    );
}
