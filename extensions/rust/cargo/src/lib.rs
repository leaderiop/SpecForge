//! @rust/cargo — reads every `Cargo.toml` of a project into `@code/core`'s `package` facts (ADR 0348 D5).
//!
//! The collector `rust-cargo` is selected by a `Cargo.toml` at the project root and runs no command: the host serves
//! it every `Cargo.toml` with its text and, without text, the files cargo's target autodiscovery looks at (a guest has
//! no file system, ADR 0293). `specforge-rust-manifest` completes each manifest as cargo does, workspace inheritance
//! and autodiscovery included, and each package becomes one `package` fact minted by `specforge-code-facts`:
//! `pkg:cargo/<name>`, located at its `[package]` table, with its targets and the project packages it depends on.
//!
//! A manifest whose text the host withheld, one cargo's reader cannot read, and a read that stopped early (the host's
//! walk reached its entry cap, or more files matched than it serves) are the collector's error: a package is never
//! dropped silently.
//!
//! A project's `exclude` setting lists directories whose manifests are not its packages (a fixture's or an example's
//! crate): a package in or under one is answered by no fact (ADR 0393 D7).

use serde_json::Value;
use specforge_code_facts::Package;
use specforge_extension_sdk::prelude::*;
use specforge_rust_manifest::read_packages;

const COLLECTOR: &str = "rust-cargo";
/// The read holding every `Cargo.toml` with its text.
const MANIFESTS: &str = "manifests";
/// The reads (name, glob) listing the files cargo's autodiscovery finds targets in, without text: one per glob, as a
/// glob has no alternation.
const TARGETS: [(&str, &str); 11] = [
    ("targets_lib", "**/src/lib.rs"),
    ("targets_main", "**/src/main.rs"),
    ("targets_build", "**/build.rs"),
    ("targets_bin", "**/src/bin/*.rs"),
    ("targets_bin_dir", "**/src/bin/*/main.rs"),
    ("targets_test", "**/tests/*.rs"),
    ("targets_test_dir", "**/tests/*/main.rs"),
    ("targets_example", "**/examples/*.rs"),
    ("targets_example_dir", "**/examples/*/main.rs"),
    ("targets_bench", "**/benches/*.rs"),
    ("targets_bench_dir", "**/benches/*/main.rs"),
];

#[specforge_extension_sdk::extension(
    name = "@rust/cargo",
    version = "1.0.0",
    description = "Reads every Cargo.toml with cargo's own manifest model into package facts (@code/core)"
)]
struct RustCargo;

impl Contributions for RustCargo {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.1");
        c.setting("exclude", |s| {
            s.string_list().default(Vec::<String>::new()).describe(
                "Project-relative directories whose manifests are not packages of this project (a fixture's or an \
                 example's crate): a manifest in or under one is read by no collector",
            );
        });
        c.collector(COLLECTOR, |k| {
            k.detect_files(&["Cargo.toml"])
                .answers(&["package"])
                .input(Input::files(
                    MANIFESTS,
                    FileReadSpec::glob("**/Cargo.toml").text(),
                ));
            for (name, glob) in TARGETS {
                k.input(Input::files(name, FileReadSpec::glob(glob)));
            }
            k.collect(collect);
        });
    }
}

/// The paths and texts of the served read `name`'s files; the collector's error when the read stopped early (the
/// host's walk reached its entry cap, or more files matched than it serves), as a package or a target would be missing.
fn read(input: &CollectInput, name: &str) -> Result<Vec<(String, Option<String>)>, String> {
    let Some(read) = input.files.get(name) else {
        return Ok(Vec::new());
    };
    if read.truncated {
        return Err(format!(
            "the read '{name}' stopped early: the host's walk reached its cap of {GLOB_VISITS_MAX} entries, or more \
             than {FILE_READ_MAX} files matched; rust-cargo would miss packages or targets, so it reads none"
        ));
    }
    read.facts
        .iter()
        .map(|file| match (&file.text, &file.error) {
            (_, Some(error)) => Err(format!(
                "{}: the host did not serve it ({error}); rust-cargo reads every manifest or none",
                file.path
            )),
            (text, None) => Ok((file.path.clone(), text.clone())),
        })
        .collect()
}

/// Every package the served manifests declare outside the project's `exclude` directories, as a `package` fact.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    packages_of(input, &exclude_list(Settings::current().get("exclude"))?)
}

/// The `exclude` setting's directories, normalized to their segments joined by one `/` (`./examples//shop/` is
/// `examples/shop`): the collector's error on an entry that is not a project-relative directory below the root
/// (absolute, a drive's, with a `\` or a `..` segment, or with no segment but `.`).
fn exclude_list(value: Option<&Value>) -> Result<Vec<String>, String> {
    let refuse = |entry: &dyn std::fmt::Display| {
        format!("exclude entry '{entry}' is not a project-relative directory")
    };
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let entries = value.as_array().ok_or_else(|| refuse(value))?;
    entries
        .iter()
        .map(|entry| {
            let text = entry.as_str().ok_or_else(|| refuse(entry))?;
            let drive =
                text.as_bytes().get(1) == Some(&b':') && text.as_bytes()[0].is_ascii_alphabetic();
            let segments: Vec<&str> = text
                .split('/')
                .filter(|s| !s.is_empty() && *s != ".")
                .collect();
            if segments.is_empty()
                || text.starts_with('/')
                || text.contains('\\')
                || drive
                || segments.contains(&"..")
            {
                return Err(refuse(&text));
            }
            Ok(segments.join("/"))
        })
        .collect()
}

/// Whether the package directory `dir` is an entry of `list` or lies under one.
fn excluded(dir: &str, list: &[String]) -> bool {
    list.iter().any(|entry| {
        dir.strip_prefix(entry.as_str())
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    })
}

/// Every package the served manifests declare outside the directories `exclude` lists, as a `package` fact. The
/// filter follows the read, so a workspace's inheritance sees every manifest.
fn packages_of(input: &CollectInput, exclude: &[String]) -> Result<CollectOutput, String> {
    let manifests = read(input, MANIFESTS)?
        .into_iter()
        .map(|(path, text)| match text {
            Some(text) => Ok((path, text)),
            None => Err(format!("{path}: the host served no text for it")),
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut listing = Vec::new();
    for (name, _) in TARGETS {
        listing.extend(read(input, name)?.into_iter().map(|(path, _)| path));
    }
    let packages = read_packages(&manifests, &listing).map_err(|e| e.to_string())?;
    Ok(CollectOutput {
        facts: packages
            .iter()
            .filter(|package| !excluded(&package.path, exclude))
            .map(Package::fact)
            .collect(),
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
