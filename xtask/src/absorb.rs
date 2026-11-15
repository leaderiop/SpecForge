//! Converts an old project's `@specforge/governance` and `@specforge/product` entities into a fresh `@gsd/core`
//! project by one typed-edit plan (ADR 0360).
//!
//!   GSD_CORE_WASM=<gsd_core.wasm> cargo run -p xtask --bin absorb -- --from <project> --to <dir> [--ledger <file>]
//!
//! The old project is compiled in-process with its own builtins, and must report no error. `--to` is absent or a
//! directory with no project; it is scaffolded with `@specforge/checks`, `@code/core` and the `@gsd/core` component
//! `GSD_CORE_WASM` names, and written by one edit plan, all or nothing. The absorb ledger (default
//! `<to>/absorb-ledger.json`) lists every old value not written as itself. Prints one JSON summary.
//!
//! Exit 0: written. 1: the edit plan was refused (the refusal printed, the ledger written, the scaffold removed).
//! 2: usage, I/O, a source with errors, or a stop before any write (every stop printed).
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
#[path = "absorb/read.rs"]
mod read;
#[path = "absorb/table.rs"]
mod table;
#[path = "absorb/write.rs"]
mod write;

#[cfg(test)]
#[path = "absorb/tests.rs"]
mod tests;

use serde_json::json;
use std::path::{Path, PathBuf};

use write::{Target, Work};

const USAGE: &str = "usage: absorb --from <project> --to <dir> [--ledger <file>]  (GSD_CORE_WASM names gsd_core.wasm)";
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
}

fn parse(args: &[String]) -> Result<Args, String> {
    let (mut from, mut to, mut ledger) = (None, None, None);
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        let value = || format!("{flag} takes a value");
        match flag.as_str() {
            "--from" => from = Some(PathBuf::from(args.next().ok_or_else(value)?)),
            "--to" => to = Some(PathBuf::from(args.next().ok_or_else(value)?)),
            "--ledger" => ledger = Some(PathBuf::from(args.next().ok_or_else(value)?)),
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    Ok(Args {
        from: from.ok_or("--from is required")?,
        to: to.ok_or("--to is required")?,
        ledger,
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
    if let Err(why) = write::check_to(&args.to) {
        return refuse(&why);
    }
    let mut work = Work::default();
    let old = match read::compile(&args.from) {
        Ok(old) => old,
        Err(why) => return refuse(&format!("--from {}: {why}", args.from.display())),
    };
    work.compiles += 1;
    let entities = old.entities();
    if let Err(stops) = plan::prechecks(&entities) {
        let lines: Vec<String> = stops.iter().map(|s| format!("  {s}")).collect();
        return refuse(&format!(
            "{} stop(s) before any write:\n{}",
            stops.len(),
            lines.join("\n")
        ));
    }
    let mut planned = plan::plan(&entities, &old.index());
    let mut target = match Target::create(&args.to, &old.name, &gsd_core, &[], &mut work) {
        Ok(target) => target,
        Err(why) => return refuse(&why),
    };
    let missing = target.missing(&planned.creates);
    if !missing.is_empty() {
        target.roll_back();
        return refuse(&format!(
            "{} required value(s) the old project lacks and no rule fills:\n  {}",
            missing.len(),
            missing.join("\n  ")
        ));
    }
    let ledger_path = args.ledger.unwrap_or_else(|| args.to.join(LEDGER_FILE));
    if let Err(why) = target.apply(&planned.creates, &mut work) {
        target.roll_back();
        eprintln!("absorb: {why}");
        if let Err(io) = write_ledger(&ledger_path, &planned.ledger.to_json(&old.name)) {
            eprintln!("absorb: {io}");
        }
        return 1;
    }
    planned.compare(|id, field| target.computed(id, field));
    if let Err(io) = write_ledger(&ledger_path, &planned.ledger.to_json(&old.name)) {
        return refuse(&io);
    }
    let ([errors, warnings, infos], findings) = target.report();
    let summary = json!({
        "converted": planned.converted,
        "created": planned.created,
        "creates": planned.creates.len(),
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

fn write_ledger(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}
