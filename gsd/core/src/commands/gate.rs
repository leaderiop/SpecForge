//! `gate_command <build|test>`: the argv the post-merge and regression gates run, in upstream's order per target
//! (`post-merge-gate.md`, `regression-gate-run.md`, gsd-core v1.19.0, MIT, `LICENSE-GSD`). Read-only: it reads the
//! project's markers through declared file reads (ADR 0293) and answers the words, so nothing re-parses a file name.
//!
//! - `[gate-argv]`: an argv, run as `specforge run -- "${ARGV[@]}"`, instead of a string under `bash -c`.
//!   The user's own `workflow.<target>_command` is the one shell string, by its contract ("any shell command").
//! - `[gate-simulator]`: a command runs no process (ADR 0335 D2.2), so the block hands in the text of
//!   `xcodebuild -list -json` and `xcrun simctl list devices available -j`; the simulator is the newest iOS runtime's
//!   first available device.
//! - `[gate-uv-lock]`, `[gate-build-script]`: pytest goes through `uv run` when `uv.lock` exists, instead
//!   of re-running a failed suite under uv; the build's package arm reads `scripts.build`, not a `"build"` anywhere.
//!
//! The package manager is a port of `resolvePackageManagerCommand` (`src/normalize-test-command.cts`, #5346):
//! `package.json`'s `packageManager` field (`name@version`, a known name) wins, then the first lockfile in
//! [`LOCKFILES`]' order, else npm. Only the project root is looked in, as upstream looks only in the gate's directory.
//!
//! Also ports gsd-core bin/lib modules: normalize-test-command.cjs (its package manager; its watch-mode rewrite is
//! not ported, the gates run the command as configured).

use serde_json::json;
use specforge_extension_sdk::host::{FsEntryKind, fs};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, required, run, text};

const MANAGERS: &[&str] = &["npm", "pnpm", "yarn", "bun"];

/// `(read name, lockfile, manager)`, in upstream's order: it only matters when a project carries several.
const LOCKFILES: &[(&str, &str, &str)] = &[
    ("pnpm_lock", "pnpm-lock.yaml", "pnpm"),
    ("yarn_lock", "yarn.lock", "yarn"),
    ("bun_lock", "bun.lock", "bun"),
    ("bun_lockb", "bun.lockb", "bun"),
];

/// `(read name, file, runner)`: the markers a gate only asks the existence of, by the runner each decides.
const MARKERS: &[(&str, &str, Runner)] = &[
    ("justfile_upper", "Justfile", Runner::Just),
    ("justfile_lower", "justfile", Runner::Just),
    ("cargo_toml", "Cargo.toml", Runner::Cargo),
    ("go_mod", "go.mod", Runner::Go),
    ("pyproject", "pyproject.toml", Runner::Python),
    ("requirements", "requirements.txt", Runner::Python),
    ("package_json_file", "package.json", Runner::Package),
];

/// `(read name, file)`: picks how pytest runs, and decides no runner.
const UV_LOCK: (&str, &str) = ("uv_lock", "uv.lock");

/// The answer's `source` when a manifest stands unread: the one answer that fails.
const UNREAD: &str = "unread";

/// The most sources the python build gate compiles.
const PY_COMPILE_MAX: usize = 20;

/// The gate asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Target {
    #[default]
    Build,
    Test,
}

impl Target {
    /// The arg's word; the declaration admits only these two.
    fn of(word: &str) -> Self {
        if word == "build" {
            Target::Build
        } else {
            Target::Test
        }
    }

    fn word(self) -> &'static str {
        match self {
            Target::Build => "build",
            Target::Test => "test",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Runner {
    Make,
    Just,
    Cargo,
    Go,
    Python,
    Package,
}

impl Runner {
    /// The files that decide this runner: one that is there and cannot be read leaves its decision open.
    fn manifests(self) -> Vec<&'static str> {
        let markers = MARKERS
            .iter()
            .filter(|(_, _, runner)| *runner == self)
            .map(|(_, file, _)| *file);
        match self {
            Runner::Make => vec!["Makefile"],
            Runner::Python => markers.chain([UV_LOCK.1]).collect(),
            Runner::Package => markers
                .chain(LOCKFILES.iter().map(|(_, file, _)| *file))
                .collect(),
            _ => markers.collect(),
        }
    }
}

/// Upstream's order per target: the two differ at `package.json`.
const BUILD: &[Runner] = &[
    Runner::Make,
    Runner::Just,
    Runner::Cargo,
    Runner::Go,
    Runner::Python,
    Runner::Package,
];
const TEST: &[Runner] = &[
    Runner::Make,
    Runner::Just,
    Runner::Package,
    Runner::Cargo,
    Runner::Go,
    Runner::Python,
];

/// The manager `package.json`'s text declares, when it is an object whose `packageManager` names a known one.
fn declared(package_json: &str) -> Option<&'static str> {
    let value: serde_json::Value = serde_json::from_str(package_json).ok()?;
    let name = value.get("packageManager")?.as_str()?.split('@').next()?;
    MANAGERS.iter().copied().find(|m| *m == name)
}

fn words(line: &[&str]) -> Vec<String> {
    line.iter().map(ToString::to_string).collect()
}

/// The words that run `script`, from `package.json`'s text and which lockfiles exist. `test` is the manager's short
/// form, except bun's (`bun test` is bun's own runner, not the script).
pub fn resolve(
    script: &str,
    package_json: Option<&str>,
    has: impl Fn(&str) -> bool,
) -> Vec<String> {
    let manager = package_json
        .and_then(declared)
        .or_else(|| LOCKFILES.iter().find(|(_, file, _)| has(file)).map(|l| l.2))
        .unwrap_or("npm");
    if script == "test" && manager != "bun" {
        words(&[manager, "test"])
    } else {
        words(&[manager, "run", script])
    }
}

/// What the caller asks: the target, the scope, the setting and what the block read for it.
#[derive(Default)]
struct Ask<'a> {
    target: Target,
    regression: bool,
    setting: Option<&'a str>,
    xcode_project: Option<&'a str>,
    xcode_list: Option<&'a str>,
    simulators: Option<&'a str>,
    files: Vec<&'a str>,
}

/// What the project holds, as the host read it.
struct Tree<'a, F: Fn(&str) -> bool> {
    has: F,
    makefile: Option<&'a str>,
    package_json: Option<&'a str>,
    python: Vec<&'a str>,
    /// The manifests that are there and did not read: a link out of the project or to nothing, a directory, a file
    /// over the host's text cap.
    unread: Vec<&'a str>,
}

/// The gate's answer.
#[derive(Debug, PartialEq)]
struct Gate {
    source: &'static str,
    argv: Vec<String>,
    note: Option<String>,
}

impl Gate {
    fn runs(source: &'static str, argv: Vec<String>) -> Self {
        Gate {
            source,
            argv,
            note: None,
        }
    }

    fn skips(source: &'static str, note: String) -> Self {
        Gate {
            source,
            argv: Vec::new(),
            note: Some(note),
        }
    }
}

/// The first scheme of `xcodebuild -list -json`'s text: the project's, else the workspace's.
fn first_scheme(list: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(list).ok()?;
    ["project", "workspace"].iter().find_map(|side| {
        let scheme = value.get(side)?.get("schemes")?.get(0)?.as_str()?;
        Some(scheme.to_string())
    })
}

/// The first available device of the newest iOS runtime in the text of `xcrun simctl list devices available -j`. The
/// runtime's version is compared by number: iOS 9 is older than iOS 18.
fn newest_simulator(simulators: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(simulators).ok()?;
    let runtimes = value.get("devices")?.as_object()?;
    let udid = |devices: &serde_json::Value| {
        let device = devices.as_array()?.iter().find(|device| {
            device
                .get("isAvailable")
                .and_then(serde_json::Value::as_bool)
                != Some(false)
        })?;
        device.get("udid")?.as_str().map(str::to_string)
    };
    runtimes
        .iter()
        .filter_map(|(runtime, devices)| {
            let version = runtime.rsplit_once(".iOS-")?.1;
            let version: Vec<u32> = version
                .split('-')
                .map(|n| n.parse().ok())
                .collect::<Option<_>>()?;
            Some((version, udid(devices)?))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, udid)| udid)
}

fn xcode(ask: &Ask<'_>, project: &str) -> Gate {
    let Some(udid) = ask.simulators.and_then(newest_simulator) else {
        let target = ask.target.word();
        return Gate::skips(
            "xcode",
            format!(
                "⚠ No available iOS Simulator found — skipping the Xcode {target} gate. Set workflow.{target}_command to run it anyway."
            ),
        );
    };
    let mut argv = words(&["xcodebuild", ask.target.word(), "-project", project]);
    if let Some(scheme) = ask.xcode_list.and_then(first_scheme) {
        argv.extend(["-scheme".to_string(), scheme]);
    }
    argv.extend(["-destination".to_string(), format!("id={udid}")]);
    Gate::runs("xcode", argv)
}

/// A file named `-x.py` is a source, not an option of the tool that takes it.
fn as_operand(path: &str) -> String {
    if path.starts_with('-') {
        format!("./{path}")
    } else {
        path.to_string()
    }
}

fn python<F: Fn(&str) -> bool>(ask: &Ask<'_>, tree: &Tree<'_, F>) -> Gate {
    if ask.target == Target::Build {
        if tree.python.is_empty() {
            return nothing(ask);
        }
        let mut argv = words(&["python", "-m", "py_compile"]);
        argv.extend(
            tree.python
                .iter()
                .take(PY_COMPILE_MAX)
                .map(|path| as_operand(path)),
        );
        return Gate::runs("python", argv);
    }
    let mut argv = if (tree.has)(UV_LOCK.1) {
        words(&["uv", "run", "python"])
    } else {
        words(&["python"])
    };
    if !ask.regression {
        argv.extend(words(&["-m", "pytest", "-x", "-q", "--tb=short"]));
        return Gate::runs("python", argv);
    }
    if ask.files.is_empty() {
        return Gate::skips(
            "python",
            "⚠ Regression gate: no prior-phase test files were collected — skipping the scoped pytest run instead of falling back to the full suite.".to_string(),
        );
    }
    argv.extend(words(&["-m", "pytest", "-q", "--tb=short", "--"]));
    argv.extend(ask.files.iter().map(ToString::to_string));
    Gate::runs("python", argv)
}

/// The answer when no marker decides: the regression gate says nothing, as upstream's `true` arm.
fn nothing(ask: &Ask<'_>) -> Gate {
    let note = match (ask.regression, ask.target) {
        (true, _) => None,
        (false, Target::Build) => {
            Some("⚠ No build command detected — skipping build gate".to_string())
        }
        (false, Target::Test) => {
            Some("⚠ No test runner detected — skipping post-merge test gate".to_string())
        }
    };
    Gate {
        source: "none",
        argv: Vec::new(),
        note,
    }
}

fn package<F: Fn(&str) -> bool>(ask: &Ask<'_>, tree: &Tree<'_, F>) -> Gate {
    let argv = resolve(ask.target.word(), tree.package_json, &tree.has);
    let mut gate = Gate::runs("package", argv);
    if ask.target == Target::Build {
        gate.note = Some(format!(
            "⚠ Build command not configured — running this project's build script ({}). Set workflow.build_command to pin it.",
            gate.argv.join(" ")
        ));
    }
    gate
}

/// Whether `Makefile`'s text has a rule for `target`, as upstream's `grep -q "^<target>:"`.
///
fn make_has(makefile: Option<&str>, target: &str) -> bool {
    makefile.is_some_and(|text| {
        text.lines().any(|line| {
            line.strip_prefix(target)
                .is_some_and(|rest| rest.starts_with(':'))
        })
    })
}

/// Whether `package.json`'s `scripts.build` is a string.
fn has_build_script(package_json: Option<&str>) -> bool {
    package_json
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .is_some_and(|value| {
            value
                .pointer("/scripts/build")
                .is_some_and(serde_json::Value::is_string)
        })
}

/// The gate `runner` answers for this project, when it decides.
fn decided<F: Fn(&str) -> bool>(runner: Runner, ask: &Ask<'_>, tree: &Tree<'_, F>) -> Option<Gate> {
    let target = ask.target.word();
    let marked = || {
        MARKERS
            .iter()
            .any(|(_, file, of)| *of == runner && (tree.has)(file))
    };
    match runner {
        Runner::Make => {
            make_has(tree.makefile, target).then(|| Gate::runs("make", words(&["make", target])))
        }
        Runner::Just => marked().then(|| Gate::runs("just", words(&["just", target]))),
        Runner::Cargo => marked().then(|| Gate::runs("cargo", words(&["cargo", target]))),
        Runner::Go => marked().then(|| Gate::runs("go", words(&["go", target, "./..."]))),
        Runner::Python => marked().then(|| python(ask, tree)),
        Runner::Package if ask.target == Target::Build => {
            has_build_script(tree.package_json).then(|| package(ask, tree))
        }
        Runner::Package => marked().then(|| package(ask, tree)),
    }
}

fn choose<F: Fn(&str) -> bool>(ask: &Ask<'_>, tree: &Tree<'_, F>) -> Gate {
    if let Some(setting) = ask.setting.filter(|s| !s.trim().is_empty()) {
        return Gate::runs("setting", words(&["bash", "-c", setting]));
    }
    if let Some(project) = ask.xcode_project.filter(|_| !ask.regression) {
        return xcode(ask, project);
    }
    let order = if ask.target == Target::Build {
        BUILD
    } else {
        TEST
    };
    for runner in order {
        if let Some(file) = runner
            .manifests()
            .iter()
            .find(|file| tree.unread.contains(file))
        {
            return Gate::skips(
                UNREAD,
                format!(
                    "⚠ '{file}' is there and could not be read (a link out of the project or to nothing, a directory, a file too large): the {target} gate is not skipped for it. Repair it or set workflow.{target}_command.",
                    target = ask.target.word(),
                ),
            );
        }
        if let Some(gate) = decided(*runner, ask, tree) {
            return gate;
        }
    }
    nothing(ask)
}

/// The argv is read one word a line (`--pick '.argv[]'`): a word no line can carry is refused, never split.
fn refuse_unreadable(argv: &[String]) -> Result<(), String> {
    match argv
        .iter()
        .find(|word| word.is_empty() || word.contains(['\n', '\r']))
    {
        Some(word) => Err(format!(
            "the gate's command holds a word no line can carry ({word:?}): an empty word or a line break"
        )),
        None => Ok(()),
    }
}

/// Whether the host read `file`, or it is not there. A glob's walk answers a link it cannot follow, with no fact of
/// a file (`outside_project`, or not `exists`), and passes a directory over: only a stat sees that.
fn read(call: &CommandCall<'_>, name: &str, file: &str) -> bool {
    match call.files(name).and_then(|read| read.facts.first()) {
        Some(fact) => fact.exists && fact.error.is_none(),
        // A file the glob did not name differs in case from `file` (`justfile`, `Justfile`), which is another name.
        None => match fs::stat("project", file) {
            Ok(None) => true,
            Ok(Some(entry)) => entry.kind == FsEntryKind::File,
            Err(_) => false,
        },
    }
}

fn gate_command(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let target = Target::of(required(call, "target")?);
    let scope = call.str("scope").unwrap_or("post_merge");
    let regression = scope == "regression";
    if regression && target == Target::Build {
        return Err(
            "the regression gate runs tests: 'build' takes no regression scope".to_string(),
        );
    }
    let settings = Settings::current();
    let blank_is_none =
        |arg: &str| Ok::<_, String>(text(call, arg)?.filter(|value| !value.is_empty()));
    let ask = Ask {
        target,
        regression,
        setting: settings.string(&format!("workflow.{}_command", target.word())),
        xcode_project: blank_is_none("xcode_project")?,
        xcode_list: blank_is_none("xcode_list")?,
        simulators: blank_is_none("simulators")?,
        files: blank_is_none("files")?
            .map(|files| files.lines().filter(|line| !line.is_empty()).collect())
            .unwrap_or_default(),
    };
    let read_text = |name: &str| {
        call.files(name)
            .and_then(|f| f.facts.first())
            .and_then(|f| f.text.as_deref())
    };
    let has = |file: &str| {
        LOCKFILES
            .iter()
            .map(|(name, lockfile, _)| (name, lockfile))
            .chain(MARKERS.iter().map(|(name, marker, _)| (name, marker)))
            .chain([(&UV_LOCK.0, &UV_LOCK.1)])
            .any(|(name, marker)| {
                *marker == file
                    && call
                        .files(name)
                        .is_some_and(|f| f.facts.iter().any(|fact| fact.exists))
            })
    };
    let read_names = std::iter::once(("makefile", "Makefile"))
        .chain(std::iter::once(("package_json", "package.json")))
        .chain(LOCKFILES.iter().map(|(name, file, _)| (*name, *file)))
        .chain(MARKERS.iter().map(|(name, file, _)| (*name, *file)))
        .chain([UV_LOCK]);
    let tree = Tree {
        unread: read_names
            .filter(|(name, file)| !read(call, name, file))
            .map(|(_, file)| file)
            .collect(),
        has,
        makefile: read_text("makefile"),
        package_json: read_text("package_json"),
        python: call
            .files("python_sources")
            .map(|f| f.facts.iter().map(|fact| fact.path.as_str()).collect())
            .unwrap_or_default(),
    };
    let gate = choose(&ask, &tree);
    refuse_unreadable(&gate.argv)?;
    // The note goes to stderr, so a block that picks the words on stdout still shows it.
    let human = if gate.argv.is_empty() {
        String::new()
    } else {
        format!("running: {}", gate.argv.join(" "))
    };
    let note = gate.note.clone().unwrap_or_default();
    let mut out = answer(
        call,
        json!({
            "target": target.word(),
            "scope": scope,
            "source": gate.source,
            "argv": gate.argv,
            "note": gate.note,
        }),
        human,
        None,
    );
    out.stderr = note;
    if gate.source == UNREAD {
        out.exit_code = 1;
    }
    Ok(out)
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("gate_command", |cmd| {
        cmd.description("Answer the argv of the build or test gate: the workflow.<target>_command setting, else the project's runner in upstream's order. Read-only; it runs nothing")
            .arg("target", |a| {
                a.required()
                    .one_of(&["build", "test"])
                    .description("the gate: build or test");
            })
            .arg("scope", |a| {
                a.one_of(&["post_merge", "regression"])
                    .default_value("post_merge")
                    .description("post_merge, or regression: no Xcode, and pytest on the named files");
            })
            .arg("xcode_project", |a| {
                a.description("the .xcodeproj the block found; empty is absent");
            })
            .arg("xcode_list", |a| {
                a.description("the text of xcodebuild -list -json");
            })
            .arg("simulators", |a| {
                a.description("the text of xcrun simctl list devices available -j");
            })
            .arg("files", |a| {
                a.description("the regression's test files, one per line");
            })
            .input(Input::files("package_json", FileReadSpec::glob("package.json").text()))
            .input(Input::files("makefile", FileReadSpec::glob("Makefile").text()))
            .input(Input::files("python_sources", FileReadSpec::glob("**/*.py")));
        for (name, file, _) in LOCKFILES {
            cmd.input(Input::files(name, FileReadSpec::glob(file)));
        }
        for (name, file, _) in MARKERS {
            cmd.input(Input::files(name, FileReadSpec::glob(file)));
        }
        cmd.input(Input::files(UV_LOCK.0, FileReadSpec::glob(UV_LOCK.1)));
        cmd.handler(|call| run(call, gate_command));
    });
}

#[cfg(test)]
mod tests;
