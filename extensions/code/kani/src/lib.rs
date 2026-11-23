//! `specforge-code-kani`: the reader of Kani's regular output that `@rust/proof` links (ADR 0389 D10).
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. It parses text and
//! depends on nothing; `@rust/proof` names each harness and mints its `test` facts.
//!
//! The format is Kani's `--output-format regular` standard output, as `kani-driver/src/harness_runner.rs` prints it
//! and Kani's own tests expect it: per harness a `Checking harness <name>...` line, its checks, `SUMMARY:` followed by
//! ` ** <failed> of <checks> failed`, `VERIFICATION:- SUCCESSFUL` or `VERIFICATION:- FAILED` and
//! `Verification Time: <x>s`; after a crate's harnesses `Manual Harness Summary:`, a `Verification failed for -
//! <name>` line per failure, then `Complete - <s> successfully verified harnesses, <f> failures, <t> total.`

#[cfg(test)]
mod tests;

/// One `Complete` line's harnesses: one crate's run.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    /// The harnesses in output order.
    pub harnesses: Vec<Harness>,
    /// The `Complete` line's numbers.
    pub successes: usize,
    pub failures: usize,
    pub total: usize,
}

/// One proof harness and Kani's verdict on it.
#[derive(Debug, Clone, PartialEq)]
pub struct Harness {
    /// Kani's name for it, the harness path without its crate (`proofs::check_add`).
    pub name: String,
    pub verdict: Verdict,
    /// From ` ** <failed> of <checks> failed`, when printed.
    pub failed_checks: Option<usize>,
    pub checks: Option<usize>,
    /// From `Verification Time: <x>s`, when printed.
    pub seconds: Option<f64>,
}

/// The `VERIFICATION:-` line's word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Successful,
    Failed,
}

/// A harness whose `VERIFICATION:-` line is not read yet.
struct Open {
    name: String,
    failed_checks: Option<usize>,
    checks: Option<usize>,
}

/// Kani's regular output `text` read into its runs, in output order, in one pass over its lines.
pub fn read(text: &str) -> Result<Vec<Run>, String> {
    let mut runs = Vec::new();
    let mut done: Vec<Harness> = Vec::new();
    let mut open: Option<Open> = None;
    let mut after_summary = false;
    for (index, raw) in text.lines().enumerate() {
        let at = index + 1;
        let line = raw.strip_suffix('\r').unwrap_or(raw).trim();
        if after_summary && !line.is_empty() {
            after_summary = false;
            if let Some(counts) = line.strip_prefix("** ") {
                let (failed, checks) =
                    summary(counts).map_err(|why| format!("line {at}: {why}"))?;
                if let Some(open) = open.as_mut() {
                    open.failed_checks = Some(failed);
                    open.checks = Some(checks);
                }
                continue;
            }
        }
        if let Some(name) = line.strip_prefix("Checking harness ") {
            let name = name.strip_suffix("...").unwrap_or(name);
            open = Some(Open {
                name: name.to_string(),
                failed_checks: None,
                checks: None,
            });
        } else if line == "SUMMARY:" {
            after_summary = true;
        } else if let Some(word) = line.strip_prefix("VERIFICATION:- ") {
            let verdict = match word {
                "SUCCESSFUL" => Verdict::Successful,
                "FAILED" => Verdict::Failed,
                other => return Err(format!("line {at}: '{other}' is no Kani verdict")),
            };
            if let Some(open) = open.take() {
                done.push(Harness {
                    name: open.name,
                    verdict,
                    failed_checks: open.failed_checks,
                    checks: open.checks,
                    seconds: None,
                });
            }
        } else if let Some(time) = line.strip_prefix("Verification Time: ") {
            let seconds = time
                .strip_suffix('s')
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| format!("line {at}: '{time}' is no verification time"))?;
            if let Some(last) = done.last_mut().filter(|h| h.seconds.is_none()) {
                last.seconds = Some(seconds);
            }
        } else if let Some(totals) = line.strip_prefix("Complete - ") {
            let (successes, failures, total) =
                complete(totals).map_err(|why| format!("line {at}: {why}"))?;
            runs.push(Run {
                harnesses: std::mem::take(&mut done),
                successes,
                failures,
                total,
            });
        }
    }
    Ok(runs)
}

/// `<failed> of <checks> failed[ trailing text]`.
fn summary(counts: &str) -> Result<(usize, usize), String> {
    let bad = || format!("'** {counts}' is no '** <failed> of <checks> failed' summary");
    let (failed, rest) = counts.split_once(" of ").ok_or_else(bad)?;
    let (checks, _) = rest.split_once(" failed").ok_or_else(bad)?;
    Ok((
        number(failed).ok_or_else(bad)?,
        number(checks).ok_or_else(bad)?,
    ))
}

/// `<s> successfully verified harnesses, <f> failures, <t> total.`
fn complete(totals: &str) -> Result<(usize, usize, usize), String> {
    let bad = || {
        format!(
            "'Complete - {totals}' is no '<s> successfully verified harnesses, <f> failures, <t> total.' line"
        )
    };
    let mut parts = totals.split(", ");
    let mut part = |suffix: &str| {
        parts
            .next()
            .and_then(|p| p.strip_suffix(suffix))
            .and_then(number)
            .ok_or_else(bad)
    };
    let successes = part(" successfully verified harnesses")?;
    let failures = part(" failures")?;
    let total = part(" total.")?;
    match parts.next() {
        None => Ok((successes, failures, total)),
        Some(_) => Err(bad()),
    }
}

/// A count written in decimal digits only.
fn number(text: &str) -> Option<usize> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}
