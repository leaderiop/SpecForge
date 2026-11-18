//! The workflows' shape the gate holds: every job's timeout, and its build-time budget under it with the two steps
//! that measure it (ADR 0375 D3); every cache keyed once, under the workflow's one prefix (ADR 0375 D1). A workflow is read line by line, as the gate's other workflow tests read it, with no
//! YAML parser: a job is a line of exactly two spaces' indent ending in `:` under `jobs:`, a step an item `- ` under
//! the job's `steps:`, and a ` #` comment ends a line. What the reader cannot place it refuses rather than skips: a
//! line at a job's indent that is no job header panics. Only the tests read the workflows, so this module is compiled
//! only for them.

/// The name of a job's first step, which records when the job started.
pub const CLOCK: &str = "Start the build-time budget clock";

/// The name of a job's last step, which fails the job when it took longer than its budget.
pub const CHECK: &str = "Build-time budget";

/// What a timeout leaves above a budget: a job that passes its budget fails there, before the timeout cuts it.
pub const MARGIN_SECONDS: u32 = 300;

/// One job of a workflow: its id and every line under it.
pub struct Job<'a> {
    pub id: &'a str,
    lines: Vec<&'a str>,
}

/// One step of a job, its first line the `- ` item.
pub struct Step<'a> {
    lines: Vec<&'a str>,
}

/// The jobs of a workflow's `jobs:` map, in order.
///
/// # Panics
///
/// On a line under `jobs:` at a job's indent (two spaces) that is no job header: a job the reader could not place
/// would escape every check.
pub fn jobs(workflow: &str) -> Vec<Job<'_>> {
    let mut jobs: Vec<Job<'_>> = Vec::new();
    let mut inside = false;
    for line in workflow.lines() {
        if bare(line).is_empty() {
            if let Some(job) = jobs.last_mut().filter(|_| inside) {
                job.lines.push(line);
            }
            continue;
        }
        if indent(line) == 0 {
            inside = bare(line) == "jobs:";
            continue;
        }
        if !inside {
            continue;
        }
        if indent(line) == 2 {
            let id = job_header(line).unwrap_or_else(|| {
                panic!("a line under `jobs:` at a job's indent is no job header: `{line}`")
            });
            jobs.push(Job {
                id,
                lines: Vec::new(),
            });
        } else if let Some(job) = jobs.last_mut() {
            job.lines.push(line);
        }
    }
    jobs
}

/// The id of a line `  <id>:`, two spaces' indent and nothing after the colon but a comment.
fn job_header(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("  ")?;
    if rest.starts_with([' ', '#']) {
        return None;
    }
    bare(rest)
        .strip_suffix(':')
        .filter(|id| !id.is_empty() && !id.contains([' ', ':']))
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// `text` without a trailing YAML comment (a `#` that opens it or follows a space or a tab), what follows it, and the
/// space before it. A value that opens with a quote keeps a `#` inside its quotes.
fn uncommented(text: &str) -> &str {
    let body = text.trim_start();
    let start = text.len() - body.len();
    let from = quoted_len(body);
    let end = body[from..]
        .char_indices()
        .map(|(at, c)| (from + at, c))
        .find(|&(at, c)| c == '#' && (at == 0 || body[..at].ends_with([' ', '\t'])))
        .map_or(body.len(), |(at, _)| at);
    text[..start + end].trim_end()
}

/// The length of the quoted scalar `body` opens with, its quotes included (a `\` escapes in `"`, a `''` is one `'`
/// in `'`); 0 when it opens with no quote, all of it when the quote does not close on this line.
fn quoted_len(body: &str) -> usize {
    let mut chars = body.char_indices().peekable();
    let quote = match chars.next() {
        Some((_, quote @ ('"' | '\''))) => quote,
        _ => return 0,
    };
    while let Some((at, c)) = chars.next() {
        if quote == '"' && c == '\\' {
            chars.next();
        } else if c == quote {
            if quote == '\'' && chars.peek().is_some_and(|&(_, next)| next == '\'') {
                chars.next();
            } else {
                return at + 1;
            }
        }
    }
    body.len()
}

/// `line` without its indent and its comment: what a line that opens a block (`env:`, `steps:`) is compared by.
fn bare(line: &str) -> &str {
    uncommented(line).trim_start()
}

/// The value of `key: value` in `line` once its indent and its comment are gone; `None` when the line holds another
/// key.
fn value_of<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.trim_start().strip_prefix(key)?.strip_prefix(':')?;
    Some(unquote(uncommented(rest).trim()))
}

/// A matrix entry's first line without its indent and its `- `.
fn entry_line(line: &str) -> &str {
    let line = line.trim_start();
    line.strip_prefix("- ").unwrap_or(line)
}

/// A variable of the workflow's own top-level `env`.
pub fn workflow_env<'a>(workflow: &'a str, name: &str) -> Option<&'a str> {
    workflow
        .lines()
        .skip_while(|line| !(indent(line) == 0 && bare(line) == "env:"))
        .skip(1)
        .take_while(|line| bare(line).is_empty() || indent(line) > 0)
        .filter(|line| indent(line) == 2)
        .find_map(|line| value_of(line, name))
}

fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|v| v.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}

impl<'a> Job<'a> {
    /// A key of the job itself (four spaces' indent).
    pub fn key(&self, key: &str) -> Option<&'a str> {
        self.lines
            .iter()
            .filter(|line| indent(line) == 4)
            .find_map(|line| value_of(line, key))
    }

    /// The lines nested under the job's key `key` (a block of four spaces' indent, e.g. `env:` or `steps:`).
    pub fn block(&self, key: &str) -> Vec<&'a str> {
        self.lines
            .iter()
            .skip_while(|line| !(indent(line) == 4 && bare(line) == format!("{key}:")))
            .skip(1)
            .take_while(|line| bare(line).is_empty() || indent(line) > 4)
            .copied()
            .collect()
    }

    /// A variable of the job's own `env`.
    pub fn env(&self, name: &str) -> Option<&'a str> {
        self.block("env")
            .into_iter()
            .filter(|line| indent(line) == 6)
            .find_map(|line| value_of(line, name))
    }

    /// The timeout in minutes, when the job names a whole number.
    pub fn timeout_minutes(&self) -> Option<u32> {
        self.key("timeout-minutes")?.parse().ok()
    }

    /// The entries of the job's `strategy.matrix` written as `- ` items, each its lines.
    fn matrix_entries(&self) -> Vec<Vec<&'a str>> {
        let mut entries: Vec<Vec<&'a str>> = Vec::new();
        let strategy = self.block("strategy");
        let matrix = strategy
            .iter()
            .skip_while(|line| bare(line) != "matrix:")
            .skip(1)
            .take_while(|line| bare(line).is_empty() || indent(line) > 6);
        for line in matrix {
            if line.trim_start().starts_with("- ") {
                entries.push(vec![line]);
            } else if let Some(entry) = entries.last_mut() {
                entry.push(line);
            }
        }
        entries
    }

    /// The value of `key` in each entry of the job's matrix, with the entry's first line to name it by. An error is a
    /// sentence saying which entry lacks the key, or that the matrix has none.
    fn matrix_values(&self, key: &str) -> Result<Vec<(&'a str, &'a str)>, String> {
        let entries = self.matrix_entries();
        if entries.is_empty() {
            return Err(format!("reads its {key} from a matrix with no entries"));
        }
        entries
            .iter()
            .map(|entry| {
                let first = entry_line(entry[0]);
                let at = indent(entry[0]) + 2;
                std::iter::once(first)
                    .chain(entry[1..].iter().filter(|line| indent(line) == at).copied())
                    .find_map(|line| value_of(line, key))
                    .map(|value| (first, value))
                    .ok_or_else(|| format!("has a matrix entry `{first}` with no {key}"))
            })
            .collect()
    }

    /// The job's budgets in seconds: its `BUDGET_SECONDS`, or, when that reads `${{ matrix.budget }}`, each matrix
    /// entry's `budget`. An error is a sentence saying what is missing.
    pub fn budgets(&self) -> Result<Vec<u32>, String> {
        let value = self
            .env("BUDGET_SECONDS")
            .ok_or_else(|| "has no BUDGET_SECONDS in its env".to_string())?;
        if value != "${{ matrix.budget }}" {
            return value
                .parse()
                .map(|budget| vec![budget])
                .map_err(|_| format!("has BUDGET_SECONDS `{value}`, not a whole number"));
        }
        self.matrix_values("budget")?
            .into_iter()
            .map(|(first, budget)| {
                budget.parse().map_err(|_| {
                    format!(
                        "has a matrix entry `{first}` whose budget `{budget}` is not a whole number"
                    )
                })
            })
            .collect()
    }

    /// The job's steps, in order.
    pub fn steps(&self) -> Vec<Step<'a>> {
        let lines = self.block("steps");
        let Some(dash) = lines
            .iter()
            .find(|line| line.trim_start().starts_with("- "))
            .map(|line| indent(line))
        else {
            return Vec::new();
        };
        let mut steps: Vec<Step<'a>> = Vec::new();
        for line in lines {
            if indent(line) == dash && line.trim_start().starts_with("- ") {
                steps.push(Step { lines: vec![line] });
            } else if let Some(step) = steps.last_mut() {
                step.lines.push(line);
            }
        }
        steps
    }

    /// The step named `name`.
    pub fn step(&self, name: &str) -> Option<Step<'a>> {
        self.steps()
            .into_iter()
            .find(|step| step.key("name") == Some(name))
    }
}

impl<'a> Step<'a> {
    /// The indent of the step's own keys: two past its `- `.
    fn key_indent(&self) -> usize {
        indent(self.lines[0]) + 2
    }

    /// A key of the step itself (its `- ` line or a sibling of it), never one nested under `with:`.
    pub fn key(&self, key: &str) -> Option<&'a str> {
        let at = self.key_indent();
        let first = &self.lines[0][at..];
        value_of(first, key).or_else(|| {
            self.lines[1..]
                .iter()
                .filter(|line| indent(line) == at)
                .find_map(|line| value_of(line, key))
        })
    }

    /// The step's own keys, in order, never one nested under another.
    pub fn keys(&self) -> Vec<&'a str> {
        let at = self.key_indent();
        std::iter::once(&self.lines[0][at..])
            .chain(
                self.lines[1..]
                    .iter()
                    .filter(|line| indent(line) == at)
                    .copied(),
            )
            .filter_map(|line| bare(line).split_once(':').map(|(key, _)| key))
            .collect()
    }

    /// An input of the step's `with:`.
    pub fn with(&self, key: &str) -> Option<&'a str> {
        let at = self.key_indent();
        self.lines
            .iter()
            .skip_while(|line| !(indent(line) == at && bare(line) == "with:"))
            .skip(1)
            .take_while(|line| bare(line).is_empty() || indent(line) > at)
            .filter(|line| indent(line) == at + 2)
            .find_map(|line| value_of(line, key))
    }

    /// Whether the step runs `Swatinem/rust-cache`, its owner and repository in any case, as GitHub reads them.
    pub fn is_rust_cache(&self) -> bool {
        const ACTION: &str = "swatinem/rust-cache@";
        self.key("uses").is_some_and(|uses| {
            uses.get(..ACTION.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(ACTION))
        })
    }

    /// The step's `run: |` script, dedented, each line ending in a newline; `None` when it runs no block script.
    pub fn script(&self) -> Option<String> {
        let at = self.key_indent();
        let start = self
            .lines
            .iter()
            .enumerate()
            .find(|(i, line)| {
                let own = if *i == 0 { &line[at..] } else { line };
                (*i == 0 || indent(line) == at) && value_of(own, "run") == Some("|")
            })?
            .0;
        let body: Vec<&str> = self.lines[start + 1..]
            .iter()
            .take_while(|line| line.trim().is_empty() || indent(line) > at)
            .copied()
            .collect();
        let dedent = body
            .iter()
            .find(|line| !line.trim().is_empty())
            .map(|line| indent(line))?;
        let mut script = String::new();
        for line in body {
            script.push_str(line.get(dedent..).unwrap_or(""));
            script.push('\n');
        }
        Some(script.trim_end().to_string() + "\n")
    }
}

/// What is wrong with one job's timeout and budget, each a sentence naming the file and the job: the clock that starts
/// it and the check that ends it are each exactly `name`, `shell: bash` and `run`, and the job has no
/// `continue-on-error` of its own.
pub fn job_findings(file: &str, job: &Job<'_>) -> Vec<String> {
    let mut findings = Vec::new();
    let mut say = |what: String| findings.push(format!("{file}: job `{}` {what}", job.id));
    let timeout = job.timeout_minutes();
    if timeout.is_none() {
        say("has no timeout-minutes".to_string());
    }
    // A job that may fail lets the run pass with the budget check failed, whatever its value (a matrix may set it).
    if job.key("continue-on-error").is_some() {
        say(
            "has a job-level continue-on-error, which lets a job past its budget pass the run"
                .to_string(),
        );
    }
    match job.budgets() {
        Err(missing) => say(missing),
        Ok(budgets) => {
            if let Some(minutes) = timeout {
                let cap = (minutes * 60).saturating_sub(MARGIN_SECONDS);
                for budget in budgets.into_iter().filter(|budget| *budget > cap) {
                    say(format!(
                        "has a budget of {budget} s, over its cap of {cap} s (timeout-minutes {minutes} * 60 - {MARGIN_SECONDS})"
                    ));
                }
            }
        }
    }
    let steps = job.steps();
    for (step, name, place) in [
        (steps.first(), CLOCK, "start"),
        (steps.last(), CHECK, "end"),
    ] {
        let Some(step) = step.filter(|step| step.key("name") == Some(name)) else {
            say(format!("does not {place} with the step `{name}`"));
            continue;
        };
        // An `if`, a `continue-on-error`, an `env` or a `working-directory` could turn the budget off unseen, and a
        // step with no `shell: bash` runs in pwsh on Windows, where the script fails.
        let mut keys = step.keys();
        keys.sort_unstable();
        if keys != ["name", "run", "shell"] || step.key("shell") != Some("bash") {
            say(format!(
                "has the step `{name}` with the keys `{}`, not exactly `name`, `shell: bash` and `run`",
                step.keys().join("`, `")
            ));
        }
    }
    findings
}

/// What is wrong with every job of one workflow (see [`job_findings`]).
pub fn findings(file: &str, workflow: &str) -> Vec<String> {
    jobs(workflow)
        .iter()
        .flat_map(|job| job_findings(file, job))
        .collect()
}

/// The workflow variable every rust-cache step reads as its `prefix-key` (ADR 0375 D1).
pub const PREFIX: &str = "CACHE_PREFIX";

/// One key a rust-cache step names: where it is named (the job, and the matrix entry when the key comes from the
/// matrix) and whether the step saves it.
pub struct CacheKey {
    pub key: String,
    pub at: String,
    pub saves: bool,
}

/// Every key the workflow's rust-cache steps name, and what is wrong with how a step names its key: a step that does
/// not read its `prefix-key` from the workflow's `CACHE_PREFIX`, names a `key` (which rust-cache joins with the job's
/// id), or names no `shared-key`. A `shared-key` of `${{ matrix.<name> }}` is one key per matrix entry; a literal one
/// is one key for the job, whatever its matrix, since rust-cache adds the runner's OS to the key itself.
pub fn cache_keys(file: &str, workflow: &str) -> (Vec<CacheKey>, Vec<String>) {
    let prefix = format!("${{{{ env.{PREFIX} }}}}");
    let mut keys = Vec::new();
    let mut findings = Vec::new();
    for job in jobs(workflow) {
        for step in job.steps().iter().filter(|step| step.is_rust_cache()) {
            let mut say = |what: String| {
                findings.push(format!("{file}: job `{}`'s rust-cache step {what}", job.id))
            };
            if step.with("prefix-key") != Some(prefix.as_str()) {
                say(format!("does not read its prefix-key from `{prefix}`"));
            }
            if step.with("key").is_some() {
                say(
                    "names a `key`, which rust-cache joins with the job's id: name a `shared-key`"
                        .to_string(),
                );
            }
            let Some(shared) = step.with("shared-key") else {
                say("names no `shared-key`".to_string());
                continue;
            };
            let saves = step.with("save-if") != Some("false");
            let matrix = shared
                .strip_prefix("${{ matrix.")
                .and_then(|rest| rest.strip_suffix(" }}"));
            match matrix {
                None if shared.contains("${{") => {
                    say(format!("has a shared-key `{shared}` the gate cannot read"));
                }
                None => keys.push(CacheKey {
                    key: shared.to_string(),
                    at: format!("job `{}`", job.id),
                    saves,
                }),
                Some(name) => match job.matrix_values(name) {
                    Err(missing) => say(missing),
                    Ok(values) => keys.extend(values.into_iter().map(|(entry, key)| CacheKey {
                        key: key.to_string(),
                        at: format!("job `{}` (matrix entry `{entry}`)", job.id),
                        saves,
                    })),
                },
            }
        }
    }
    (keys, findings)
}

/// What is wrong with the workflow's cache keys (ADR 0375 D1): no `CACHE_PREFIX` in its env, a step that names its
/// key otherwise than [`cache_keys`] reads it, a key two steps save (each saver needs a key of its own, or one would
/// save over what the other restored), and a key a step only restores that no step saves.
pub fn key_findings(file: &str, workflow: &str) -> Vec<String> {
    let mut findings = Vec::new();
    if workflow_env(workflow, PREFIX).is_none_or(str::is_empty) {
        findings.push(format!(
            "{file}: the workflow's env has no {PREFIX}, the prefix-key of every cache"
        ));
    }
    let (keys, step_findings) = cache_keys(file, workflow);
    findings.extend(step_findings);
    let savers = |key: &str| -> Vec<&str> {
        keys.iter()
            .filter(|saver| saver.saves && saver.key == key)
            .map(|saver| saver.at.as_str())
            .collect()
    };
    let mut seen: Vec<&str> = Vec::new();
    for key in &keys {
        if seen.contains(&key.key.as_str()) {
            continue;
        }
        seen.push(&key.key);
        let by = savers(&key.key);
        if by.len() > 1 {
            findings.push(format!(
                "{file}: the cache key `{}` is saved by {}: each saver needs a key of its own",
                key.key,
                by.join(" and ")
            ));
        }
    }
    for key in keys.iter().filter(|key| !key.saves) {
        if savers(&key.key).is_empty() {
            findings.push(format!(
                "{file}: {} restores the cache key `{}`, which no step saves",
                key.at, key.key
            ));
        }
    }
    findings
}

mod tests;
