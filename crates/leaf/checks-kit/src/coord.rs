//! A package as a value names it: `crates:serde@1`, `npm:@scope/name@^18`, `pypi:Requests>=2`. Its ecosystem, its name
//! as the registry keys it, the requirement its versions must meet, and where each lives: the index that lists its
//! releases and the record of one release. And what an index answers: each release, with whether it is yanked.

use std::collections::BTreeMap;
use std::ops::Range;

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::Invalid;
use crate::ranges::Requirement;

/// A package ecosystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ecosystem {
    Crates,
    Npm,
    Pypi,
}

impl Ecosystem {
    /// Every ecosystem.
    const ALL: [Ecosystem; 3] = [Ecosystem::Crates, Ecosystem::Npm, Ecosystem::Pypi];

    /// Its prefix in a value.
    pub fn as_str(self) -> &'static str {
        match self {
            Ecosystem::Crates => "crates",
            Ecosystem::Npm => "npm",
            Ecosystem::Pypi => "pypi",
        }
    }

    /// The ecosystem whose prefix is `prefix`.
    fn of_prefix(prefix: &str) -> Option<Ecosystem> {
        Ecosystem::ALL
            .into_iter()
            .find(|ecosystem| ecosystem.as_str() == prefix)
    }

    /// Its name in OSV's schema.
    pub fn osv(self) -> &'static str {
        match self {
            Ecosystem::Crates => "crates.io",
            Ecosystem::Npm => "npm",
            Ecosystem::Pypi => "PyPI",
        }
    }
}

/// A package and the versions it asks for.
#[derive(Debug, Clone)]
pub struct Coord {
    pub ecosystem: Ecosystem,
    /// As the registry keys it: lowercase on crates.io, as written on npm, PEP 503-normalised on PyPI.
    pub name: String,
    pub requirement: Requirement,
    /// The bytes of the requirement in the value, when one is written.
    pub requirement_at: Option<Range<usize>>,
}

/// What a path segment keeps unencoded.
const SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

impl Coord {
    /// The index that lists its releases: crates.io's sparse index, npm's packument, PyPI's JSON API.
    pub fn index_url(&self) -> String {
        match self.ecosystem {
            Ecosystem::Crates => {
                let name = &self.name;
                let path = match name.len() {
                    1 => format!("1/{name}"),
                    2 => format!("2/{name}"),
                    3 => format!("3/{}/{name}", &name[..1]),
                    _ => format!("{}/{}/{name}", &name[..2], &name[2..4]),
                };
                format!("https://index.crates.io/{path}")
            }
            Ecosystem::Npm => format!("https://registry.npmjs.org/{}", self.npm_path()),
            Ecosystem::Pypi => format!("https://pypi.org/pypi/{}/json", self.name),
        }
    }

    /// The record of its release `version`, which states its licence.
    pub fn release_url(&self, version: &str) -> String {
        let version = utf8_percent_encode(version, SEGMENT);
        match self.ecosystem {
            Ecosystem::Crates => format!("https://crates.io/api/v1/crates/{}/{version}", self.name),
            Ecosystem::Npm => format!("https://registry.npmjs.org/{}/{version}", self.npm_path()),
            Ecosystem::Pypi => format!("https://pypi.org/pypi/{}/{version}/json", self.name),
        }
    }

    /// An npm name as a path segment: a scope's `/` encoded.
    fn npm_path(&self) -> String {
        self.name.replacen('/', "%2f", 1)
    }
}

/// `text` as a package and its requirement.
pub fn parse(text: &str) -> Result<Coord, Invalid> {
    let Some((prefix, rest)) = text.split_once(':') else {
        return Err(Invalid::new(
            "a package is written 'crates:', 'npm:' or 'pypi:' and its name",
        ));
    };
    let start = prefix.len() + 1;
    let Some(ecosystem) = Ecosystem::of_prefix(prefix) else {
        return Err(Invalid::at(
            format!("'{prefix}' is no ecosystem a package is read from: crates, npm or pypi"),
            0..prefix.len(),
        ));
    };
    let (name, written) = match ecosystem {
        Ecosystem::Crates => {
            let (name, at) = split_at_sign(rest, 0);
            (crate_name(text, start..start + name.len())?, at)
        }
        Ecosystem::Npm => {
            let skip = usize::from(rest.starts_with('@'));
            let (name, at) = split_at_sign(rest, skip);
            (npm_name(name, start..start + name.len())?, at)
        }
        Ecosystem::Pypi => {
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || "._-".contains(c)))
                .unwrap_or(rest.len());
            let name = pypi_name(&rest[..end], start..start + end)?;
            let mut after = end;
            if rest[end..].starts_with('[') {
                let close = rest[end..].find(']').ok_or_else(|| {
                    Invalid::at(
                        "the extras are not closed with ']'",
                        start + end..start + rest.len(),
                    )
                })?;
                after = end + close + 1;
            }
            let specifiers = &rest[after..];
            (name, (!specifiers.trim().is_empty()).then_some(after))
        }
    };
    let (requirement, requirement_at) = match written {
        None => (Requirement::Any, None),
        Some(at) => {
            let skip = usize::from(ecosystem != Ecosystem::Pypi);
            let written = &rest[at + skip..];
            let range = start + at + skip..text.len();
            let requirement = Requirement::parse(ecosystem, written).map_err(|why| {
                Invalid::at(
                    format!(
                        "'{written}' is no {} requirement: {why}",
                        ecosystem.as_str()
                    ),
                    range.clone(),
                )
            })?;
            (requirement, Some(range))
        }
    };
    Ok(Coord {
        ecosystem,
        name,
        requirement,
        requirement_at,
    })
}

/// A name and, at the first `@` from `skip`, where its requirement's `@` is.
fn split_at_sign(rest: &str, skip: usize) -> (&str, Option<usize>) {
    match rest[skip..].find('@') {
        Some(at) => (&rest[..skip + at], Some(skip + at)),
        None => (rest, None),
    }
}

/// The crate name at `at` of `text`: ASCII letters, digits, `-` and `_`, starting with a letter, at most 64; keyed
/// lowercase.
fn crate_name(text: &str, at: Range<usize>) -> Result<String, Invalid> {
    let name = &text[at.clone()];
    if name.is_empty() || name.len() > 64 {
        return Err(Invalid::at("a crate name has 1 to 64 characters", at));
    }
    let wrong = |(i, c): &(usize, char)| {
        !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            || (*i == 0 && !c.is_ascii_alphabetic())
    };
    if let Some((i, c)) = name.char_indices().find(wrong) {
        let reason = format!("'{c}' cannot stand there in a crate name");
        return Err(Invalid::char_at(reason, text, at.start + i));
    }
    Ok(name.to_ascii_lowercase())
}

/// An npm name: an optional `@scope/`, then lowercase letters, digits, `-`, `.`, `_` and `~`, not starting with `.`
/// or `_`; at most 214 in all.
fn npm_name(name: &str, at: Range<usize>) -> Result<String, Invalid> {
    let (scope, package) = match name.strip_prefix('@') {
        Some(scoped) => match scoped.split_once('/') {
            Some((scope, package)) => (Some(scope), package),
            None => return Err(Invalid::at("a scoped npm name is '@scope/name'", at)),
        },
        None => (None, name),
    };
    let valid = |part: &str| {
        !part.is_empty()
            && !part.starts_with(['.', '_'])
            && part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "-._~".contains(c))
    };
    if name.len() > 214 || !scope.is_none_or(valid) || !valid(package) {
        return Err(Invalid::at(
            "an npm name is lowercase letters, digits, '-', '.', '_' and '~', not starting with '.' or '_'",
            at,
        ));
    }
    Ok(name.to_string())
}

/// A PyPI name, PEP 508's form, normalised as PEP 503 says: lowercase, each run of `-`, `_` and `.` one `-`.
fn pypi_name(name: &str, at: Range<usize>) -> Result<String, Invalid> {
    let ends = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric());
    if !ends(name.chars().next()) || !ends(name.chars().last()) {
        return Err(Invalid::at(
            "a PyPI name starts and ends with a letter or a digit",
            at,
        ));
    }
    let mut normal = String::with_capacity(name.len());
    for c in name.chars() {
        if "-_.".contains(c) {
            if !normal.ends_with('-') {
                normal.push('-');
            }
        } else {
            normal.push(c.to_ascii_lowercase());
        }
    }
    Ok(normal)
}

/// One release an index lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub yanked: bool,
}

/// The releases `body`, an index's answer in `ecosystem`, lists; why it does not read otherwise. Every field it does not
/// need is skipped unread, so a large packument costs its version keys. The fields it needs are required: an answer
/// without them, or a crates index file with no line, is not one the registry serves, and reads as a transport problem
/// (W172), never as a package whose every release is yanked.
pub fn releases(ecosystem: Ecosystem, body: &[u8]) -> Result<Vec<Release>, String> {
    match ecosystem {
        Ecosystem::Crates => {
            #[derive(Deserialize)]
            struct Line {
                vers: String,
                #[serde(default)]
                yanked: bool,
            }
            body.split(|b| *b == b'\n')
                .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
                .map(|line| {
                    let line: Line = serde_json::from_slice(line).map_err(|e| e.to_string())?;
                    Ok(Release {
                        version: line.vers,
                        yanked: line.yanked,
                    })
                })
                .collect::<Result<Vec<_>, String>>()
                .and_then(|releases| {
                    if releases.is_empty() {
                        Err("the index file lists no release".to_string())
                    } else {
                        Ok(releases)
                    }
                })
        }
        Ecosystem::Npm => {
            #[derive(Deserialize)]
            struct Packument {
                versions: BTreeMap<String, IgnoredAny>,
            }
            let packument: Packument = serde_json::from_slice(body).map_err(|e| e.to_string())?;
            Ok(packument
                .versions
                .into_keys()
                .map(|version| Release {
                    version,
                    yanked: false,
                })
                .collect())
        }
        Ecosystem::Pypi => pypi_releases(body),
    }
}

/// PyPI's JSON API: each release and its files; a release is yanked when it has files and every one is.
fn pypi_releases(body: &[u8]) -> Result<Vec<Release>, String> {
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        yanked: bool,
    }
    #[derive(Deserialize)]
    struct Project {
        releases: BTreeMap<String, Vec<File>>,
    }
    let project: Project = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    Ok(project
        .releases
        .into_iter()
        .map(|(version, files)| Release {
            yanked: !files.is_empty() && files.iter().all(|file| file.yanked),
            version,
        })
        .collect())
}

/// The licence a release record states: crates.io's `version.license`, npm's `license` (a string, or an old object's
/// `type`), PyPI's `license_expression`, else its classifiers' licence, else its `license` when it is one SPDX id.
pub fn licence(ecosystem: Ecosystem, body: &[u8]) -> Result<Option<String>, String> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NpmLicence {
        Text(String),
        Typed {
            #[serde(rename = "type")]
            kind: String,
        },
        Other(IgnoredAny),
    }
    #[derive(Deserialize)]
    struct CratesVersion {
        license: Option<String>,
    }
    #[derive(Deserialize)]
    struct Crates {
        version: CratesVersion,
    }
    #[derive(Deserialize)]
    struct Npm {
        license: Option<NpmLicence>,
    }
    #[derive(Deserialize)]
    struct PypiInfo {
        license_expression: Option<String>,
        license: Option<String>,
        #[serde(default)]
        classifiers: Vec<String>,
    }
    #[derive(Deserialize)]
    struct Pypi {
        info: PypiInfo,
    }
    let read = |e: serde_json::Error| e.to_string();
    let stated = match ecosystem {
        Ecosystem::Crates => {
            serde_json::from_slice::<Crates>(body)
                .map_err(read)?
                .version
                .license
        }
        Ecosystem::Npm => match serde_json::from_slice::<Npm>(body).map_err(read)?.license {
            Some(NpmLicence::Text(text)) => Some(text),
            Some(NpmLicence::Typed { kind }) => Some(kind),
            Some(NpmLicence::Other(_)) | None => None,
        },
        Ecosystem::Pypi => {
            let info = serde_json::from_slice::<Pypi>(body).map_err(read)?.info;
            // The classifiers before the free-text `license`: a classifier names one licence by a fixed list, while
            // `license` often holds a whole licence's text or a name no SPDX id spells.
            info.license_expression
                .or_else(|| info.classifiers.iter().find_map(|c| classifier_licence(c)))
                .or_else(|| {
                    info.license
                        .filter(|l| spdx::license_id(l.trim()).is_some())
                })
        }
    };
    Ok(stated
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty()))
}

/// Whether the SPDX expression `stated` holds with only the licences `allow` names (`MIT OR Apache-2.0` holds with
/// `MIT` alone); `None` when `stated` is no SPDX expression.
pub fn licence_admitted(stated: &str, allow: &[&str]) -> Option<bool> {
    let expression = spdx::Expression::parse(stated).ok()?;
    Some(
        expression.evaluate(|requirement| match &requirement.license {
            spdx::LicenseItem::Spdx { id, .. } => allow.contains(&id.name),
            spdx::LicenseItem::Other { .. } => false,
        }),
    )
}

/// The SPDX id of a `License :: OSI Approved :: …` classifier that names one plainly.
fn classifier_licence(classifier: &str) -> Option<String> {
    let name = classifier.strip_prefix("License :: OSI Approved :: ")?;
    let id = match name {
        "MIT License" => "MIT",
        "Apache Software License" => "Apache-2.0",
        "ISC License (ISCL)" => "ISC",
        "Mozilla Public License 2.0 (MPL 2.0)" => "MPL-2.0",
        "GNU General Public License v3 (GPLv3)" => "GPL-3.0-only",
        "GNU General Public License v2 (GPLv2)" => "GPL-2.0-only",
        "GNU Lesser General Public License v3 (LGPLv3)" => "LGPL-3.0-only",
        "The Unlicense (Unlicense)" => "Unlicense",
        _ => return None,
    };
    Some(id.to_string())
}
