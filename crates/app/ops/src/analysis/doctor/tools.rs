//! The programs the collectors detected here run (ADR 0389 D3): `specforge doctor` asks the process port's `which`
//! for each and says, once per program it does not find, which collectors that stops and which codes no collector can
//! then judge, derived from each collector's `answers` and each code's `facts` in the code book. An info note, never
//! a failure; a program that is found says nothing. Nothing is run.

use std::collections::{BTreeMap, BTreeSet};

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
    codes: BTreeSet<String>,
    installs: Vec<String>,
}

/// One info finding per program a collector detected in `view`'s project runs that `process` does not find.
pub(super) fn probe(view: &ProjectView, process: &dyn Process) -> Vec<Finding> {
    let Some(root) = view.root() else {
        return Vec::new();
    };
    let git = view.env().git();
    let codes = view.registries().codes();
    let mut missing: BTreeMap<String, Missing> = BTreeMap::new();
    for collector in detected_here(view.registries().declarations(), root, git.as_deref()) {
        let Some(program) = collector.run.first() else {
            continue;
        };
        if process.which(program).is_some() {
            continue;
        }
        let entry = missing.entry(printable(program).into_owned()).or_default();
        entry.collectors.insert(format!(
            "{} ({})",
            printable(&collector.name),
            printable(&collector.extension)
        ));
        entry.codes.extend(codes.judging(&collector.answers));
        if let Some(line) = collector.install.as_deref().filter(|l| is_install_line(l)) {
            let line = printable(line).into_owned();
            if !entry.installs.contains(&line) {
                entry.installs.push(line);
            }
        }
    }
    missing
        .into_iter()
        .map(|(program, m)| finding(program, m))
        .collect()
}

/// The finding about `program`, missing.
fn finding(program: String, missing: Missing) -> Finding {
    let collectors: Vec<String> = missing.collectors.into_iter().collect();
    let codes: Vec<String> = missing.codes.into_iter().collect();
    let mut check = format!(
        "`{program}` is not found here, so collector {} cannot run",
        collectors.join(", ")
    );
    if !codes.is_empty() {
        check.push_str(&format!("; {} are not judged here", codes.join(", ")));
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
