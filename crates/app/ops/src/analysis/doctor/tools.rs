//! The programs the collectors detected here run (ADR 0389 D3): its argv's first, or the `npx` or `node` script it
//! names, and each program it `requires`. `specforge doctor` asks the process port's `which` for each (an `npx`
//! program also in the `node_modules/.bin` of the root and of each npm workspace package inside the project, never
//! above the root) and says, once per program it does not find, which collectors that stops and which codes no collector can
//! then judge, derived from each collector's `answers` and each code's `facts` in the code book: a code that judges a
//! kind a runnable collector still answers is not named. An info note, never a failure; a program that is found says
//! nothing. Nothing is run.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use specforge_common::remedy::printable;
use specforge_effects::Process;
use specforge_protocol_types::{is_install_line, is_program_name, is_wild};

use super::{About, Finding, FindingStatus};
use crate::Containment;
use crate::analysis::collect::detected_here;
use crate::view::ProjectView;

/// The finding code of a program a detected collector runs that this machine lacks.
const TOOL_MISSING: &str = "tool_missing";

/// What one missing program stops.
#[derive(Default)]
struct Missing {
    collectors: BTreeSet<String>,
    /// The kinds its collectors answer.
    kinds: Vec<String>,
    installs: Vec<String>,
}

/// One info finding per program a collector detected in `view`'s project runs that `process` does not find. The codes
/// it names are the ones that judge a kind its collectors answer and no kind a runnable detected collector answers (one
/// whose program is found, or that runs none): another collector still answering a kind keeps its codes judged.
pub(super) fn probe(view: &ProjectView, process: &dyn Process) -> Vec<Finding> {
    let Some(root) = view.root() else {
        return Vec::new();
    };
    let git = view.env().git();
    let codes = view.registries().codes();
    let lookup = Lookup {
        root,
        containment: view.containment(),
        process,
        bin_dirs: bin_dirs(root, view.containment()),
    };
    let mut missing: BTreeMap<String, Missing> = BTreeMap::new();
    let mut answered: Vec<String> = Vec::new();
    for collector in detected_here(view.registries().declarations(), root, git.as_deref()) {
        let lacking: Vec<String> = lookup
            .program(&collector.run)
            .into_iter()
            .chain(lookup.required(&collector.requires))
            .collect();
        if lacking.is_empty() {
            answered.extend(collector.answers);
            continue;
        }
        for program in lacking {
            let entry = missing.entry(printable(&program).into_owned()).or_default();
            entry.collectors.insert(format!(
                "{} ({})",
                printable(&collector.name),
                printable(&collector.extension)
            ));
            entry.kinds.extend(collector.answers.iter().cloned());
            if let Some(line) = collector.install.as_deref().filter(|l| is_install_line(l)) {
                let line = printable(line).into_owned();
                if !entry.installs.contains(&line) {
                    entry.installs.push(line);
                }
            }
        }
    }
    missing
        .into_iter()
        .map(|(program, m)| {
            let off = codes.switched_off(&m.kinds, &answered);
            finding(program, m, off)
        })
        .collect()
}

/// Where doctor looks a program up: PATH through the process port, and files under the project root, held by its
/// containment (a link out of the project reads as absent). It reads nothing above the root and runs nothing.
struct Lookup<'a> {
    root: &'a Path,
    containment: &'a Containment,
    process: &'a dyn Process,
    /// The project-relative directories whose `node_modules/.bin` an `npx` program may be in ([`bin_dirs`]).
    bin_dirs: Vec<String>,
}

impl Lookup<'_> {
    fn found(&self, program: &str) -> bool {
        self.process.which(program).is_some()
    }

    /// Whether the declared relative path `relative` is a file under the root.
    fn under_root(&self, relative: &str) -> bool {
        crate::containment::declared(Path::new(relative)).is_some_and(|p| {
            self.containment
                .resolve(&self.root.join(p))
                .is_ok_and(|held| held.real.is_file())
        })
    }

    /// The program `run` needs that this machine lacks; none when it has it, or `run` is empty (ADR 0389 D3). `npx
    /// -- <bin>` needs `<bin>`, in a `node_modules/.bin` of [`Lookup::bin_dirs`] or on PATH; `node
    /// node_modules/<package>/...` needs `node` and that script under the root, else names the package; anything else
    /// needs its first item on PATH.
    fn program(&self, run: &[String]) -> Option<String> {
        let first = run.first()?;
        match first.as_str() {
            "npx" => {
                if let Some(bin) = run.iter().skip_while(|arg| *arg != "--").nth(1) {
                    let local = |name: &str| {
                        self.bin_dirs
                            .iter()
                            .any(|dir| self.under_root(&format!("{dir}node_modules/.bin/{name}")))
                    };
                    let present = local(bin) || local(&format!("{bin}.cmd")) || self.found(bin);
                    return (!present).then(|| bin.clone());
                }
            }
            "node" => {
                if !self.found("node") {
                    return Some("node".into());
                }
                let script = run[1..]
                    .iter()
                    .find(|arg| arg.starts_with("node_modules/"))?;
                if self.under_root(script) {
                    return None;
                }
                if crate::containment::declared(Path::new(script)).is_none() {
                    return Some(script.clone());
                }
                // The package directory: `jest`, or a scoped `@acme/tool`.
                let mut segments = script["node_modules/".len()..].split('/');
                let package = match segments.next() {
                    Some(scope) if scope.starts_with('@') => {
                        format!("{scope}/{}", segments.next().unwrap_or_default())
                    }
                    name => name.unwrap_or_default().to_string(),
                };
                return Some(package);
            }
            _ => {}
        }
        (!self.found(first)).then(|| first.clone())
    }

    /// The programs of a collector's `requires` that PATH lacks; a name that is no bare program name is W021's and
    /// never looked up.
    fn required(&self, requires: &[String]) -> Vec<String> {
        requires
            .iter()
            .filter(|program| is_program_name(program) && !self.found(program))
            .cloned()
            .collect()
    }
}

/// The project-relative directories, each ending `/` (the root `""`), whose `node_modules/.bin` may hold an `npx`
/// program: the root, then each npm workspace package directory the root `package.json` names (`workspaces`, a list or
/// `{"packages": [...]}`), inside the project. shortcut: a pattern is a literal directory or one ending `/*` (every
/// directory directly under it); `**` and inner wildcards are not expanded, upgrade when a monorepo needs them.
fn bin_dirs(root: &Path, containment: &Containment) -> Vec<String> {
    let mut dirs = vec![String::new()];
    let Ok((_, bytes)) = containment.read(&root.join("package.json")) else {
        return dirs;
    };
    let Ok(manifest) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return dirs;
    };
    let workspaces = manifest.get("workspaces");
    let patterns = workspaces
        .and_then(serde_json::Value::as_array)
        .or_else(|| workspaces?.get("packages")?.as_array());
    let inside = |dir: &str| {
        !dir.split('/').any(is_wild)
            && crate::containment::declared(Path::new(dir)).is_some()
            && containment
                .resolve(&root.join(dir))
                .is_ok_and(|held| held.real.is_dir())
    };
    for pattern in patterns
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
    {
        let pattern = pattern
            .strip_prefix("./")
            .unwrap_or(pattern)
            .trim_end_matches('/');
        match pattern.strip_suffix("/*") {
            Some(parent) if inside(parent) => {
                let mut names: Vec<String> = std::fs::read_dir(root.join(parent))
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .filter(|name| inside(&format!("{parent}/{name}")))
                    .collect();
                names.sort();
                dirs.extend(names.into_iter().map(|name| format!("{parent}/{name}/")));
            }
            None if inside(pattern) => dirs.push(format!("{pattern}/")),
            _ => {}
        }
    }
    dirs
}

/// The finding about `program`, missing, which switches off `codes`.
fn finding(program: String, missing: Missing, codes: Vec<String>) -> Finding {
    let collectors: Vec<String> = missing.collectors.into_iter().collect();
    let mut check = format!(
        "`{program}` is not found here, so collector {} cannot run",
        collectors.join(", ")
    );
    if !codes.is_empty() {
        check.push_str(&format!(
            "; no collector that runs here answers what {} judge, so they are not judged here",
            codes.join(", ")
        ));
    }
    let remediation = if missing.installs.is_empty() {
        format!("install `{program}` and put it on PATH")
    } else {
        missing.installs.join("; ")
    };
    Finding {
        about: About::Tools {
            program,
            collectors,
            codes,
        },
        check,
        status: FindingStatus::Info,
        code: TOOL_MISSING.into(),
        remediation,
    }
}

#[cfg(test)]
mod tests;
