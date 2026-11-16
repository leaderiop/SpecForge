//! The `self-spec-sample` check's verdict (ADR 0360): the WASM-host sample of the self-spec, absorbed into `@gsd/core`
//! kinds, checks with no error and no warning, its work stays within its bound, and its infos, ledger and selection
//! equal their goldens. [`verdict`] and the helpers here are pure: they judge what a run found; [`run`] does the run.

use std::collections::BTreeMap;

use serde_json::{Value, json};

mod steps;
pub use steps::run;

/// What the sample is converted from and judged by: the spec and the manifest it reads, the converter, the `@gsd`
/// workspace it converts into, every crate (the binaries and the cited tests are built from them), the builtins, the
/// build's own configuration, this check and the copy it makes, and its goldens.
pub const TRIGGER: &[&str] = &[
    "spec/**",
    "specforge.json",
    "xtask/Cargo.toml",
    "xtask/src/absorb.rs",
    "xtask/src/absorb/**",
    "gsd/**",
    "crates/**",
    "extensions/**",
    "Cargo.toml",
    "Cargo.lock",
    ".cargo/config.toml",
    ".config/nextest.toml",
    "gate/src/self_spec_sample.rs",
    "gate/src/self_spec_sample/**",
    "gate/src/gsd.rs",
    "gate/src/checked_tree.rs",
    "gate/golden/self-spec-sample/**",
];

/// Where the goldens live, from the repository root.
pub const GOLDENS: &str = "gate/golden/self-spec-sample";

/// The most added or removed lines a golden difference shows of each.
const SHOWN: usize = 20;

/// The work absorb may do on the sample: the old project, the scaffold and the target each compiled once, and one
/// plan of typed edits.
const ABSORB_WORK: (u64, u64) = (3, 1);

/// What a run of the sample found.
#[derive(Debug, Clone, Default)]
pub struct SampleRun {
    /// `specforge check --format json` on the converted project.
    pub diagnostics: Vec<Value>,
    /// absorb's `--selection` file.
    pub selection: String,
    /// absorb's ledger.
    pub ledger: String,
    /// Each absolute path of the run and the placeholder it becomes (`<source>`, `<target>`, `<gsd_core.wasm>`).
    pub paths: Vec<(String, &'static str)>,
    pub work: Work,
}

/// What the run did beyond its output: the bound the check holds it to.
#[derive(Debug, Clone, Default)]
pub struct Work {
    /// The build units the cited tests' run compiled (their fingerprint entries); the gate's test build makes it 0.
    pub compiled: Vec<String>,
    /// Whether the converted project holds a `target` directory (a build that ignored the gate's target directory).
    pub target_dir: bool,
    /// The test ids the run ran, as absorb cites them (`rust:<binary id>#<test name>`).
    pub ran: Vec<String>,
    /// The test ids absorb cited as proofs.
    pub cited: Vec<String>,
    /// absorb's `work` summary: `{compiles, edit_plans}`.
    pub absorb: Value,
}

/// The three goldens, as their files hold them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SampleGoldens {
    pub selection: String,
    pub infos: String,
    pub ledger: String,
}

/// The check's lines, whether it failed and why, and (blessing a clean run) the goldens to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleVerdict {
    pub lines: Vec<String>,
    pub failed: bool,
    /// What failed it, chosen by what it is: the first judged diagnostic, else the first broken bound, else the first
    /// golden that differs. None when it passed.
    pub reason: Option<String>,
    pub bless: Option<SampleGoldens>,
}

/// Judge `run`: an error or a warning that is not the machine's, or a broken work bound, fails it, and is never
/// blessed. Otherwise its infos, selection and ledger are compared with `goldens` (line multisets), or, with
/// `bless`, become the goldens.
pub fn verdict(run: &SampleRun, goldens: &SampleGoldens, bless: bool) -> SampleVerdict {
    let mut lines = Vec::new();
    let place = |text: &str| placeheld(text, &run.paths);
    let severity = |d: &Value| d["severity"].as_str().unwrap_or("").to_string();
    let judged: Vec<&Value> = run
        .diagnostics
        .iter()
        .filter(|d| {
            let severity = severity(d);
            severity == "Error" || (severity == "Warning" && d["machine"] != true)
        })
        .collect();
    let machine: Vec<&Value> = run
        .diagnostics
        .iter()
        .filter(|d| severity(d) == "Warning" && d["machine"] == true)
        .collect();
    let (infos, on_code) = infos(run);
    let info_count = infos.len();
    let infos = render(&infos);
    let errors = judged.iter().filter(|d| severity(d) == "Error").count();
    lines.push(format!(
        "check: {errors} errors, {} warnings, {} infos",
        judged.len() - errors,
        info_count
    ));
    let judged_lines: Vec<String> = judged
        .iter()
        .map(|d| {
            format!(
                "{} {}:{}: {}",
                d["code"].as_str().unwrap_or("?"),
                place(file_of(d)),
                line_of(d),
                place(d["message"].as_str().unwrap_or(""))
            )
        })
        .collect();
    lines.extend(judged_lines.iter().map(|line| format!("  {line}")));
    // After the judged lines: it never fails the check, so it never stands first.
    if !on_code.is_empty() {
        let counts: Vec<String> = on_code
            .iter()
            .map(|(code, n)| format!("{code} {n}"))
            .collect();
        lines.push(format!(
            "  {} infos on the collected code, counted, not golden: {}",
            on_code.values().sum::<usize>(),
            counts.join(", ")
        ));
    }
    if !machine.is_empty() {
        let mut named: BTreeMap<&str, usize> = BTreeMap::new();
        for d in &machine {
            *named.entry(d["code"].as_str().unwrap_or("?")).or_default() += 1;
        }
        let named: Vec<String> = named.iter().map(|(c, n)| format!("{c} {n}")).collect();
        lines.push(format!(
            "  {} machine warnings not judged: {}",
            machine.len(),
            named.join(", ")
        ));
    }
    let bound = bound(&run.work);
    lines.extend(bound.iter().cloned());
    // What fails the run, by what it is: a judged diagnostic, then a broken bound.
    let mut reason = judged_lines
        .first()
        .cloned()
        .or_else(|| bound.first().map(|line| line.trim().to_string()));
    let found = SampleGoldens {
        selection: place(&run.selection),
        infos,
        ledger: place(&run.ledger),
    };
    if bless {
        if reason.is_none() {
            lines.push(format!(
                "blessed: {GOLDENS}/{{selection,infos,ledger}}.json"
            ));
            return SampleVerdict {
                lines,
                failed: false,
                reason: None,
                bless: Some(found),
            };
        }
        lines.push(
            "not blessed: a run with an error, a warning or a broken bound writes no golden".into(),
        );
        return SampleVerdict {
            lines,
            failed: true,
            reason,
            bless: None,
        };
    }
    for (name, golden, now) in [
        ("selection", &goldens.selection, &found.selection),
        ("infos", &goldens.infos, &found.infos),
        ("ledger", &goldens.ledger, &found.ledger),
    ] {
        let difference = difference(golden, now);
        if difference.is_empty() {
            lines.push(format!("{name}: {} lines (golden)", now.lines().count()));
        } else {
            let differs = format!("{name}: differs from {GOLDENS}/{name}.json");
            reason.get_or_insert_with(|| differs.clone());
            lines.push(differs);
            lines.extend(difference);
        }
    }
    SampleVerdict {
        lines,
        failed: reason.is_some(),
        reason,
        bless: None,
    }
}

/// The run's infos (every diagnostic neither an error nor a warning) on the converted project, as
/// `{code, file, message}`, the run's paths held as placeholders, sorted; and the count by code of those placed in the
/// code the collectors read. Those are about the whole workspace, a test anywhere naming an entity outside the sample,
/// so they move with every change to the code, never with the conversion: a golden of them would be blessed on every
/// change and so prove nothing.
fn infos(run: &SampleRun) -> (Vec<Value>, BTreeMap<String, usize>) {
    let mut on_code: BTreeMap<String, usize> = BTreeMap::new();
    let mut infos: Vec<Value> = run
        .diagnostics
        .iter()
        .filter(|d| !matches!(d["severity"].as_str(), Some("Error" | "Warning")))
        .filter(|d| {
            let file = file_of(d);
            let converted =
                file.is_empty() || file == "specforge.json" || file.starts_with("spec/");
            if !converted {
                *on_code
                    .entry(d["code"].as_str().unwrap_or("?").to_string())
                    .or_default() += 1;
            }
            converted
        })
        .map(|d| {
            json!({
                "code": d["code"].as_str().unwrap_or("?"),
                "file": placeheld(file_of(d), &run.paths),
                "message": placeheld(d["message"].as_str().unwrap_or(""), &run.paths),
            })
        })
        .collect();
    infos.sort_by_key(Value::to_string);
    (infos, on_code)
}

/// `values` as a pretty JSON array with a final newline.
fn render(values: &[Value]) -> String {
    let mut text = serde_json::to_string_pretty(values).expect("JSON values serialize");
    text.push('\n');
    text
}

fn file_of(d: &Value) -> &str {
    d["file"]
        .as_str()
        .or_else(|| d["span"]["file"].as_str())
        .unwrap_or("")
}

fn line_of(d: &Value) -> String {
    let line = if d["line"].is_null() {
        &d["span"]["start_line"]
    } else {
        &d["line"]
    };
    line.as_u64().map_or_else(|| "?".into(), |n| n.to_string())
}

/// `text` with each of `paths`' absolute paths replaced by its placeholder, the longest first so a path inside
/// another is never half replaced.
fn placeheld(text: &str, paths: &[(String, &'static str)]) -> String {
    let mut paths: Vec<&(String, &str)> = paths.iter().filter(|(p, _)| !p.is_empty()).collect();
    paths.sort_by_key(|(p, _)| std::cmp::Reverse(p.len()));
    paths
        .iter()
        .fold(text.to_string(), |text, (path, placeholder)| {
            text.replace(path.as_str(), placeholder)
        })
}

/// The lines that break the work bound, empty when it holds.
fn bound(work: &Work) -> Vec<String> {
    let mut broken = Vec::new();
    if !work.compiled.is_empty() {
        broken.push(format!(
            "  bound: {} units compiled; the gate's test build should have held them: {}",
            work.compiled.len(),
            work.compiled.join(", ")
        ));
    }
    if work.target_dir {
        broken.push(
            "  bound: the converted project holds a target directory; its build ignored CARGO_TARGET_DIR".into(),
        );
    }
    let uncited: Vec<&String> = work
        .ran
        .iter()
        .filter(|id| !work.cited.contains(id))
        .collect();
    if !uncited.is_empty() {
        broken.push(format!(
            "  bound: {} tests ran that no proof cites: {}",
            uncited.len(),
            uncited
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let absorb = (
        work.absorb["compiles"].as_u64(),
        work.absorb["edit_plans"].as_u64(),
    );
    if absorb != (Some(ABSORB_WORK.0), Some(ABSORB_WORK.1)) {
        broken.push(format!(
            "  bound: absorb's work is {}, not {{\"compiles\":{},\"edit_plans\":{}}}",
            work.absorb, ABSORB_WORK.0, ABSORB_WORK.1
        ));
    }
    broken
}

/// The lines `now` adds to `golden` (`+`) and removes from it (`-`), as multisets, at most [`SHOWN`] of each, then
/// their counts; empty when they hold the same lines.
fn difference(golden: &str, now: &str) -> Vec<String> {
    let count = |text: &str| {
        let mut counts: BTreeMap<String, i64> = BTreeMap::new();
        for line in text.lines() {
            *counts.entry(line.to_string()).or_default() += 1;
        }
        counts
    };
    let (before, after) = (count(golden), count(now));
    let side = |of: &BTreeMap<String, i64>, other: &BTreeMap<String, i64>| -> Vec<String> {
        of.iter()
            .flat_map(|(line, n)| {
                let extra = n - other.get(line).copied().unwrap_or(0);
                std::iter::repeat_n(line.clone(), usize::try_from(extra).unwrap_or(0))
            })
            .collect()
    };
    let (added, removed) = (side(&after, &before), side(&before, &after));
    if added.is_empty() && removed.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<String> = Vec::new();
    lines.extend(added.iter().take(SHOWN).map(|l| format!("  + {l}")));
    lines.extend(removed.iter().take(SHOWN).map(|l| format!("  - {l}")));
    lines.push(format!(
        "  {} lines added, {} removed",
        added.len(),
        removed.len()
    ));
    lines
}

/// The nextest filterset that runs exactly `cited` (`rust:<binary id>#<test name>`):
/// `(binary_id(=<binary id>) & test(=<test name>)) | …`, or `none()` when none is cited. An id of another form, or
/// one holding a character a filterset would read, is refused.
pub fn filterset(cited: &[String]) -> Result<String, String> {
    if cited.is_empty() {
        return Ok("none()".into());
    }
    let mut terms = Vec::new();
    for id in cited {
        let parts = id
            .strip_prefix("rust:")
            .and_then(|rest| rest.split_once('#'))
            .filter(|(binary, test)| {
                let plain = |s: &str, extra: &[char]| {
                    !s.is_empty()
                        && s.chars().all(|c| {
                            c.is_ascii_alphanumeric() || c == '_' || c == ':' || extra.contains(&c)
                        })
                };
                plain(binary, &['-', '/']) && plain(test, &[])
            });
        let Some((binary, test)) = parts else {
            return Err(format!(
                "the cited test `{id}` is not `rust:<binary id>#<test name>`"
            ));
        };
        terms.push(format!("(binary_id(={binary}) & test(={test}))"));
    }
    Ok(terms.join(" | "))
}

/// The cargo profile `@rust/test`'s `rust-test` capability builds with, from its declaration
/// (`describe_capabilities.json`): the value after `--cargo-profile`, else nextest's default, `test`.
pub fn rust_test_profile(capabilities: &str) -> Result<String, String> {
    let declared: Value = serde_json::from_str(capabilities)
        .map_err(|e| format!("@rust/test's describe_capabilities.json is no JSON: {e}"))?;
    let capability = declared["items"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|c| c["name"] == "rust-test")
        .ok_or("@rust/test's describe_capabilities.json declares no `rust-test` capability")?;
    let argv: Vec<&str> = capability["argv"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let profile = argv
        .iter()
        .position(|a| *a == "--cargo-profile")
        .and_then(|i| argv.get(i + 1))
        .map(|p| p.to_string())
        .or_else(|| {
            argv.iter()
                .find_map(|a| a.strip_prefix("--cargo-profile="))
                .map(String::from)
        });
    Ok(profile.unwrap_or_else(|| "test".into()))
}

/// The directory under the target directory a cargo profile builds into: `dev` and `test` share `debug`, `release`
/// and `bench` share `release`, any other profile has its own.
pub fn profile_dir(profile: &str) -> &str {
    match profile {
        "dev" | "test" => "debug",
        "release" | "bench" => "release",
        other => other,
    }
}

#[cfg(test)]
mod tests;
