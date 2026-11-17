//! Converts an old project's `@specforge/governance` and `@specforge/product` entities into a fresh `@gsd/core`
//! project by one typed-edit plan (ADR 0360).
//!
//!   GSD_CORE_WASM=<gsd_core.wasm> cargo run -p xtask --bin absorb -- --from <project> --to <dir>
//!       [--only ids:<file> | --only sample:<name> [--selection <file>]] [--extension <name>]... [--ledger <file>]
//!
//! The old project is compiled in-process with its own builtins, and must report no error; no recorded item is
//! required. `@gsd/core` holds exactly gsd-core's kinds and fields (ADR 0370 D8): a value with no home there is
//! ledgered `dropped`, no home writes a reference, and no kept kind holds a proof, so absorb cites no test (the
//! summary's `cited_tests` is empty). `--only` converts just the old ids a JSON array names, `--only sample:<name>`
//! the ids a sample's selector computes from the old graph (`--selection` writes them, by old kind).
//! `--extension` adds an extension to the target. `--to` is absent or a directory with no project; it is scaffolded with `@specforge/checks`, `@code/core` and the `@gsd/core`
//! component `GSD_CORE_WASM` names, keeping the source's name, version and spec root, and written by one edit plan,
//! all or nothing; a stop leaves it as it was. The absorb ledger (default
//! `<to>/absorb-ledger.json`) lists every old value not written as itself. Prints one JSON summary.
//!
//! Exit 0: written. 1: the edit plan was refused (the refusal printed, the ledger written, the scaffold removed), or
//! the ledger could not be completed once the plan wrote (the target rolled back, the ledger printed to stderr).
//! 2: usage, I/O, a source with errors, or a stop before any write (every stop printed); the ledger is written before
//! the plan applies, so a ledger that cannot be written is such a stop.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a developer tool: its summary is its stdout, its refusals its stderr"
)]

#[path = "absorb/how.rs"]
mod how;
#[path = "absorb/ledger.rs"]
mod ledger;
#[path = "absorb/plan.rs"]
mod plan;
#[path = "absorb/project.rs"]
mod project;
#[path = "absorb/read.rs"]
mod read;
#[path = "absorb/scope.rs"]
mod scope;
#[path = "absorb/select.rs"]
mod select;
#[path = "absorb/table.rs"]
mod table;
#[path = "absorb/write.rs"]
mod write;

#[cfg(test)]
#[path = "absorb/tests.rs"]
mod tests;

use serde_json::json;
use std::path::PathBuf;

use read::Project;
use scope::Scope;
use write::{Target, Work};

const USAGE: &str = "usage: absorb --from <project> --to <dir> [--only ids:<file> | --only sample:<name> \
                     [--selection <file>]] [--extension <name>]... [--ledger <file>]  (GSD_CORE_WASM names \
                     gsd_core.wasm)";
const BUILD_GSD_CORE: &str =
    "build it: (cd gsd && cargo build --locked -p gsd-core --release --target wasm32-wasip2)";
/// The ledger's file in the target when `--ledger` is not given.
const LEDGER_FILE: &str = "absorb-ledger.json";

/// What the command line asks.
#[derive(Debug, Clone, PartialEq)]
struct Args {
    from: PathBuf,
    to: PathBuf,
    ledger: Option<PathBuf>,
    scope: Scope,
    /// The sample `--only sample:<name>` names: its scope is computed once the old project is compiled.
    sample: Option<&'static select::Sample>,
    /// Where `--selection` writes the sample's selection.
    selection: Option<PathBuf>,
    extensions: Vec<String>,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let (mut from, mut to, mut ledger, mut scope) = (None, None, None, Scope::All);
    let (mut sample, mut selection, mut only) = (None, None, None::<String>);
    let mut extensions = Vec::new();
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        let value = || format!("{flag} takes a value");
        match flag.as_str() {
            "--from" => from = Some(PathBuf::from(args.next().ok_or_else(value)?)),
            "--to" => to = Some(PathBuf::from(args.next().ok_or_else(value)?)),
            "--ledger" => ledger = Some(PathBuf::from(args.next().ok_or_else(value)?)),
            "--only" => {
                let given = args.next().ok_or_else(value)?;
                if let Some(first) = &only {
                    return Err(format!(
                        "--only takes one value (ids:<file> or sample:<name>): '{first}' and '{given}' were both given"
                    ));
                }
                match given.strip_prefix("sample:") {
                    Some(name) => sample = Some(select::sample(name)?),
                    None => scope = Scope::from_arg(given)?,
                }
                only = Some(given.clone());
            }
            "--selection" => selection = Some(PathBuf::from(args.next().ok_or_else(value)?)),
            "--extension" => extensions.push(args.next().ok_or_else(value)?.clone()),
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    if selection.is_some() && sample.is_none() {
        return Err(
            "--selection writes a sample's selection: it needs --only sample:<name>".to_string(),
        );
    }
    Ok(Args {
        from: from.ok_or("--from is required")?,
        to: to.ok_or("--to is required")?,
        ledger,
        scope,
        sample,
        selection,
        extensions,
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(run(&args));
}

fn run(args: &[String]) -> i32 {
    let args = match parse(args) {
        Ok(args) => args,
        Err(why) => return refuse(&format!("{why}\n{USAGE}")),
    };
    let gsd_core = match std::env::var_os("GSD_CORE_WASM").map(PathBuf::from) {
        Some(path) if path.is_file() => path,
        Some(path) => {
            return refuse(&format!(
                "GSD_CORE_WASM {} is not a file; {BUILD_GSD_CORE}",
                path.display()
            ));
        }
        None => return refuse(&format!("GSD_CORE_WASM is not set; {BUILD_GSD_CORE}")),
    };
    let source = Project::read(&args.from);
    // The ledger is a path absorb writes like the scaffold's, checked the same way before any write.
    let ledger_path = args
        .ledger
        .clone()
        .unwrap_or_else(|| args.to.join(LEDGER_FILE));
    if let Err(why) = write::check_output("ledger", &ledger_path) {
        return refuse(&why);
    }
    if let Some(path) = &args.selection
        && let Err(why) = write::check_output("selection", path)
    {
        return refuse(&why);
    }
    let mut scaffold = match write::check_to(&args.to, source.spec_root.as_deref()) {
        Ok(scaffold) => scaffold,
        Err(why) => return refuse(&why),
    };
    let mut work = Work::default();
    let old = match read::compile(&args.from) {
        Ok(old) => old,
        Err(why) => return refuse(&format!("--from {}: {why}", args.from.display())),
    };
    work.compiles += 1;
    let scope = match args.sample {
        Some(sample) => {
            let selection = select::select(&old, sample);
            if let Some(path) = &args.selection
                && let Err(why) = write::output("selection", path, &selection.to_json())
            {
                return refuse(&why);
            }
            selection.scope()
        }
        None => args.scope.clone(),
    };
    let entities = match scope.select(old.entities()) {
        Ok(entities) => entities,
        Err(why) => return refuse(&why),
    };
    if let Err(stops) = plan::prechecks(&entities) {
        let lines: Vec<String> = stops.iter().map(|s| format!("  {s}")).collect();
        return refuse(&format!(
            "{} stop(s) before any write:\n{}",
            stops.len(),
            lines.join("\n")
        ));
    }
    let mut planned = plan::plan(&entities);
    if let Err(why) = scaffold.claim(&planned.files()) {
        return refuse(&why);
    }
    let mut target = match Target::create(scaffold, &source, &gsd_core, &args.extensions, &mut work)
    {
        Ok(target) => target,
        Err(why) => return refuse(&why),
    };
    project::ledger(
        &old.project,
        target.project(),
        &old.name,
        &mut planned.ledger,
    );
    let lacked: Vec<String> = target
        .missing(&planned.creates)
        .into_iter()
        .map(|(kind, id, field)| format!("{kind} '{id}' {field}"))
        .collect();
    let refused = target.refused_ids(&planned.creates);
    if !lacked.is_empty() || !refused.is_empty() {
        target.roll_back();
        let mut why = Vec::new();
        if !lacked.is_empty() {
            why.push(format!(
                "{} required value(s) the old project lacks and no rule fills:\n  {}",
                lacked.len(),
                lacked.join("\n  ")
            ));
        }
        if !refused.is_empty() {
            why.push(format!(
                "{} id(s) the target refuses; rename them in the source first:\n  {}",
                refused.len(),
                refused.join("\n  ")
            ));
        }
        return refuse(&why.join("\n"));
    }
    // The ledger as the plan has it, before the plan writes: a place it cannot be written stops the run here, the
    // scaffold rolled back, so the record of what does not convert is never lost behind a written target.
    if let Err(io) = write::output("ledger", &ledger_path, &planned.ledger.to_json(&old.name)) {
        target.roll_back();
        return refuse(&format!("the ledger: {io}; nothing was written"));
    }
    if let Err(why) = target.apply(&planned.creates, &mut work) {
        target.roll_back();
        eprintln!("absorb: {why}");
        // A roll back of a directory the run made removes a ledger written inside it.
        if let Err(io) = write::output("ledger", &ledger_path, &planned.ledger.to_json(&old.name)) {
            eprintln!("absorb: the ledger: {io}");
        }
        return 1;
    }
    planned.compare(|id, field| target.computed(id, field));
    let ledger = planned.ledger.to_json(&old.name);
    if let Err(io) = write::output("ledger", &ledger_path, &ledger) {
        // Written once already, the ledger cannot be completed: the target is rolled back, so a rerun converts it
        // again, and the whole ledger goes to stderr, never lost.
        target.roll_back();
        eprintln!("absorb: the ledger: {io}; the target is rolled back, and the ledger follows");
        eprint!("{ledger}");
        return 1;
    }
    let ([errors, warnings, infos], findings) = target.report();
    let summary = json!({
        "converted": planned.converted,
        "created": planned.created,
        "creates": planned.creates.len(),
        "cited_tests": [],
        "ledger": planned.ledger.counts(),
        "work": work,
        "target": {"errors": errors, "warnings": warnings, "infos": infos, "findings": findings},
    });
    println!("{summary}");
    0
}

fn refuse(why: &str) -> i32 {
    eprintln!("absorb: {why}");
    2
}
