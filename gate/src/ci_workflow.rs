//! The workflows' shape the gate holds: every job's timeout, and its build-time budget under it with the two steps
//! that measure it (ADR 0375 D3). A workflow is read line by line, as the gate's other workflow tests read it, with no
//! YAML parser: a job is a line of exactly two spaces' indent ending in `:` under `jobs:`, a step an item `- ` under
//! the job's `steps:`. Only the tests read the workflows, so this module is compiled only for them.

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
pub fn jobs(workflow: &str) -> Vec<Job<'_>> {
    let mut jobs: Vec<Job<'_>> = Vec::new();
    let mut inside = false;
    for line in workflow.lines() {
        if indent(line) == 0 && !line.trim().is_empty() && !line.starts_with('#') {
            inside = line == "jobs:";
            continue;
        }
        if !inside {
            continue;
        }
        match job_header(line) {
            Some(id) => jobs.push(Job {
                id,
                lines: Vec::new(),
            }),
            None => {
                if let Some(job) = jobs.last_mut() {
                    job.lines.push(line);
                }
            }
        }
    }
    jobs
}

/// The id of a line `  <id>:`, two spaces' indent and nothing after the colon.
fn job_header(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("  ")?;
    if rest.starts_with([' ', '#']) {
        return None;
    }
    rest.strip_suffix(':')
        .filter(|id| !id.is_empty() && !id.contains([' ', ':']))
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The value of `key: value` in `line` once its indent is gone; `None` when the line holds another key.
fn value_of<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.trim_start().strip_prefix(key)?.strip_prefix(':')?;
    Some(unquote(rest.trim()))
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
            .skip_while(|line| !(indent(line) == 4 && line.trim() == format!("{key}:")))
            .skip(1)
            .take_while(|line| line.trim().is_empty() || indent(line) > 4)
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
            .skip_while(|line| line.trim() != "matrix:")
            .skip(1)
            .take_while(|line| line.trim().is_empty() || indent(line) > 6);
        for line in matrix {
            if line.trim_start().starts_with("- ") {
                entries.push(vec![line]);
            } else if let Some(entry) = entries.last_mut() {
                entry.push(line);
            }
        }
        entries
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
        let entries = self.matrix_entries();
        if entries.is_empty() {
            return Err("reads its budget from a matrix with no entries".to_string());
        }
        entries
            .iter()
            .map(|entry| {
                let first = entry[0].trim_start().trim_start_matches("- ");
                let budget = entry
                    .iter()
                    .map(|line| line.trim_start().trim_start_matches("- "))
                    .find_map(|line| value_of(line, "budget"))
                    .ok_or_else(|| format!("has a matrix entry `{first}` with no budget"))?;
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

    /// An input of the step's `with:`.
    pub fn with(&self, key: &str) -> Option<&'a str> {
        let at = self.key_indent();
        self.lines
            .iter()
            .skip_while(|line| !(indent(line) == at && line.trim() == "with:"))
            .skip(1)
            .take_while(|line| line.trim().is_empty() || indent(line) > at)
            .filter(|line| indent(line) == at + 2)
            .find_map(|line| value_of(line, key))
    }

    /// Whether the step runs `Swatinem/rust-cache`.
    pub fn is_rust_cache(&self) -> bool {
        self.key("uses")
            .is_some_and(|uses| uses.starts_with("Swatinem/rust-cache@"))
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

/// What is wrong with one job's timeout and budget, each a sentence naming the file and the job.
pub fn job_findings(file: &str, job: &Job<'_>) -> Vec<String> {
    let mut findings = Vec::new();
    let mut say = |what: String| findings.push(format!("{file}: job `{}` {what}", job.id));
    let timeout = job.timeout_minutes();
    if timeout.is_none() {
        say("has no timeout-minutes".to_string());
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
    let names: Vec<Option<&str>> = job.steps().iter().map(|step| step.key("name")).collect();
    if names.first().copied().flatten() != Some(CLOCK) {
        say(format!("does not start with the step `{CLOCK}`"));
    }
    if names.last().copied().flatten() != Some(CHECK) {
        say(format!("does not end with the step `{CHECK}`"));
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

mod tests;
