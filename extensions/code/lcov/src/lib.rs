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

use specforge_code_facts::{relative, shown, Branch, CoverageFile, Function};

/// The file sections of the LCOV tracefile `text`, in document order, each `SF:` path made project-relative against
/// the project root `cwd`. Read: `SF`, `FN`, `FNDA`, the LCOV 2.2 group form `FNL`/`FNA`, `DA`, `BRDA` and
/// `end_of_record`; ignored: `TN`, `VER`, the file
/// totals (`FNF`, `FNH`, `LF`, `LH`, `BRF`, `BRH`), `MCDC`/`MCF`/`MCH`, `#` comments, blank lines and any other
/// all-capitals `KEY:`. An `SF` path is trimmed. An error names the 1-based line: a line that is no `KEY:value`, a
/// record outside a file section, a malformed number, an `SF` naming no file or one outside the project, and a text
/// ending inside a file section, which must not drop its last file. An empty text has no file.
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
                    shown(&section.file.file),
                    section.opened
                ));
            }
            let value = value.trim();
            if value.is_empty() {
                return Err(format!("line {at}: SF names no file"));
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
        if !matches!(key, "FN" | "FNDA" | "FNL" | "FNA" | "DA" | "BRDA") {
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
            section.opened,
            shown(&section.file.file)
        )),
        None => Ok(files),
    }
}

/// An open file section: what it has read so far, the line its `SF` was on, its functions by name and the start lines
/// of its function groups.
struct Section {
    file: CoverageFile,
    opened: usize,
    /// Name → the functions of that name and the `FN` and `FNDA` records of it read so far.
    named: HashMap<String, Named>,
    /// `FNL` index → the group's start line.
    groups: HashMap<String, usize>,
}

/// The functions of one name, in the order their first record met them, and how many `FN` and `FNDA` records of the
/// name have been read: the n-th `FN` and the n-th `FNDA` of a name are one function's (two impls' `new`, `fmt`).
#[derive(Default)]
struct Named {
    /// Indexes in `file.functions`.
    functions: Vec<usize>,
    fns: usize,
    fndas: usize,
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
            groups: HashMap::new(),
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
            // FNDA:<hits>,<name>: paired with the FN of that name and rank, else kept at line 0, which no item holds.
            "FNDA" => {
                let (hits, name) = value.split_once(',').ok_or("has no name")?;
                let hits = count(hits)?;
                if name.is_empty() {
                    return Err("has no name".to_string());
                }
                let index = self.pair(name, Record::Fnda);
                if let Some(function) = self.file.functions.get_mut(index) {
                    function.hits = function.hits.saturating_add(hits);
                }
                Ok(())
            }
            // FNL:<index>,<start line>[,<end line>]: a function group (LCOV 2.2), its aliases named by FNA.
            "FNL" => {
                let (index, lines) = value.split_once(',').ok_or("has no start line")?;
                let (start, end) = match lines.split_once(',') {
                    Some((start, end)) => (start, Some(end)),
                    None => (lines, None),
                };
                let start = number(start)?;
                end.map(number).transpose()?;
                self.groups.insert(index.trim().to_string(), start);
                Ok(())
            }
            // FNA:<index>,<hits>,<name>: one alias of a group, a function at its start line (0 with no FNL).
            "FNA" => {
                let mut fields = value.splitn(3, ',');
                let index = fields.next().unwrap_or_default().trim();
                let hits = count(fields.next().ok_or("has no hit count")?)?;
                let name = fields
                    .next()
                    .filter(|name| !name.is_empty())
                    .ok_or("has no name")?;
                self.file.functions.push(Function {
                    line: self.groups.get(index).copied().unwrap_or(0),
                    name: name.to_string(),
                    hits,
                });
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

    /// A function starting at `line`: an `FNDA` of its name and rank read before it, kept at line 0, takes its line.
    fn function(&mut self, line: usize, name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("has no name".to_string());
        }
        let index = self.pair(name, Record::Fn);
        if let Some(function) = self.file.functions.get_mut(index) {
            function.line = line;
        }
        Ok(())
    }

    /// The index of the function the next `record` of `name` belongs to: the n-th of its kind pairs with the n-th
    /// function of that name, one no record has met yet appended at line 0 with no hit.
    fn pair(&mut self, name: &str, record: Record) -> usize {
        let named = self.named.entry(name.to_string()).or_default();
        let rank = match record {
            Record::Fn => &mut named.fns,
            Record::Fnda => &mut named.fndas,
        };
        let at = *rank;
        *rank += 1;
        match named.functions.get(at) {
            Some(&index) => index,
            None => {
                let index = self.file.functions.len();
                self.file.functions.push(Function {
                    line: 0,
                    name: name.to_string(),
                    hits: 0,
                });
                named.functions.push(index);
                index
            }
        }
    }
}

/// The two records that pair by name.
#[derive(Clone, Copy)]
enum Record {
    Fn,
    Fnda,
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
