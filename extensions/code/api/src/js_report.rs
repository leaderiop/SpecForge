//! The api-extractor `.api.md` report: the API a package accepted, the file a project commits for review.

use specforge_code_facts::{ApiEntry, Scheme, Visibility};

use crate::js_model::{api_kind, nameless};

/// The release tags a tag line writes, and their names as the doc model spells them.
const TAGS: [(&str, &str); 4] = [
    ("@public", "Public"),
    ("@beta", "Beta"),
    ("@alpha", "Alpha"),
    ("@internal", "Internal"),
];

/// The deepest a body may nest: a report nests two or three; more is a hostile file.
const DEPTH_MAX: usize = 64;

/// Words before a declaration's keyword.
const MODIFIERS: [&str; 4] = ["export", "declare", "default", "abstract"];

/// Words before a member's name.
const MEMBER_MODIFIERS: [&str; 9] = [
    "static",
    "readonly",
    "abstract",
    "protected",
    "public",
    "private",
    "declare",
    "override",
    "async",
];

/// What an open body holds.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Body {
    /// A class's or an interface's members.
    Members,
    Enum,
    /// A namespace's declarations.
    Namespace,
}

/// A declaration whose body is open.
struct Frame {
    name: String,
    /// The tag its members inherit.
    tag: &'static str,
    indent: usize,
    body: Body,
    line: usize,
}

/// A declaration or member read over more than one line.
struct Pending {
    text: String,
    line: usize,
    indent: usize,
    /// Its own tag line's.
    tag: Option<&'static str>,
}

/// The entries of the api-extractor report `text`, in document order (the tool writes them alphabetically).
///
/// The grammar, as api-extractor 7.60.1 writes it: an H2 title `## API Report File for "<package>"` names the package
/// (every entry's container), and the body is the one ```` ```ts ```` fence after it. In the fence, `import` lines
/// are skipped; a declaration is a top-level line `export [declare] [default] [abstract]` then `function`, `class`,
/// `interface`, `[const] enum`, `type`, `const`, `let`, `var` or `namespace`, then its name; a class, interface, enum
/// or namespace line ending `{` opens a body, closed by the `}` line at its own indent. A class's or interface's
/// members are the indented lines of its body: a method (`name(`, `name<`), a property, a `get`/`set` accessor (a
/// pair on consecutive lines is one entry, its two texts joined by one space, as the doc model holds one property),
/// a `constructor`, and the nameless `[key: …]` (`[]`), `(…)` (`()`) and `new (…)` (`new()`) the doc model names
/// that way; an enum's members are its lines; a namespace's are declarations. A declaration or member ends at the
/// first line at or left of its own indent ending `;` or `,` (an enum member at its line), so a multi-line type alias
/// is one entry. The release tag is the first `@public`, `@beta`, `@alpha` or `@internal` among the leading `@` words
/// of the comment lines directly above (a warning's prose and a blank line are no tag line), as the doc model spells
/// it (`Public`, …); a member with none inherits its container's, a declaration with none is `Public`.
///
/// Each entry: the path its names (a namespace's members below it), the kind as [`crate::read_js_model`] maps the
/// doc model's, visibility public, no owner, the signature its text as the report wrote it, a `//` comment inside it
/// removed and its lines trimmed and joined by one space. One pass over the lines, no recursion. An error, naming
/// why: no title before the fence, no fence, a fence or a body never closed, a declaration never ended, a closing
/// brace with no body open, a line that is no declaration or member, a body nested deeper than 64, a line starting
/// `<<<<<<<`, `=======` or `>>>>>>>` (a merge conflict), each with its 1-based line.
pub fn read_js_report(text: &str) -> Result<Vec<ApiEntry>, String> {
    let mut package: Option<&str> = None;
    let mut reader: Option<Reader> = None;
    let mut opened = 0;
    let mut closed = false;
    for (n, line) in text.lines().enumerate() {
        let n = n + 1;
        let line = line.strip_suffix('\r').unwrap_or(line);
        if ["<<<<<<<", "=======", ">>>>>>>"]
            .iter()
            .any(|marker| line.starts_with(marker))
        {
            return Err(format!("line {n}: a merge-conflict marker"));
        }
        if closed {
            continue;
        }
        match reader.as_mut() {
            Some(open) if line.trim_end() == "```" => {
                open.finish(n)?;
                closed = true;
            }
            Some(open) => open.line(line, n)?,
            None if line.trim_end() == "```ts" => {
                let package = package.ok_or_else(no_title)?;
                reader = Some(Reader::new(package));
                opened = n;
            }
            None => {
                package = package.or_else(|| {
                    line.strip_prefix("## API Report File for \"")
                        .and_then(|rest| rest.trim_end().strip_suffix('"'))
                        .filter(|name| !name.is_empty())
                });
            }
        }
    }
    match reader {
        None if package.is_none() => Err(no_title()),
        None => Err("no ```ts fence holds the report's body".to_string()),
        Some(_) if !closed => Err(format!("line {opened}: the ```ts fence is never closed")),
        Some(open) => Ok(open.entries),
    }
}

fn no_title() -> String {
    "no `## API Report File for \"<package>\"` title before the ```ts fence".to_string()
}

/// The fence's state between its lines.
struct Reader<'t> {
    package: &'t str,
    entries: Vec<ApiEntry>,
    frames: Vec<Frame>,
    pending: Option<Pending>,
    /// The tag of the comment lines read since the last blank line, declaration or member.
    tag: Option<&'static str>,
    /// The last entry, when it is an accessor its pair joins.
    accessor: Option<usize>,
}

impl<'t> Reader<'t> {
    fn new(package: &'t str) -> Self {
        Reader {
            package,
            entries: Vec::new(),
            frames: Vec::new(),
            pending: None,
            tag: None,
            accessor: None,
        }
    }

    /// Read the fence's line `line`, the `n`th of the file.
    fn line(&mut self, line: &str, n: usize) -> Result<(), String> {
        let trimmed = line.trim();
        let indent = line.len() - line.trim_start().len();
        let code = without_comment(trimmed);
        if let Some(pending) = &mut self.pending {
            if !code.is_empty() {
                pending.text.push(' ');
                pending.text.push_str(code);
            }
            if indent <= pending.indent && ends(code) {
                return self.complete(n);
            }
            return Ok(());
        }
        if trimmed.is_empty() {
            self.tag = None;
            return Ok(());
        }
        if let Some(comment) = trimmed.strip_prefix("//") {
            self.tag = self.tag.or_else(|| release_tag(comment));
            return Ok(());
        }
        let body = self.frames.last().map(|frame| (frame.indent, frame.body));
        if trimmed.starts_with('}') {
            return match body {
                Some((open, _)) if open == indent => {
                    self.frames.pop();
                    self.tag = None;
                    self.accessor = None;
                    Ok(())
                }
                _ => Err(format!(
                    "line {n}: a closing brace with no body open at its indent"
                )),
            };
        }
        if body.is_none() && trimmed.starts_with("import ") {
            self.tag = None;
            return Ok(());
        }
        self.pending = Some(Pending {
            text: code.to_string(),
            line: n,
            indent,
            tag: self.tag.take(),
        });
        let opens = !matches!(body, Some((_, Body::Members)))
            && code.ends_with('{')
            && declaration(code).is_some_and(|(_, _, opens)| opens.is_some());
        if matches!(body, Some((_, Body::Enum))) || ends(code) || opens {
            return self.complete(n);
        }
        Ok(())
    }

    /// The pending declaration or member is whole: its entry, and its body when it opens one.
    fn complete(&mut self, n: usize) -> Result<(), String> {
        let Some(pending) = self.pending.take() else {
            return Ok(());
        };
        let parent = self.frames.last();
        let text = pending.text.as_str();
        let (kind, name, accessor, opens) = match parent.map(|frame| frame.body) {
            Some(Body::Enum) => (Some("EnumMember"), identifier(text), false, None),
            Some(Body::Members) => match member(text) {
                Some((kind, name, accessor)) => (Some(kind), Some(name), accessor, None),
                None => (None, None, false, None),
            },
            _ => match declaration(text) {
                Some((kind, name, opens)) => (Some(kind), Some(name), false, opens),
                None => (None, None, false, None),
            },
        };
        let (Some(kind), Some(name)) = (kind, name) else {
            return Err(format!(
                "line {}: neither a declaration nor a member the report grammar knows",
                pending.line
            ));
        };
        let tag = pending
            .tag
            .or(parent.map(|frame| frame.tag))
            .unwrap_or("Public");
        let mut path: Vec<String> = self.frames.iter().map(|f| f.name.clone()).collect();
        path.push(name.to_string());
        self.tag = None;
        if let Some(last) = self.accessor.filter(|_| accessor) {
            if self.entries[last].path == path {
                self.entries[last].signature.push(' ');
                self.entries[last].signature.push_str(text);
                self.accessor = None;
                return Ok(());
            }
        }
        self.accessor = accessor.then_some(self.entries.len());
        self.entries.push(ApiEntry {
            scheme: Scheme::Js,
            container: self.package.to_string(),
            path,
            owner: None,
            kind: api_kind(kind),
            visibility: Visibility::Public,
            signature: text.to_string(),
            tag: Some(tag.to_string()),
        });
        if let Some(body) = opens.filter(|_| text.ends_with('{')) {
            if self.frames.len() == DEPTH_MAX {
                return Err(format!("line {n}: a body nested deeper than {DEPTH_MAX}"));
            }
            self.frames.push(Frame {
                name: name.to_string(),
                tag,
                indent: pending.indent,
                body,
                line: pending.line,
            });
        }
        Ok(())
    }

    /// The fence closes at line `n`: nothing may still be open.
    fn finish(&self, n: usize) -> Result<(), String> {
        if let Some(pending) = &self.pending {
            return Err(format!(
                "line {}: a declaration never ended before the fence closes at line {n}",
                pending.line
            ));
        }
        match self.frames.last() {
            Some(frame) => Err(format!(
                "line {}: the body of `{}` is never closed",
                frame.line, frame.name
            )),
            None => Ok(()),
        }
    }
}

/// Whether a line's code ends a declaration or member.
fn ends(code: &str) -> bool {
    code.ends_with(';') || code.ends_with(',')
}

/// The first release tag among the leading `@` words of a comment, as the doc model spells it.
fn release_tag(comment: &str) -> Option<&'static str> {
    comment
        .split_whitespace()
        .take_while(|word| word.starts_with('@'))
        .find_map(|word| TAGS.iter().find(|(at, _)| *at == word).map(|(_, tag)| *tag))
}

/// `code` up to a `//` outside a string, trimmed.
// shortcut: a backslash-escaped quote inside a string is not seen; the report prints string literal types only.
fn without_comment(code: &str) -> &str {
    let mut quote = None;
    for (at, c) in code.char_indices() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None if matches!(c, '"' | '\'' | '`') => quote = Some(c),
            None if code[at..].starts_with("//") => return code[..at].trim_end(),
            None => {}
        }
    }
    code
}

/// The name a word starts with: up to the first character that ends a name.
fn identifier(word: &str) -> Option<&str> {
    let end = word
        .find(|c: char| c.is_whitespace() || "(<:=;,?!{".contains(c))
        .unwrap_or(word.len());
    (end > 0).then(|| &word[..end])
}

/// The doc model's kind, the name and the body of the declaration `code` (top-level or in a namespace).
fn declaration(code: &str) -> Option<(&'static str, &str, Option<Body>)> {
    let words: Vec<&str> = code.split_whitespace().collect();
    let mut at = words.iter().position(|word| !MODIFIERS.contains(word))?;
    if words[at] == "const" && words.get(at + 1) == Some(&"enum") {
        at += 1;
    }
    let (kind, body) = match words[at] {
        "function" => ("Function", None),
        "class" => ("Class", Some(Body::Members)),
        "interface" => ("Interface", Some(Body::Members)),
        "enum" => ("Enum", Some(Body::Enum)),
        "namespace" | "module" => ("Namespace", Some(Body::Namespace)),
        "type" => ("TypeAlias", None),
        "const" | "let" | "var" => ("Variable", None),
        _ => return None,
    };
    Some((kind, identifier(words.get(at + 1)?)?, body))
}

/// The doc model's kind and the name of the class or interface member `code`, and whether it is an accessor.
fn member(code: &str) -> Option<(&'static str, &str, bool)> {
    let mut rest = code;
    while let Some((word, after)) = rest.split_once(' ') {
        if !MEMBER_MODIFIERS.contains(&word) {
            break;
        }
        rest = after.trim_start();
    }
    if let Some(("get" | "set", after)) = rest.split_once(' ') {
        return Some(("Property", identifier(after.trim_start())?, true));
    }
    let signature = |kind| Some((kind, nameless(kind)?, false));
    if rest.starts_with('(') || rest.starts_with('<') {
        return signature("CallSignature");
    }
    if rest
        .strip_prefix("new")
        .is_some_and(|after| after.trim_start().starts_with(['(', '<']))
    {
        return signature("ConstructSignature");
    }
    let (name, after) = match rest.strip_prefix('[') {
        Some(inner) => {
            let close = inner.find(']')?;
            if inner[..close].contains(':') {
                return signature("IndexSignature");
            }
            rest.split_at(close + 2)
        }
        None => {
            let name = identifier(rest)?;
            (name, &rest[name.len()..])
        }
    };
    if name == "constructor" {
        return signature("Constructor");
    }
    let method = after.trim_start_matches(['?', '!']).starts_with(['(', '<']);
    Some((if method { "Method" } else { "Property" }, name, false))
}
