//! The run of the `self-spec-sample` check: build what it needs, copy the tracked tree twice, collect the source,
//! absorb the WASM-host sample into the copy without a spec, collect the target and run its cited tests on the gate's
//! own test build, check it, and judge what it found ([`super::verdict`]).
//!
//! Every `specforge` and `absorb` call runs with `HOME` under the run directory (a user's registries, consents and
//! caches never reach it; `CARGO_HOME` and `RUSTUP_HOME` stay the user's, so the toolchain and the registry cache are
//! the gate's), `SPECFORGE_CONSENT_FILE` in a `0700` directory emptied first, the gate's `CARGO_TARGET_DIR`, and no
//! cargo credential variable (ADR 0288): a collector's command is a program the host runs, and keeps the rest of
//! the environment. No bless switch reaches a cited test, and insta writes nothing: a test built under the checkout
//! would write its goldens there. The cited tests run from the gate's own test build in a copy, so the run says so
//! ([`COPY_OF_ENV`]: the test support then reads the checkout they were built under), and their own reports go to
//! the run directory ([`TEST_REPORT_DIR_ENV`]), never into the shared `<target>/specforge` the `tests` check fills,
//! where a report of another nextest run would prune that check's.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use serde_json::Value;

use super::{
    GOLDENS, SampleGoldens, SampleRun, SampleVerdict, Work, filterset, profile_dir,
    rust_test_profile, verdict,
};
use crate::checks::Selection;
use crate::run::Ctx;

/// The sample absorb converts.
const SAMPLE: &str = "wasm-host";

/// The extensions the converted project loads beside `@gsd/core`: the Rust packs that collect it and run its tests.
const EXTENSIONS: [&str; 3] = ["@rust/cargo", "@rust/source", "@rust/test"];

/// `@rust/test`'s committed declaration, which names the cargo profile its run builds with.
const RUST_TEST_DECLARATION: &str =
    "crates/runtime/component/tests/declarations/rust_test/describe_capabilities.json";

/// The variable that says a run is in a copy of the checkout (`specforge_repo_root::COPY_OF_ENV`).
const COPY_OF_ENV: &str = "SPECFORGE_CHECKOUT_COPY_OF";

/// The variable that says where a test binary's own report goes when no collect run names one
/// (`specforge_test::atexit::REPORT_DIR_ENV`).
const TEST_REPORT_DIR_ENV: &str = "SPECFORGE_TEST_REPORT_DIR";

/// The variable a collect run tells a runner integration its report path by (`specforge_test::atexit::REPORT_ENV`).
const REPORT_ENV: &str = "SPECFORGE_REPORT";

/// The variables a cargo credential or a registry's configuration lives in: never passed to a `specforge` call.
const CREDENTIALS: [&str; 4] = [
    "CARGO_REGISTRY_",
    "CARGO_REGISTRIES_",
    "CARGO_CREDENTIAL_",
    "CARGO_HTTP_PROXY",
];

/// The variables that make a test write its goldens or snapshots rather than compare with them (insta's `INSTA_*`, the
/// repository's own `SPECFORGE_BLESS` and `GSD_BLESS`, expect-test's `UPDATE_EXPECT`): never passed to a call, since a
/// cited test built under the checkout would write into the checkout. Insta is then told to write nothing
/// ([`INSTA_UPDATE`]), not even the `.snap.new` its default writes beside a snapshot that differs.
const BLESS_SWITCHES: [&str; 4] = ["SPECFORGE_BLESS", "GSD_BLESS", "UPDATE_EXPECT", "INSTA_"];

/// Insta's write mode, set to `no` on every call.
const INSTA_UPDATE: &str = "INSTA_UPDATE";

pub fn run(ctx: &Ctx, _: &Selection) -> Result<(), String> {
    let wasm = crate::gsd::build_wasm(ctx)?;
    let specforge = ctx.specforge()?.to_path_buf();
    let absorb = ctx.build("xtask", "absorb")?;
    // The repository's test build: fresh after `build`, else compiled here, so the cited tests' run compiles nothing.
    crate::steps::compile_tests(ctx)?;
    let dir = ctx.repo.target_dir.join("gate/self-spec-sample");
    let (source, target) = (dir.join("src"), dir.join("dst"));
    let tracked = crate::steps::tracked_paths(ctx)?;
    crate::checked_tree::copy_tracked(&ctx.repo.root, &tracked, &source)?;
    let without_spec: Vec<String> = tracked
        .iter()
        .filter(|p| !p.starts_with("spec/") && p.as_str() != "specforge.json")
        .cloned()
        .collect();
    crate::checked_tree::copy_tracked(&ctx.repo.root, &without_spec, &target)?;
    let env = Env::prepare(&dir, &ctx.repo.root, &ctx.repo.target_dir)?;
    // Every variable is read as the system holds it: one that is not Unicode passes unread, named in the log.
    for name in not_unicode(std::env::vars_os()) {
        ctx.log(&format!("not Unicode, passed unread: {name}"));
    }
    let call = |program: &Path, args: &[&str]| {
        let mut cmd = ctx.cmd(program);
        env.apply(&mut cmd, std::env::vars_os().map(|(name, _)| name));
        cmd.args(args);
        cmd
    };
    let step = |what: &str, mut cmd: Command| -> Result<String, String> {
        let (ok, out) = ctx.capture(&mut cmd)?;
        if ok {
            Ok(out)
        } else {
            let tail: String = out
                .chars()
                .rev()
                .take(2000)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            Err(format!("{what} failed: {tail}"))
        }
    };
    let collect = |project: &Path, runner: &str, filter: Option<&str>| {
        let project = project.to_string_lossy();
        let mut args = vec!["collect", "--path", &project, "--runner", runner];
        if let Some(filter) = filter {
            args.extend(["--filter", filter]);
        }
        args.extend(["--yes", "--effects", "online", "--format", "json"]);
        step(
            &format!("collect {runner} in {project}"),
            call(&specforge, &args),
        )
    };
    for runner in ["rust-cargo", "rust-source"] {
        collect(&source, runner, None)?;
    }
    let (ledger, selection) = (dir.join("ledger.json"), dir.join("selection.json"));
    let mut args: Vec<String> = [
        "--from",
        &source.to_string_lossy(),
        "--to",
        &target.to_string_lossy(),
        "--only",
        &format!("sample:{SAMPLE}"),
        "--ledger",
        &ledger.to_string_lossy(),
        "--selection",
        &selection.to_string_lossy(),
    ]
    .map(String::from)
    .to_vec();
    for extension in EXTENSIONS {
        args.extend(["--extension".to_string(), extension.to_string()]);
    }
    let mut converting = call(
        &absorb,
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    converting.env("GSD_CORE_WASM", &wasm.core);
    let summary: Value = serde_json::from_str(&step("absorb", converting)?)
        .map_err(|e| format!("absorb printed no JSON summary: {e}"))?;
    let cited: Vec<String> = summary["cited_tests"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(String::from)
        .collect();
    let filter = filterset(&cited)?;
    for runner in ["rust-cargo", "rust-source"] {
        collect(&target, runner, None)?;
    }
    let declaration = read(&ctx.repo.root.join(RUST_TEST_DECLARATION))?;
    let mut metadata = call(
        Path::new("cargo"),
        &["metadata", "--format-version", "1", "--no-deps"],
    );
    // In the copy, as nextest runs there: cargo reads its configuration (`build.build-dir`) from where it runs.
    metadata.current_dir(&target);
    let build_dir = build_directory(&step("cargo metadata", metadata)?)?;
    let profile = build_dir.join(profile_dir(&rust_test_profile(&declaration)?));
    // The positive control: the gate's test build is there, so a unit the run compiles can be seen.
    units(&profile)?;
    let mark = SystemTime::now();
    collect(&target, "rust-test", Some(&filter))?;
    let compiled = built_since(&units(&profile)?, mark);
    let ran = ran(&read(&target.join(".specforge/reports/rust-test.xml"))?);
    let path = target.to_string_lossy();
    let checked = ctx.output(&mut call(
        &specforge,
        &[
            "check",
            "--path",
            &path,
            "--format",
            "json",
            "--effects",
            "cached",
        ],
    ))?;
    let diagnostics = check_diagnostics(
        &checked.status,
        &String::from_utf8_lossy(&checked.stdout),
        &String::from_utf8_lossy(&checked.stderr),
    )?;
    let stats: Value = serde_json::from_str(&step(
        "stats",
        call(&specforge, &["stats", "--path", &path, "--format", "json"]),
    )?)
    .map_err(|e| format!("`specforge stats` printed no JSON: {e}"))?;
    let found = SampleRun {
        diagnostics,
        selection: read(&selection)?,
        ledger: read(&ledger)?,
        paths: vec![
            (source.to_string_lossy().into_owned(), "<source>"),
            (target.to_string_lossy().into_owned(), "<target>"),
            (wasm.core.to_string_lossy().into_owned(), "<gsd_core.wasm>"),
            (dir.to_string_lossy().into_owned(), "<run>"),
        ],
        work: Work {
            target_dir: built_in(&target),
            compiled,
            ran,
            cited,
            absorb: summary["work"].clone(),
        },
    };
    let goldens_dir = ctx.repo.root.join(GOLDENS);
    let golden = |name: &str| {
        std::fs::read_to_string(goldens_dir.join(format!("{name}.json"))).unwrap_or_default()
    };
    let goldens = SampleGoldens {
        selection: golden("selection"),
        infos: golden("infos"),
        ledger: golden("ledger"),
    };
    let bless = std::env::var_os("SPECFORGE_BLESS").is_some_and(|v| !v.is_empty());
    let judged = verdict(&found, &goldens, bless);
    for line in head(&found, &stats) {
        ctx.log(&line);
    }
    for line in &judged.lines {
        ctx.log(line);
    }
    if let Some(blessed) = &judged.bless {
        std::fs::create_dir_all(&goldens_dir)
            .map_err(|e| format!("{}: {e}", goldens_dir.display()))?;
        for (name, text) in [
            ("selection", &blessed.selection),
            ("infos", &blessed.infos),
            ("ledger", &blessed.ledger),
        ] {
            let path = goldens_dir.join(format!("{name}.json"));
            std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    if let Some(reason) = failure(&judged) {
        return Err(reason);
    }
    Ok(())
}

/// `specforge check --format json`'s diagnostics, from its exit `status`, `stdout` and `stderr`. Its JSON must be an
/// array: any other JSON (an error object, a changed format) is an error, never "no diagnostics". A failed exit must
/// be explained by an error among them (`check` exits non-zero on one, which the verdict then judges); a failed exit
/// with none is an error naming the exit and what `check` said on stderr.
fn check_diagnostics(
    status: &std::process::ExitStatus,
    stdout: &str,
    stderr: &str,
) -> Result<Vec<Value>, String> {
    let said = || format!("{status}, stderr: {}", stderr.trim());
    let printed: Value = serde_json::from_str(stdout).map_err(|e| {
        format!(
            "`specforge check` printed no JSON ({e}; {}): {stdout}",
            said()
        )
    })?;
    let Value::Array(diagnostics) = printed else {
        return Err(format!(
            "`specforge check` printed no diagnostic array ({}): {stdout}",
            said()
        ));
    };
    if !status.success() && !diagnostics.iter().any(|d| d["severity"] == "Error") {
        return Err(format!(
            "`specforge check` failed with no error among its diagnostics ({})",
            said()
        ));
    }
    Ok(diagnostics)
}

/// The check's error for a failed verdict: its reason, and where the whole log is.
fn failure(judged: &SampleVerdict) -> Option<String> {
    judged.failed.then(|| {
        let reason = judged
            .reason
            .as_deref()
            .unwrap_or("the sample's verdict failed");
        format!("{reason} (read <target>/gate/self-spec-sample.log)")
    })
}

/// The environment of every `specforge` and `absorb` call of the run.
struct Env {
    home: PathBuf,
    /// The checkout the copies are made from.
    checkout: PathBuf,
    /// Where the cited tests' own reports go.
    reports: PathBuf,
    cargo_home: Option<PathBuf>,
    rustup_home: Option<PathBuf>,
    consent: PathBuf,
    target_dir: PathBuf,
}

/// An I/O error, naming the path it happened at.
fn at(path: &Path) -> impl Fn(std::io::Error) -> String + '_ {
    move |e| format!("{}: {e}", path.display())
}

impl Env {
    /// The run's home (kept between runs: the extension compile cache lives there) and its consent file, removed
    /// first, in a `0700` directory.
    fn prepare(dir: &Path, checkout: &Path, target_dir: &Path) -> Result<Env, String> {
        let (home, consents) = (dir.join("home"), dir.join("consent"));
        std::fs::create_dir_all(&home).map_err(at(&home))?;
        std::fs::create_dir_all(&consents).map_err(at(&consents))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&consents, std::fs::Permissions::from_mode(0o700))
                .map_err(at(&consents))?;
        }
        // The cited tests' own reports of an earlier run are not this one's.
        let reports = dir.join("test-reports");
        match std::fs::remove_dir_all(&reports) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(at(&reports)(e)),
            _ => {}
        }
        let consent = consents.join("consent.json");
        match std::fs::remove_file(&consent) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(at(&consent)(e)),
            _ => {}
        }
        let user = std::env::var_os("HOME").map(PathBuf::from);
        let kept = |name: &str, under: &str| {
            std::env::var_os(name)
                .map(PathBuf::from)
                .or_else(|| user.as_ref().map(|home| home.join(under)))
        };
        Ok(Env {
            home,
            checkout: checkout.to_path_buf(),
            reports,
            cargo_home: kept("CARGO_HOME", ".cargo"),
            rustup_home: kept("RUSTUP_HOME", ".rustup"),
            consent,
            target_dir: target_dir.to_path_buf(),
        })
    }

    /// Set the run's variables on `cmd` and remove each of `inherited` that names a cargo credential or a bless switch
    /// ([`withheld`]); insta writes nothing. The cited tests'
    /// reports are directed by [`TEST_REPORT_DIR_ENV`]: collect owns [`REPORT_ENV`], so the gate passes none of it.
    fn apply(&self, cmd: &mut Command, inherited: impl Iterator<Item = OsString>) {
        cmd.env("HOME", &self.home)
            .env(COPY_OF_ENV, &self.checkout)
            .env_remove(REPORT_ENV)
            .env(TEST_REPORT_DIR_ENV, &self.reports)
            .env("SPECFORGE_CONSENT_FILE", &self.consent)
            .env("CARGO_TARGET_DIR", &self.target_dir);
        if let Some(cargo_home) = &self.cargo_home {
            cmd.env("CARGO_HOME", cargo_home);
        }
        if let Some(rustup_home) = &self.rustup_home {
            cmd.env("RUSTUP_HOME", rustup_home);
        }
        for name in withheld(inherited) {
            cmd.env_remove(name);
        }
        cmd.env(INSTA_UPDATE, "no");
    }
}

/// The names among `names` the calls never receive: a cargo credential or a registry's configuration
/// ([`CREDENTIALS`]) and a test's bless switch ([`BLESS_SWITCHES`]), compared as Unicode (a name that is not is read
/// lossily, so `CARGO_REGISTRY_` before a stray byte still counts), in any case on Windows, where cargo finds
/// `cargo_registry_token` too.
fn withheld(names: impl Iterator<Item = OsString>) -> Vec<OsString> {
    names
        .filter(|name| {
            let name = name.to_string_lossy();
            let name = if cfg!(windows) {
                name.to_ascii_uppercase()
            } else {
                name.into_owned()
            };
            CREDENTIALS
                .iter()
                .chain(&BLESS_SWITCHES)
                .any(|prefix| name.starts_with(prefix))
        })
        .collect()
}

/// The names, read lossily, of the variables among `vars` whose name or value is not Unicode.
fn not_unicode(vars: impl Iterator<Item = (OsString, OsString)>) -> Vec<String> {
    vars.filter(|(name, value)| name.to_str().is_none() || value.to_str().is_none())
        .map(|(name, _)| name.to_string_lossy().into_owned())
        .collect()
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Cargo's build directory, from `cargo metadata`'s JSON: `build_directory` (which `build.build-dir` moves), else
/// `target_directory`, for a cargo that reports none.
fn build_directory(metadata: &str) -> Result<PathBuf, String> {
    let metadata: Value = serde_json::from_str(metadata)
        .map_err(|e| format!("`cargo metadata` printed no JSON: {e}"))?;
    metadata["build_directory"]
        .as_str()
        .or_else(|| metadata["target_directory"].as_str())
        .map(PathBuf::from)
        .ok_or_else(|| "`cargo metadata` names no build or target directory".into())
}

/// Each build unit's fingerprint directory under `profile` (a profile's directory in cargo's build directory), by
/// unit name, sorted: `.fingerprint/<unit>` in the layout every unit shares, `build/<package>/<hash>/fingerprint`
/// (named `<package>-<hash>`) in the one that gives each unit its own directory (cargo 1.100). None found is an
/// error, never "nothing compiled": the bound cannot be judged where cargo keeps no fingerprint.
fn units(profile: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let entries = |dir: &Path| {
        std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| {
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    entry.path(),
                )
            })
            .collect::<Vec<_>>()
    };
    let mut units = entries(&profile.join(".fingerprint"));
    for (package, at) in entries(&profile.join("build")) {
        for (hash, unit) in entries(&at) {
            if unit.join("fingerprint").is_dir() {
                units.push((format!("{package}-{hash}"), unit.join("fingerprint")));
            }
        }
    }
    if units.is_empty() {
        return Err(format!(
            "{}: no build unit's fingerprint (neither `.fingerprint/` nor `build/*/*/fingerprint/`), so the bound \
             of 0 units compiled cannot be judged",
            profile.display()
        ));
    }
    units.sort();
    Ok(units)
}

/// The names of `units` whose fingerprint directory, or a file in it, was written at or after `mark`.
fn built_since(units: &[(String, PathBuf)], mark: SystemTime) -> Vec<String> {
    let newer = |path: &Path| {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .is_ok_and(|t| t >= mark)
    };
    units
        .iter()
        .filter(|(_, dir)| {
            newer(dir)
                || std::fs::read_dir(dir)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .any(|file| newer(&file.path()))
        })
        .map(|(name, _)| name.clone())
        .collect()
}

/// Whether `project` holds a cargo build of its own: a `target` directory with anything but nextest's store
/// (`target/nextest`, which nextest keeps under the workspace root whatever the target directory).
fn built_in(project: &Path) -> bool {
    std::fs::read_dir(project.join("target"))
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| entry.file_name() != "nextest")
}

/// The test ids a nextest JUnit report says ran (a `testcase` with no `skipped`), as absorb cites them:
/// `rust:<classname>#<name>`.
fn ran(junit: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = junit;
    while let Some(start) = rest.find("<testcase ") {
        rest = &rest[start..];
        let open_end = rest.find('>').unwrap_or(rest.len());
        let open = &rest[..open_end];
        let body_end = if open.ends_with('/') {
            open_end
        } else {
            rest.find("</testcase>").unwrap_or(rest.len())
        };
        let body = &rest[..body_end];
        if let (Some(class), Some(name)) = (attribute(open, "classname"), attribute(open, "name"))
            && !body.contains("<skipped")
        {
            ids.push(format!("rust:{class}#{name}"));
        }
        rest = &rest[open_end.min(rest.len())..];
    }
    ids.sort();
    ids
}

/// The unescaped value of `name="…"` in an element's opening tag.
fn attribute(open: &str, name: &str) -> Option<String> {
    let key = format!(" {name}=\"");
    let start = open.find(&key)? + key.len();
    let end = open[start..].find('"')? + start;
    Some(
        open[start..end]
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&"),
    )
}

/// The log's head, the lines a pull request quotes: what was sampled, what the target holds, what ran, the work.
fn head(run: &SampleRun, stats: &Value) -> Vec<String> {
    let selection: std::collections::BTreeMap<String, Vec<String>> =
        serde_json::from_str(&run.selection).unwrap_or_default();
    let sampled: usize = selection.values().map(Vec::len).sum();
    let kinds: Vec<String> = selection
        .iter()
        .map(|(k, ids)| format!("{k} {}", ids.len()))
        .collect();
    let counts: Vec<String> = stats["entity_counts"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| format!("{} {}", c["kind"].as_str().unwrap_or("?"), c["count"]))
        .collect();
    let work = &run.work;
    vec![
        format!(
            "sample {SAMPLE}: {sampled} old entities ({})",
            kinds.join(", ")
        ),
        format!(
            "target: {} entities ({})",
            stats["entity_count"],
            counts.join(", ")
        ),
        format!(
            "tests: {} cited, {} run, {} held as skipped (ignored), {} units compiled",
            work.cited.len(),
            work.ran.len(),
            work.cited.len().saturating_sub(work.ran.len()),
            work.compiled.len()
        ),
        format!(
            "work: absorb compiles {}, edit plans {}; collects 5; checks 1",
            work.absorb["compiles"], work.absorb["edit_plans"]
        ),
    ]
}

#[cfg(test)]
#[path = "steps/tests.rs"]
mod tests;
