//! The programs the collectors detected here run (ADR 0389 D3): `specforge doctor` asks the process port's `which`
//! for each and says, once per program it does not find, which collectors that stops and which codes no collector can
//! then judge, derived from each collector's `answers` and each code's `facts` in the code book: a code that judges a
//! kind a runnable collector still answers is not named. An info note, never a failure; a program that is found says
//! nothing. Nothing is run.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use specforge_common::remedy::printable;
use specforge_effects::Process;
use specforge_protocol_types::is_install_line;

use super::{About, Finding, FindingStatus};
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
    let mut missing: BTreeMap<String, Missing> = BTreeMap::new();
    let mut answered: Vec<String> = Vec::new();
    for collector in detected_here(view.registries().declarations(), root, git.as_deref()) {
        let Some(program) = self::program(&collector.run, root, process) else {
            answered.extend(collector.answers);
            continue;
        };
        let entry = missing.entry(printable(&program).into_owned()).or_default();
        entry.collectors.insert(format!(
            "{} ({})",
            printable(&collector.name),
            printable(&collector.extension)
        ));
        entry.kinds.extend(collector.answers);
        if let Some(line) = collector.install.as_deref().filter(|l| is_install_line(l)) {
            let line = printable(line).into_owned();
            if !entry.installs.contains(&line) {
                entry.installs.push(line);
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

/// The program `run` needs that this machine lacks; none when it has it, or `run` is empty (ADR 0389 D3). `npx --
/// <bin>` needs `<bin>`, in the project's `node_modules/.bin` or on PATH; `node node_modules/<package>/...` needs
/// `node` and that script under the root, else names the package; anything else needs its first item on PATH. A
/// file check is `is_file` on a declared relative path joined under `root`: no read, no run.
fn program(run: &[String], root: &Path, process: &dyn Process) -> Option<String> {
    let first = run.first()?;
    let found = |program: &str| process.which(program).is_some();
    let under_root = |relative: &str| {
        crate::containment::declared(Path::new(relative)).is_some_and(|p| root.join(p).is_file())
    };
    match first.as_str() {
        "npx" => {
            if let Some(bin) = run.iter().skip_while(|arg| *arg != "--").nth(1) {
                let local = |name: String| under_root(&format!("node_modules/.bin/{name}"));
                let present = local(bin.clone()) || local(format!("{bin}.cmd")) || found(bin);
                return (!present).then(|| bin.clone());
            }
        }
        "node" => {
            if !found("node") {
                return Some("node".into());
            }
            let script = run[1..]
                .iter()
                .find(|arg| arg.starts_with("node_modules/"))?;
            if under_root(script) {
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
    (!found(first)).then(|| first.clone())
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
