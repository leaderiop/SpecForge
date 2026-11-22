//! `specforge-code-lcov`: the LCOV reader the coverage packs link (ADR 0388).
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. The wire has no
//! guest-to-guest call, so a coverage pack reads its tracefile here; `specforge-code-facts` mints the `coverage` facts
//! from what it reads ([`specforge_code_facts::coverage_facts`]).
//!
//! - [`read`] reads a tracefile (geninfo(1) LCOV, `cargo llvm-cov --lcov`, c8/istanbul `lcovonly`) into one
//!   [`CoverageFile`] per file section, its path made project-relative.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use specforge_code_facts::{relative, Branch, CoverageFile, Function};

/// The file sections of the LCOV tracefile `text`, in document order, each `SF:` path made project-relative against
/// the project root `cwd`. Read: `SF`, `FN`, `FNDA`, `DA`, `BRDA` and `end_of_record`; ignored: `TN`, `VER`, the file
/// totals (`FNF`, `FNH`, `LF`, `LH`, `BRF`, `BRH`), `MCDC`/`MCF`/`MCH`, `#` comments, blank lines and any other
/// all-capitals `KEY:`. An error names the 1-based line: a line that is no `KEY:value`, a record outside a file
/// section, a malformed number, an `SF` outside the project, and a text ending inside a file section, which must not
/// drop its last file. An empty text has no file.
pub fn read(text: &str, cwd: &str) -> Result<Vec<CoverageFile>, String> {
    let mut files = Vec::new();
    let mut open: Option<Section> = None;
    for (n, line) in text.lines().enumerate() {
        let at = n + 1;
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        if line.trim_end() == "end_of_record" {
            let section = open
                .take()
                .ok_or_else(|| format!("line {at}: end_of_record outside a file section"))?;
            files.push(section.file);
            continue;
        }
        let (key, value) = line
            .split_once(':')
            .filter(|(key, _)| !key.is_empty() && key.bytes().all(|b| b.is_ascii_uppercase()))
            .ok_or_else(|| format!("line {at}: `{}` is no LCOV record (KEY:value)", shown(line)))?;
        if key == "SF" {
            if let Some(section) = &open {
                return Err(format!(
                    "line {at}: a new file section opens before `{}` (line {}) ends with end_of_record",
                    section.file.file, section.opened
                ));
            }
            let file = relative(value, cwd).ok_or_else(|| {
                format!(
                    "line {at}: the file `{}` lies outside the project",
                    shown(value)
                )
            })?;
            open = Some(Section::new(file, at));
            continue;
        }
        if !matches!(key, "FN" | "FNDA" | "DA" | "BRDA") {
            continue;
        }
        let section = open.as_mut().ok_or_else(|| {
            format!("line {at}: the record {key} lies outside a file section (no SF before it)")
        })?;
        section
            .record(key, value)
            .map_err(|why| format!("line {at}: {key}:{} {why}", shown(value)))?;
    }
    match open {
        Some(section) => Err(format!(
            "line {}: the report ends inside the file `{}` opened here, with no end_of_record",
            section.opened, section.file.file
        )),
        None => Ok(files),
    }
}

/// An open file section: what it has read so far, the line its `SF` was on, and its functions by name.
struct Section {
    file: CoverageFile,
    opened: usize,
    /// Name → the index in `file.functions` of the first function of that name.
    named: HashMap<String, usize>,
}

impl Section {
    fn new(file: String, opened: usize) -> Section {
        Section {
            file: CoverageFile {
                file,
                ..CoverageFile::default()
            },
            opened,
            named: HashMap::new(),
        }
    }

    /// Reads the record `key:value` of this section; the error says what is wrong with it.
    fn record(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            // FN:<line>[,<end line>],<name>: the second field is an end line only when it is a number and a name
            // follows, since a C++ name holds commas.
            "FN" => {
                let (line, rest) = value.split_once(',').ok_or("has no name")?;
                let line = number(line)?;
                let name = match rest.split_once(',') {
                    Some((end, name)) if end.trim().parse::<usize>().is_ok() => name,
                    _ => rest,
                };
                self.function(line, name)
            }
            // FNDA:<hits>,<name>: paired with the FN of that name, else kept at line 0, which no item holds.
            "FNDA" => {
                let (hits, name) = value.split_once(',').ok_or("has no name")?;
                let hits = count(hits)?;
                if name.is_empty() {
                    return Err("has no name".to_string());
                }
                let index = match self.named.get(name) {
                    Some(&index) => index,
                    None => self.push(0, name),
                };
                if let Some(function) = self.file.functions.get_mut(index) {
                    function.hits = function.hits.saturating_add(hits);
                }
                Ok(())
            }
            // DA:<line>,<hits>[,<checksum>]
            "DA" => {
                let mut fields = value.splitn(3, ',');
                let line = number(fields.next().unwrap_or_default())?;
                let hits = count(fields.next().ok_or("has no hit count")?)?;
                self.file.lines.push((line, hits));
                Ok(())
            }
            // BRDA:<line>,<block>,<branch>,<taken or ->: the branch may be an expression holding commas.
            "BRDA" => {
                let (line, rest) = value.split_once(',').ok_or("has no block")?;
                let (rest, taken) = rest.rsplit_once(',').ok_or("has no taken count")?;
                let (block, branch) = rest.split_once(',').ok_or("has no branch")?;
                let taken = match taken.trim() {
                    "-" => None,
                    taken => Some(count(taken)?),
                };
                self.file.branches.push(Branch {
                    line: number(line)?,
                    block: block.to_string(),
                    branch: branch.to_string(),
                    taken,
                });
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// A function starting at `line`: an `FNDA` read before it, kept at line 0, takes its line.
    fn function(&mut self, line: usize, name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("has no name".to_string());
        }
        match self.named.get(name).copied() {
            Some(index) if self.file.functions.get(index).is_some_and(|f| f.line == 0) => {
                if let Some(function) = self.file.functions.get_mut(index) {
                    function.line = line;
                }
            }
            _ => {
                self.push(line, name);
            }
        }
        Ok(())
    }

    /// Appends a function with no hit yet; the first of a name is the one an `FNDA` of that name pairs with.
    fn push(&mut self, line: usize, name: &str) -> usize {
        let index = self.file.functions.len();
        self.file.functions.push(Function {
            line,
            name: name.to_string(),
            hits: 0,
        });
        self.named.entry(name.to_string()).or_insert(index);
        index
    }
}

/// A 1-based line number.
fn number(text: &str) -> Result<usize, String> {
    text.trim()
        .parse()
        .map_err(|_| format!("has `{}` where a line number belongs", shown(text)))
}

/// A hit or taken count.
fn count(text: &str) -> Result<u64, String> {
    text.trim()
        .parse()
        .map_err(|_| format!("has `{}` where a count belongs", shown(text)))
}

/// At most 80 characters of report text, control characters escaped, for an error message.
fn shown(text: &str) -> String {
    let mut shown: String = text.chars().take(80).flat_map(char::escape_debug).collect();
    if text.chars().nth(80).is_some() {
        shown.push('…');
    }
    shown
}
