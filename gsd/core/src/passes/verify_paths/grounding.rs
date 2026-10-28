//! Where a verify command runs, read from its text and never executed: a port of `resolveVerifyCommandTarget`
//! (`src/verify-command-grounding.cts`) up to the point where it touches the disk. The guest has no disk, so it only
//! names the paths to look at; the host reads them (K11) and the check pass judges the facts.
//!
//! shortcut: the guest does not know the project root, so an absolute target, or one that climbs out of the project,
//! is `Outside` whether or not it exists (GSD probes a climb that names a sibling, and lets an absolute path inside
//! the root through); upgrade when the host sends the root.

/// What a command's text says about where it runs.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// No `cd` or `--prefix` target, an empty command, or the `MISSING` sentinel.
    Skip,
    /// The target holds `$`, a backtick, `*`, `?`, a newline or a leading `~`.
    Dynamic,
    /// An absolute target, or one that climbs out of the project.
    Outside,
    At(Target),
}

/// A directory the command runs in, and the manifest its tool needs there.
#[derive(Debug, PartialEq, Eq)]
pub struct Target {
    /// Project-relative, lexically normalised; empty is the project root.
    pub dir: String,
    pub need: Need,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Need {
    Nothing,
    /// `package.json`, and the script `npm run <script>` names.
    Package(Option<String>),
    Makefile,
}

impl Target {
    /// The manifest's project-relative path.
    pub fn manifest(&self) -> Option<String> {
        let name = match self.need {
            Need::Nothing => return None,
            Need::Package(_) => "package.json",
            Need::Makefile => "Makefile",
        };
        Some(if self.dir.is_empty() {
            name.to_string()
        } else {
            format!("{}/{name}", self.dir)
        })
    }

    /// Every path the host must read for this target, the directory first.
    pub fn paths(&self) -> Vec<String> {
        let dir = (!self.dir.is_empty()).then(|| self.dir.clone());
        dir.into_iter().chain(self.manifest()).collect()
    }
}

pub fn resolve(command: &str) -> Verdict {
    let trimmed = command.trim().replace("&amp;", "&");
    if trimmed.is_empty() || missing_sentinel(&trimmed) {
        return Verdict::Skip;
    }
    let segments = split_segments(&trimmed);
    let cds: Vec<&str> = segments.iter().map_while(|s| cd_argument(s)).collect();
    let (raw, rest) = if cds.is_empty() {
        match find_prefix(&trimmed, 0) {
            Some(found) => (found.value.to_string(), trimmed.clone()),
            None => return Verdict::Skip,
        }
    } else {
        (fold_cd(&cds), segments[cds.len()..].join(" && "))
    };
    let leading = raw.strip_prefix(['"', '\'']).unwrap_or(&raw);
    if raw.contains(['$', '`', '*', '?', '\n']) || leading.starts_with('~') {
        return Verdict::Dynamic;
    }
    let Some(dir) = lexical(&to_slash(strip_quotes(&raw))) else {
        return Verdict::Outside;
    };
    Verdict::At(Target {
        dir,
        need: need(rest.trim()),
    })
}

fn need(rest: &str) -> Need {
    if ["npm", "npx", "pnpm", "yarn", "bun"]
        .iter()
        .any(|tool| starts_word(rest, tool))
    {
        Need::Package(npm_script(&strip_prefix_flag(rest)))
    } else if starts_word(rest, "make") {
        Need::Makefile
    } else {
        Need::Nothing
    }
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `^word\b`.
fn starts_word(text: &str, word: &str) -> bool {
    text.strip_prefix(word)
        .is_some_and(|after| !after.starts_with(is_word))
}

/// `^MISSING(\s|$)`: the Wave-0 sentinel is not a command.
pub fn missing_sentinel(text: &str) -> bool {
    text.strip_prefix("MISSING")
        .is_some_and(|after| after.is_empty() || after.starts_with(char::is_whitespace))
}

/// Split on `&&`, `||`, `;` and newlines; trim; drop empties. No quote-awareness.
fn split_segments(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let (mut start, mut at) = (0, 0);
    let bytes = text.as_bytes();
    while at < bytes.len() {
        let width = match bytes[at] {
            b';' | b'\n' => 1,
            b'&' | b'|' if bytes.get(at + 1) == Some(&bytes[at]) => 2,
            _ => 0,
        };
        if width == 0 {
            at += 1;
            continue;
        }
        found.push(&text[start..at]);
        at += width;
        start = at;
    }
    found.push(&text[start..]);
    found
        .into_iter()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// `^cd\s+(.+)$`.
fn cd_argument(segment: &str) -> Option<&str> {
    let rest = segment.strip_prefix("cd")?;
    rest.starts_with(char::is_whitespace)
        .then(|| rest.trim())
        .filter(|a| !a.is_empty())
}

fn strip_quotes(text: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = text.strip_prefix(quote).and_then(|t| t.strip_suffix(quote)) {
            return inner;
        }
    }
    text
}

fn to_slash(text: &str) -> String {
    text.replace('\\', "/")
}

/// Chained `cd`s left to right: a relative segment appends, an absolute one starts over.
fn fold_cd(args: &[&str]) -> String {
    args.iter().fold(String::new(), |acc, raw| {
        if acc.is_empty() || to_slash(strip_quotes(raw)).starts_with('/') {
            (*raw).to_string()
        } else {
            format!("{acc}/{raw}")
        }
    })
}

/// `path` without `.` and `//`, `..` folded; `None` when it is absolute or climbs out of the project.
fn lexical(path: &str) -> Option<String> {
    if path.starts_with('/') {
        return None;
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

struct Prefix<'a> {
    start: usize,
    end: usize,
    value: &'a str,
}

/// `(?:^|\s)--prefix(?:=|\s+)("[^"]*"|'[^']*'|\S+)`, the first match from `from`.
fn find_prefix(text: &str, from: usize) -> Option<Prefix<'_>> {
    for (at, _) in text[from..].match_indices("--prefix") {
        let at = from + at;
        let before = text[..at].chars().next_back();
        if before.is_some_and(|c| !c.is_whitespace()) {
            continue;
        }
        let after = &text[at + "--prefix".len()..];
        let rest = match after.strip_prefix('=') {
            Some(rest) => rest,
            None => {
                let rest = after.trim_start();
                if rest.len() == after.len() {
                    continue;
                }
                rest
            }
        };
        let quoted = ['"', '\''].into_iter().find_map(|q| {
            let inner = rest.strip_prefix(q)?;
            Some(&rest[..inner.find(q)? + 2])
        });
        let value = quoted.unwrap_or_else(|| rest.split(char::is_whitespace).next().unwrap_or(""));
        if value.is_empty() {
            continue;
        }
        let start = at - before.map_or(0, char::len_utf8);
        let end = text.len() - rest.len() + value.len();
        return Some(Prefix { start, end, value });
    }
    None
}

/// The command without its `--prefix` flags, so `npm --prefix ./web run lint` reads as `npm run lint`.
fn strip_prefix_flag(text: &str) -> String {
    let (mut out, mut at) = (String::new(), 0);
    while let Some(found) = find_prefix(text, at) {
        out.push_str(&text[at..found.start]);
        out.push(' ');
        at = found.end;
    }
    out.push_str(&text[at..]);
    out
}

/// `\bnpm\s+run\s+([\w:@./-]+)`, the first.
fn npm_script(text: &str) -> Option<String> {
    for (at, _) in text.match_indices("npm") {
        if text[..at].chars().next_back().is_some_and(is_word) {
            continue;
        }
        let after = &text[at + 3..];
        let Some(after) = after
            .trim_start()
            .strip_prefix("run")
            .filter(|_| after.starts_with(char::is_whitespace))
        else {
            continue;
        };
        let name = after.trim_start();
        if name.len() == after.len() {
            continue;
        }
        let script: String = name
            .chars()
            .take_while(|c| is_word(*c) || ":@./-".contains(*c))
            .collect();
        if !script.is_empty() {
            return Some(script);
        }
    }
    None
}

#[cfg(test)]
mod tests;
