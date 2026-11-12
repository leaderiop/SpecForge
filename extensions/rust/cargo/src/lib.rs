//! @rust/cargo — reads every `Cargo.toml` of a project into `@code/core`'s `package` facts (ADR 0348 D5).
//!
//! The collector `rust-cargo` is selected by a `Cargo.toml` at the project root and runs no command: the host serves
//! it every `Cargo.toml` with its text and, without text, the files cargo's target autodiscovery looks at (a guest has
//! no file system, ADR 0293). `specforge-rust-manifest` completes each manifest as cargo does, workspace inheritance
//! and autodiscovery included, and each package becomes one `package` fact minted by `specforge-code-facts`:
//! `pkg:cargo/<name>`, located at its `[package]` table, with its targets and the project packages it depends on.
//!
//! A manifest whose text the host withheld, one cargo's reader cannot read, and a read that matched more files than
//! the host serves are the collector's error: a package is never dropped silently.

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

/// The paths and texts of the served read `name`'s files; the collector's error when the read matched more files than
/// the host serves, as a package or a target would be missing.
fn read(input: &CollectInput, name: &str) -> Result<Vec<(String, Option<String>)>, String> {
    let Some(read) = input.files.get(name) else {
        return Ok(Vec::new());
    };
    if read.truncated {
        return Err(format!(
            "the read '{name}' matched more files than the host serves ({FILE_READ_MAX}): rust-cargo would miss \
             packages or targets, so it reads none"
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

/// Every package the served manifests declare, as a `package` fact.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
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
        facts: packages.iter().map(Package::fact).collect(),
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
