//! `--pick <path>`: the values at a path of a JSON answer, one per line (ADR 0325), so a shell reads one value of
//! `specforge query` or of an extension command without jq.
//!
//! The path is steps: `.key`, `[n]`, `[]` (every item) and `[<key>=<value>]` (the items whose field `<key>`, a dotted
//! path such as `fields.result`, is `<value>`: a string by its text, another scalar as JSON). A string prints
//! bare, a number or boolean as JSON, an object or array as compact JSON; `null` and a missing value
//! print nothing, and are not an error.

use crate::outcome::{Exit, OutputFormat};
use crate::terminal::{Terminal, errln, out};
use serde_json::Value;

/// The rules every surface's long help states.
macro_rules! rules {
    () => {
        "The path is steps: `.key`, `[n]` (digits, no leading zero), `[]` (every item) and `[<key>=<value>]` (the \
         items whose field <key> is <value>, e.g. `[kind=plan]`: <key> may be dotted, `[fields.result=issue]`, and a \
         number, boolean or null is compared as JSON, `[fields.backlog=true]`, `[fields.wave=2]`; an object \
         or array is never selected); a `.` before `[` may be left out, and `.` \
         alone is the whole answer. One line per value reached: a string bare (a newline in it is printed as is), an \
         integer as is and any other number as an f64 (`1.0`, `-0.0`, `1e3` as `1000.0`), a boolean, object or array \
         as compact JSON; null or a missing value prints nothing, and is not an error."
    };
}

/// The short help of `--fields`, beside `--pick` on `query` and `export`.
pub(crate) const FIELDS_HELP: &str =
    "With --pick: one row per item reached, a cell per key, the cells separated by 0x1f";

/// The long help of `--fields`.
pub(crate) const FIELDS_LONG_HELP: &str = "With --pick: print one row per item the path reaches, one cell per \
     comma-separated key, in order, the cells separated by the ASCII unit separator (0x1f), e.g. --pick \
     '.nodes[kind=commit]' --fields fields.hash,fields.subject, read with IFS=$'\\x1f' read -r hash subject. A key may \
     be dotted. A cell prints as --pick prints a value, with a missing value or null as an empty cell; 0x1f is not IFS \
     whitespace, so a read keeps an empty cell and the rows never misalign. A backslash, newline, carriage return or \
     0x1f in a cell prints as \\\\, \\n, \\r or \\x1f.";

/// `--pick`'s id: the shapes and `--fields` require it.
pub(crate) const PICK: &str = "pick";

/// The short help of `--pick`, on every command that takes it.
pub(crate) const HELP: &str = "Print only the values at this path of the JSON answer, one per line";

/// `query --pick`'s long help.
pub(crate) const QUERY_LONG_HELP: &str = concat!(
    "Print only the values at this path of the graph format's JSON answer, one per line, e.g. \
     '.nodes[kind=plan].id'. ",
    rules!(),
    " It takes --format graph or json, and neither --inputs nor --include-coverage."
);

/// An extension command's `--pick` long help.
pub(crate) const COMMAND_LONG_HELP: &str = concat!(
    "Print only the values at this path of the command's own JSON answer, one per line, e.g. '.next_command'. ",
    rules!(),
    " It implies --format json (another --format is refused): errors print as they do under json, and the exit code \
     is the command's, or 1 when it passed but its answer is not JSON. A writing command is picked from its own \
     answer, never from the plan report --format json wraps it in."
);

/// `export --pick`'s long help.
pub(crate) const EXPORT_LONG_HELP: &str = concat!(
    "Print only the values at this path of the graph format's JSON answer, one per line, e.g. \
     '.nodes[kind=phase].id'. ",
    rules!(),
    " It takes --format graph or json."
);

/// The long help of `--pick` on a command whose JSON is its answer (`check`, `settings`).
pub(crate) const ANSWER_LONG_HELP: &str = concat!(
    "Print only the values at this path of the command's JSON answer, one per line, e.g. '.[code=E003].message' \
     on check or '.extensions[].settings[name=<setting>].value' on settings. ",
    rules!(),
    " It implies --format json (--format human is refused): errors print as they do under json, and the exit code \
     is the command's."
);

/// The long help of `--pick` on a verb (`files`, `http`, `git`), whose answer is JSON by default.
pub(crate) const VERB_LONG_HELP: &str = concat!(
    "Print only the values at this path of the verb's JSON answer, one per line, e.g. '.entries[].path' on files \
     list or '.branch' on git head. ",
    rules!(),
    " It takes no --format human: errors print as they do under json, and the exit code is the verb's."
);

/// `--format human|json` beside `--pick`, for a core command whose JSON form is its answer (`check`, `settings`):
/// with `--pick` the format is `json` unless one is given, so an explicit `--format human` can be refused.
#[derive(clap::Args, Clone, Debug)]
#[command(mut_arg(PICK, |arg| arg.long_help(ANSWER_LONG_HELP)))]
pub(crate) struct PickedOutputArg {
    /// Output format: human or json (json with --pick)
    #[arg(
        long,
        default_value = "human",
        default_value_if(PICK, clap::builder::ArgPredicate::IsPresent, "json")
    )]
    format: OutputFormat,

    #[command(flatten)]
    picked: Picked,
}

impl PickedOutputArg {
    /// The format, or the usage error (exit 2) of `--pick` beside `--format human`.
    pub(crate) fn format(&self, terminal: &Terminal) -> Result<OutputFormat, Exit> {
        // a usage error, not a note: written under either format
        match (self.picked.picks(), self.format) {
            (true, OutputFormat::Human) => {
                errln!(
                    terminal,
                    "error: --pick implies --format json: it takes no --format human"
                );
                Err(Exit::Unjudged)
            }
            (_, format) => Ok(format),
        }
    }

    pub(crate) fn picked(&self) -> &Picked {
        &self.picked
    }
}

/// What a command reads beside `--pick`'s path: the `--fields` rows of `query` and `export` ([`FieldRows`], ADR 0325
/// D7), or none ([`NoRows`]).
pub(crate) trait Rows: clap::Args + clap::FromArgMatches {
    /// The `--fields` keys given.
    fn fields(&self) -> Option<&Fields>;
}

// No `--fields`: a command whose items are not entities. (A `///` here would be the about of the command it is
// flattened in.)
#[derive(clap::Args, Clone, Debug, Default)]
#[group(skip)]
pub(crate) struct NoRows {}

impl Rows for NoRows {
    fn fields(&self) -> Option<&Fields> {
        None
    }
}

// `--fields`, on `query` and `export`.
#[derive(clap::Args, Clone, Debug, Default)]
#[group(skip)]
pub(crate) struct FieldRows {
    #[arg(long, value_name = "KEYS", requires = PICK, conflicts_with_all = [COUNT, JOIN], value_parser = Fields::parse, help = FIELDS_HELP, long_help = FIELDS_LONG_HELP)]
    fields: Option<Fields>,
}

impl Rows for FieldRows {
    fn fields(&self) -> Option<&Fields> {
        self.fields.as_ref()
    }
}

// `--pick`, the rows beside it and the shape its values print in: one value, passed whole. `--pick`'s long help is
// the command's, set by `mut_arg(PICK, …)` where it is flattened.
#[derive(clap::Args, Clone, Debug, Default)]
pub(crate) struct Picked<R: Rows = NoRows> {
    #[arg(id = PICK, long, value_name = "PATH", value_parser = Pick::parse, help = HELP)]
    pick: Option<Pick>,

    #[command(flatten)]
    rows: R,

    #[command(flatten)]
    shape: Shape,
}

impl<R: Rows> Picked<R> {
    /// Whether `--pick` was given.
    pub(crate) fn picks(&self) -> bool {
        self.pick.is_some()
    }

    /// `document` as the JSON form prints it, or the values `--pick` reaches in it, in the shape.
    pub(crate) fn print_json(&self, terminal: &Terminal, document: &impl serde::Serialize) {
        let Some(pick) = &self.pick else {
            return terminal.json(document);
        };
        let value = serde_json::to_value(document).expect("serialize JSON output");
        let lines = self.shape.shaped(pick.texts(&value, self.rows.fields()));
        // A picked string is a file's text, a commit subject or a response: exact into a pipe, which a script
        // reads, but escaped before a terminal, where an escape sequence would act (ADR 0339 D4).
        if terminal.facts().out_terminal {
            out!(
                terminal,
                "{}",
                specforge_common::remedy::printable_lines(&lines)
            );
        } else {
            out!(terminal, "{lines}");
        }
    }

    /// What `--pick` prints for an answer printed as text (`None` without `--pick`): nothing for an empty answer,
    /// else `Err` with the refusal when it is not JSON.
    pub(crate) fn printed(&self, answer: &str) -> Option<Result<String, String>> {
        let pick = self.pick.as_ref()?;
        Some(pick.printed(answer, self.rows.fields(), &self.shape))
    }
}

impl Picked {
    /// The host options of an extension command (ADR 0325 D4, D8): `--pick` and the shapes `added` names (of
    /// [`SHAPES`]), the args this type derives, each shape conflicting only with the other shapes added, so a
    /// declared `count` keeps `--first`.
    pub(crate) fn host_args(added: &[&str]) -> Vec<clap::Arg> {
        use clap::Args as _;
        let derived = Self::augment_args(clap::Command::new(""));
        derived
            .get_arguments()
            .filter(|arg| arg.get_id() == PICK || added.contains(&arg.get_id().as_str()))
            .map(|arg| {
                let kept: Vec<clap::Id> = derived
                    .get_arg_conflicts_with(arg)
                    .into_iter()
                    .map(|other| other.get_id().clone())
                    .filter(|id| added.contains(&id.as_str()))
                    .collect();
                // the order the empty command gave it is reset: the host's command orders it
                arg.clone()
                    .display_order(None)
                    .conflicts_with(clap::builder::Resettable::Reset)
                    .conflicts_with_all(kept)
            })
            .collect()
    }

    /// What an extension command's `matches` hold for the host options `added` names: a name the command declares
    /// is its own arg, never read here.
    pub(crate) fn of_matches(matches: &clap::ArgMatches, added: &[&str]) -> Self {
        Self {
            pick: matches.try_get_one::<Pick>(PICK).ok().flatten().cloned(),
            rows: NoRows {},
            shape: Shape::of_matches(matches, added),
        }
    }
}

/// The short help of `--count`.
pub(crate) const COUNT_HELP: &str = "With --pick: print the number of values it reaches";

/// The long help of `--count`.
pub(crate) const COUNT_LONG_HELP: &str = "With --pick: print the number of values the path reaches, the lines --pick \
     would print (null or a missing value is none, a string holding a newline is one), 0 for none; nothing when the \
     command answered nothing, so `$(… --count) || n='?'` tells an unread answer from 0. It takes no --first, \
     --last, --join or --fields (ADR 0325 D8).";

/// The short help of `--first`.
pub(crate) const FIRST_HELP: &str = "With --pick: keep only the first N values it reaches";

/// The short help of `--last`.
pub(crate) const LAST_HELP: &str = "With --pick: keep only the last N values it reaches";

/// The short help of `--join`.
pub(crate) const JOIN_HELP: &str = "With --pick: print the values on one line, joined by SEP";

/// The long help of `--join`.
pub(crate) const JOIN_LONG_HELP: &str = "With --pick: print the values kept on one line joined by SEP exactly as \
     given (no escape is read: $'\\n  ' is the shell's), then a newline; nothing when no value is reached. It \
     combines with --first or --last, and takes no --count or --fields (ADR 0325 D8).";

/// How the values `--pick` reaches print (ADR 0325 D8): one per line (no shape given), their number, the first or
/// last n, or on one line joined by a separator.
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Shape {
    #[arg(id = COUNT, long, requires = PICK, conflicts_with_all = [FIRST, LAST, JOIN], help = COUNT_HELP, long_help = COUNT_LONG_HELP)]
    count: bool,

    #[arg(id = FIRST, long, value_name = "N", requires = PICK, conflicts_with = LAST, help = FIRST_HELP)]
    first: Option<usize>,

    #[arg(id = LAST, long, value_name = "N", requires = PICK, help = LAST_HELP)]
    last: Option<usize>,

    #[arg(id = JOIN, long, value_name = "SEP", requires = PICK, allow_hyphen_values = true, help = JOIN_HELP, long_help = JOIN_LONG_HELP)]
    join: Option<String>,
}

/// The shapes' names, the ids [`Shape`] derives its args under: an extension command takes each as a host option
/// unless it declares an arg of the name.
pub(crate) const SHAPES: [&str; 4] = [COUNT, FIRST, LAST, JOIN];
pub(crate) const COUNT: &str = "count";
const FIRST: &str = "first";
const LAST: &str = "last";
pub(crate) const JOIN: &str = "join";

impl Shape {
    /// The shape `matches` holds for the host options `added` names: a name the command declares is its own arg,
    /// never read here.
    fn of_matches(matches: &clap::ArgMatches, added: &[&str]) -> Self {
        let has = |name: &str| added.contains(&name);
        Self {
            count: has(COUNT) && matches.get_flag(COUNT),
            first: has(FIRST)
                .then(|| matches.get_one::<usize>(FIRST).copied())
                .flatten(),
            last: has(LAST)
                .then(|| matches.get_one::<usize>(LAST).copied())
                .flatten(),
            join: has(JOIN)
                .then(|| matches.get_one::<String>(JOIN).cloned())
                .flatten(),
        }
    }

    /// The text the shape makes of `texts` (one per value or row reached, in document order); it prints nothing.
    fn shaped(&self, texts: Vec<String>) -> String {
        if self.count {
            return format!("{}\n", texts.len());
        }
        let kept = match (self.first, self.last) {
            (Some(n), _) => &texts[..n.min(texts.len())],
            (_, Some(n)) => &texts[texts.len().saturating_sub(n)..],
            (None, None) => &texts[..],
        };
        match &self.join {
            Some(_) if kept.is_empty() => String::new(),
            Some(separator) => kept.join(separator) + "\n",
            None => kept.iter().map(|text| format!("{text}\n")).collect(),
        }
    }
}

/// One step of a path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Key(String),
    Index(usize),
    Each,
    /// `[<key>=<value>]`: the items whose field at the dotted `key` (its segments) is `value`: a string by its text,
    /// another scalar as JSON.
    Field(Vec<String>, String),
}

/// Whether `item`'s field at the dotted `key` is `wanted`: a string by its text, another scalar as JSON.
fn field_is(item: &Value, key: &[String], wanted: &str) -> bool {
    match key
        .iter()
        .try_fold(item, |value, segment| value.get(segment))
    {
        Some(Value::String(text)) => text == wanted,
        // the wanted text read as JSON: `2` is 2 and `true` is true; the integer 2 is not the float 2.0
        Some(scalar @ (Value::Bool(_) | Value::Number(_) | Value::Null)) => {
            serde_json::from_str::<Value>(wanted).is_ok_and(|w| w == *scalar)
        }
        _ => false,
    }
}

/// A parsed `--fields` list: the dotted keys of a row's cells, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fields(Vec<Vec<String>>);

impl Fields {
    /// Parse `a,b.c`; a refusal names the key that goes wrong.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        text.split(',')
            .map(|key| {
                if key.is_empty() || key.split('.').any(str::is_empty) {
                    Err(format!(
                        "'{key}': a key is names joined by '.', and keys are joined by ','"
                    ))
                } else if key.contains(|c: char| c.is_whitespace() || "[]=".contains(c)) {
                    Err(format!(
                        "'{key}': a key holds no whitespace, '[', ']' or '='"
                    ))
                } else {
                    Ok(key.split('.').map(str::to_string).collect())
                }
            })
            .collect::<Result<_, _>>()
            .map(Self)
    }

    /// The row of `item`: a cell per key, separated by the unit separator (0x1f).
    fn row(&self, item: &Value) -> String {
        self.0
            .iter()
            .map(|key| {
                match key
                    .iter()
                    .try_fold(item, |value, segment| value.get(segment))
                {
                    None | Some(Value::Null) => String::new(),
                    Some(Value::String(text)) => escaped(text),
                    Some(other) => escaped(&other.to_string()),
                }
            })
            .collect::<Vec<_>>()
            .join("\u{1f}")
    }
}

/// `text` with a backslash, newline, carriage return and unit separator written as `\\`, `\n`, `\r` and `\x1f`: one
/// cell.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\u{1f}' => out.push_str("\\x1f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

/// A parsed `--pick` path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pick(Vec<Step>);

impl Pick {
    /// Parse `text`; a refusal names the byte where the path goes wrong.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        // `.` alone is the whole answer, printed compact
        if text == "." {
            return Ok(Self(Vec::new()));
        }
        let mut steps = Vec::new();
        let mut at = 0;
        while at < text.len() {
            let rest = &text[at..];
            if let Some(key) = rest.strip_prefix('.') {
                // `.[`, as jq writes it, is `[`
                if key.starts_with('[') {
                    at += 1;
                    continue;
                }
                let len = key.find(['.', '[']).unwrap_or(key.len());
                if len == 0 {
                    return Err(format!("at byte {at}: '.' needs a key after it"));
                }
                if let Some(bad) =
                    key[..len].find(|c: char| c.is_whitespace() || c == ']' || c == '=')
                {
                    return Err(format!(
                        "at byte {}: a key holds no whitespace, ']' or '='",
                        at + 1 + bad
                    ));
                }
                steps.push(Step::Key(key[..len].to_string()));
                at += 1 + len;
            } else if let Some(inner) = rest.strip_prefix('[') {
                let Some(len) = inner.find(']') else {
                    return Err(format!("at byte {at}: '[' is not closed"));
                };
                let inner = &inner[..len];
                steps.push(match inner.split_once('=') {
                    Some(("", _)) => return Err(format!("at byte {at}: '[=…]' needs a key")),
                    Some((_, "")) => return Err(format!("at byte {at}: '[{inner}]' needs a value")),
                    Some((key, _)) if key.contains(char::is_whitespace) => {
                        return Err(format!("at byte {at}: a key holds no whitespace, ']' or '='"));
                    }
                    Some((key, _)) if key.split('.').any(str::is_empty) => {
                        return Err(format!("at byte {at}: a dotted key has a name between its dots"));
                    }
                    Some((key, value)) => Step::Field(
                        key.split('.').map(str::to_string).collect(),
                        value.to_string(),
                    ),
                    None if inner.is_empty() => Step::Each,
                    // digits only, no sign and no leading zero: `usize::from_str` takes `+1` and `01`
                    None if inner.bytes().all(|b| b.is_ascii_digit())
                        && (inner == "0" || !inner.starts_with('0')) =>
                    {
                        Step::Index(inner.parse().map_err(|_| {
                            format!("at byte {at}: the index '{inner}' is too large")
                        })?)
                    }
                    None => {
                        return Err(format!(
                            "at byte {at}: expected '[<index>]', '[]' or '[<key>=<value>]', found '[{inner}]'"
                        ));
                    }
                });
                at += 2 + len;
            } else {
                return Err(format!(
                    "at byte {at}: expected '.<key>' or '[', found '{rest}'"
                ));
            }
        }
        if steps.is_empty() {
            return Err("at byte 0: the path is empty".to_string());
        }
        Ok(Self(steps))
    }

    /// The values `document` holds at the path, in document order.
    fn values<'v>(&self, document: &'v Value) -> Vec<&'v Value> {
        let mut reached = vec![document];
        for step in &self.0 {
            reached = reached
                .into_iter()
                .flat_map(|value| -> Vec<&Value> {
                    match (step, value) {
                        (Step::Key(key), Value::Object(object)) => {
                            object.get(key).into_iter().collect()
                        }
                        (Step::Index(n), Value::Array(items)) => {
                            items.get(*n).into_iter().collect()
                        }
                        (Step::Each, Value::Array(items)) => items.iter().collect(),
                        (Step::Field(key, wanted), Value::Array(items)) => items
                            .iter()
                            .filter(|item| field_is(item, key, wanted))
                            .collect(),
                        _ => Vec::new(),
                    }
                })
                .collect();
        }
        reached
    }

    /// The texts `--pick` prints for `document`, in document order: one per value reached, a null or missing value
    /// none; with `fields`, one row per value reached, null included.
    fn texts(&self, document: &Value, fields: Option<&Fields>) -> Vec<String> {
        let values = self.values(document).into_iter();
        match fields {
            Some(fields) => values.map(|item| fields.row(item)).collect(),
            None => values
                .filter_map(|value| match value {
                    Value::Null => None,
                    Value::String(s) => Some(s.clone()),
                    other => Some(other.to_string()),
                })
                .collect(),
        }
    }

    /// What `--pick` prints in `shape` for an answer printed as text: nothing for an empty answer, else `Err` with
    /// the answer when it is not JSON.
    fn printed(
        &self,
        answer: &str,
        fields: Option<&Fields>,
        shape: &Shape,
    ) -> Result<String, String> {
        if answer.trim().is_empty() {
            return Ok(String::new());
        }
        serde_json::from_str(answer)
            .map(|document| shape.shaped(self.texts(&document, fields)))
            .map_err(|e| format!("error: --pick: the answer is not JSON ({e})\n"))
    }
}

#[cfg(test)]
mod tests;
