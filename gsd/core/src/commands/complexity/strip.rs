//! Ported from gsd-core v1.18.0 src/complexity-trigger.cts (`stripLiterals`), MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! Replaces comment and literal content with spaces, preserving length and every newline so line numbers survive.
//! Handles `//`, `/* */`, `'`, `"`, backtick templates (`${...}` interpolations stay code), regex literals
//! (told from division by the preceding significant token) and backslash escapes. An unterminated string, template
//! or block comment is [`Unparseable`].

/// The source holds a literal or a comment that never ends: no number is emitted for it.
#[derive(Debug, PartialEq, Eq)]
pub struct Unparseable;

/// Words after which a following `/` starts an expression, so it is a regex literal and never a division.
const REGEX_KEYWORD_CONTEXT_WORDS: &[&str] = &[
    "return",
    "typeof",
    "instanceof",
    "in",
    "of",
    "new",
    "delete",
    "void",
    "throw",
    "case",
    "do",
    "else",
    "yield",
    "await",
    "default",
];

/// JavaScript's `\s`, which is not Rust's `char::is_whitespace` (it has U+FEFF, and not U+0085).
pub fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

pub fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

pub fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

fn blank(c: char) -> char {
    if c == '\n' { '\n' } else { ' ' }
}

/// One open template literal: in its text, or inside a `${ ... }` at `depth` braces.
struct Template {
    interp: bool,
    depth: u32,
}

pub fn strip_literals(source: &[char]) -> Result<Vec<char>, Unparseable> {
    let len = source.len();
    let at = |k: usize| source.get(k).copied().unwrap_or('\0');
    let mut out = vec![' '; len];
    let mut i = 0;
    let mut last_value = false;
    let mut templates: Vec<Template> = Vec::new();

    while i < len {
        if templates.last().is_some_and(|t| !t.interp) {
            let ch = source[i];
            if ch == '\\' {
                out[i] = blank(ch);
                i += 1;
                if i < len {
                    out[i] = blank(source[i]);
                    i += 1;
                }
                continue;
            }
            if ch == '`' {
                out[i] = ' ';
                templates.pop();
                i += 1;
                last_value = true;
                continue;
            }
            if ch == '$' && at(i + 1) == '{' {
                out[i] = '$';
                out[i + 1] = '{';
                if let Some(top) = templates.last_mut() {
                    top.interp = true;
                    top.depth = 0;
                }
                i += 2;
                last_value = false;
                continue;
            }
            out[i] = blank(ch);
            i += 1;
            continue;
        }

        let ch = source[i];

        // Line comment.
        if ch == '/' && at(i + 1) == '/' {
            while i < len && source[i] != '\n' {
                out[i] = ' ';
                i += 1;
            }
            continue;
        }

        // Block comment.
        if ch == '/' && at(i + 1) == '*' {
            out[i] = ' ';
            out[i + 1] = ' ';
            i += 2;
            let mut closed = false;
            while i < len {
                if source[i] == '*' && at(i + 1) == '/' {
                    out[i] = ' ';
                    out[i + 1] = ' ';
                    i += 2;
                    closed = true;
                    break;
                }
                out[i] = blank(source[i]);
                i += 1;
            }
            if !closed {
                return Err(Unparseable);
            }
            last_value = false;
            continue;
        }

        // Single or double quoted string. A raw newline before the closing quote is unterminated.
        if ch == '\'' || ch == '"' {
            let quote = ch;
            out[i] = ' ';
            i += 1;
            let mut closed = false;
            while i < len {
                let c = source[i];
                if c == '\n' {
                    break;
                }
                if c == '\\' {
                    out[i] = ' ';
                    i += 1;
                    if i < len && source[i] != '\n' {
                        out[i] = ' ';
                        i += 1;
                    }
                    continue;
                }
                if c == quote {
                    out[i] = ' ';
                    i += 1;
                    closed = true;
                    break;
                }
                out[i] = ' ';
                i += 1;
            }
            if !closed {
                return Err(Unparseable);
            }
            last_value = true;
            continue;
        }

        // Template literal open.
        if ch == '`' {
            out[i] = ' ';
            templates.push(Template {
                interp: false,
                depth: 0,
            });
            i += 1;
            continue;
        }

        // Regex literal candidate: only where the preceding token says an expression starts.
        if ch == '/' && !last_value {
            let start = i;
            out[i] = ' ';
            i += 1;
            let mut in_class = false;
            let mut closed = false;
            while i < len {
                let c = source[i];
                if c == '\n' {
                    break;
                }
                if c == '\\' {
                    out[i] = ' ';
                    i += 1;
                    if i < len && source[i] != '\n' {
                        out[i] = ' ';
                        i += 1;
                    }
                    continue;
                }
                if c == '[' {
                    in_class = true;
                    out[i] = ' ';
                    i += 1;
                    continue;
                }
                if c == ']' {
                    in_class = false;
                    out[i] = ' ';
                    i += 1;
                    continue;
                }
                if c == '/' && !in_class {
                    out[i] = ' ';
                    i += 1;
                    closed = true;
                    break;
                }
                out[i] = ' ';
                i += 1;
            }
            if closed {
                while i < len && source[i].is_ascii_alphabetic() {
                    out[i] = ' ';
                    i += 1;
                }
                last_value = true;
                continue;
            }
            // Never closed: it was a division. Scan again from there.
            i = start;
            out[i] = source[i];
            i += 1;
            last_value = false;
            continue;
        }

        // Template interpolation brace tracking (real code inside `${ ... }`).
        if let Some(top) = templates.last_mut().filter(|t| t.interp) {
            if ch == '{' {
                top.depth += 1;
                out[i] = ch;
                i += 1;
                last_value = false;
                continue;
            }
            if ch == '}' {
                if top.depth == 0 {
                    out[i] = '}';
                    top.interp = false;
                    i += 1;
                    last_value = false;
                    continue;
                }
                top.depth -= 1;
                out[i] = ch;
                i += 1;
                last_value = true;
                continue;
            }
        }

        // Identifier or keyword.
        if is_ident_start(ch) {
            let mut j = i;
            while j < len && is_ident(source[j]) {
                j += 1;
            }
            let word: String = source[i..j].iter().collect();
            out[i..j].copy_from_slice(&source[i..j]);
            last_value = !REGEX_KEYWORD_CONTEXT_WORDS.contains(&word.as_str());
            i = j;
            continue;
        }

        // Numeric literal.
        if ch.is_ascii_digit() {
            let mut j = i;
            while j < len && (source[j].is_ascii_hexdigit() || "xXoObB.".contains(source[j])) {
                j += 1;
            }
            out[i..j].copy_from_slice(&source[i..j]);
            last_value = true;
            i = j;
            continue;
        }

        if ch == ')' || ch == ']' {
            out[i] = ch;
            i += 1;
            last_value = true;
            continue;
        }

        if ch == '\n' {
            out[i] = '\n';
            i += 1;
            continue;
        }

        out[i] = ch;
        i += 1;
        if !is_space(ch) {
            last_value = false;
        }
    }

    if templates.is_empty() {
        Ok(out)
    } else {
        Err(Unparseable)
    }
}
