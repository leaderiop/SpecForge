//! Ported from gsd-core v1.18.0 src/complexity-trigger.cts (`scanFunctions`), MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! Scores each function of a stripped source (see `strip`): 1 for the function, +1 for each `if`, `for`, `while`,
//! `do`, `case`, `catch`, `&&`, `||` and `?:` inside it and not inside a function nested in it. `?.`, `??`, a bare
//! `else` and `default:` count nothing.

use super::FunctionScore;
use super::strip::{is_ident, is_ident_start, is_space};

const DECISION_KEYWORDS: &[&str] = &["if", "for", "while", "do", "case", "catch"];
/// Words that can precede `(...) {` without defining a method: `if (x) {`, `switch (x) {` ...
const NON_METHOD_WORDS: &[&str] = &[
    "if", "for", "while", "do", "case", "catch", "switch", "with",
];

/// `s[k]`, or NUL past the end: no character the scanner looks for.
fn at(s: &[char], k: usize) -> char {
    s.get(k).copied().unwrap_or('\0')
}

fn skip_space(s: &[char], mut k: usize) -> usize {
    while k < s.len() && is_space(s[k]) {
        k += 1;
    }
    k
}

fn find_matching_paren(s: &[char], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (k, c) in s.iter().enumerate().skip(open) {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}

/// Skips a generic type-parameter list at `s[idx] == '<'` (`<T>`, `<T extends X>`, nested): the index past the
/// matching `>`, or `None` when the content is not shaped like one (a `<` used as less-than is not).
fn skip_generic_param_list(s: &[char], idx: usize) -> Option<usize> {
    let len = s.len();
    let mut i = idx + 1;
    let mut stack = vec!['>'];
    while i < len {
        let (ch, next) = (s[i], at(s, i + 1));
        if ch == '=' && next == '>' {
            i += 2;
            continue;
        }
        if (ch == '=' && next == '=')
            || (ch == '<' && next == '=')
            || (ch == '>' && next == '=')
            || (ch == '&' && next == '&')
            || (ch == '|' && next == '|')
            || "+*%!?;/-".contains(ch)
        {
            return None;
        }
        match ch {
            '<' => stack.push('>'),
            '(' => stack.push(')'),
            '[' => stack.push(']'),
            '{' => stack.push('}'),
            '>' | ')' | ']' | '}' => {
                if stack.last() != Some(&ch) {
                    return None;
                }
                stack.pop();
                if stack.is_empty() {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Skips a return-type annotation at `s[idx] == ':'`: the index of the terminator that follows (a body `{`, an
/// arrow `=>` or a `;`), never consuming it, or `None` when the content is not shaped like a type.
fn skip_type_expr(s: &[char], idx: usize) -> Option<usize> {
    let len = s.len();
    let mut i = idx + 1;
    let mut stack: Vec<char> = Vec::new();
    let mut expect_start = true;
    while i < len {
        let ch = s[i];
        if is_space(ch) {
            i += 1;
            continue;
        }
        let next = at(s, i + 1);
        if stack.is_empty() {
            if ch == ';' || (ch == '=' && next == '>') {
                return Some(i);
            }
            if (ch == '{' || ch == ',') && !expect_start {
                return Some(i);
            }
            if ch == '?' {
                return None;
            }
        }
        if ch == '=' && next == '>' {
            i += 2;
            expect_start = true;
            continue;
        }
        if (ch == '=' && next == '=')
            || (ch == '<' && next == '=')
            || (ch == '>' && next == '=')
            || (ch == '&' && next == '&')
            || (ch == '|' && next == '|')
            || "+*%!".contains(ch)
        {
            return None;
        }
        match ch {
            '<' | '(' | '[' | '{' => {
                stack.push(match ch {
                    '<' => '>',
                    '(' => ')',
                    '[' => ']',
                    _ => '}',
                });
                i += 1;
                expect_start = true;
                continue;
            }
            '>' | ')' | ']' | '}' => {
                if stack.last() != Some(&ch) {
                    return None;
                }
                stack.pop();
                i += 1;
                expect_start = false;
                continue;
            }
            '|' | '&' | ',' | '.' | ':' => {
                i += 1;
                expect_start = true;
                continue;
            }
            _ => {}
        }
        if is_ident(ch) {
            while i < len && is_ident(s[i]) {
                i += 1;
            }
            expect_start = false;
            continue;
        }
        i += 1;
    }
    None
}

fn newlines_between(s: &[char], from: usize, to: usize) -> usize {
    s[from..to].iter().filter(|c| **c == '\n').count()
}

/// Best-effort name of an arrow function from the `NAME = ` or `NAME: ` just before it.
fn infer_arrow_name(s: &[char], pos: usize) -> String {
    let mut j = pos as i64 - 1;
    let get = |k: i64| if k >= 0 { at(s, k as usize) } else { '\0' };
    while j >= 0 && is_space(get(j)) {
        j -= 1;
    }
    let plain_equals = j >= 0 && get(j) == '=' && !matches!(get(j - 1), '=' | '!' | '<' | '>');
    let prop_colon = j >= 0 && get(j) == ':';
    if !plain_equals && !prop_colon {
        return String::new();
    }
    j -= 1;
    while j >= 0 && is_space(get(j)) {
        j -= 1;
    }
    let end = j + 1;
    while j >= 0 && is_ident(get(j)) {
        j -= 1;
    }
    s[(j + 1) as usize..end as usize].iter().collect()
}

enum Frame {
    Block,
    Function {
        name: String,
        start_line: usize,
        score: u32,
        /// `Some(depth)` for an expression-bodied arrow, closed at that bracket depth.
        entry_depth: Option<i64>,
    },
}

struct Scan<'a> {
    s: &'a [char],
    line: usize,
    depth: i64,
    stack: Vec<Frame>,
    results: Vec<FunctionScore>,
}

impl Scan<'_> {
    fn bump(&mut self) {
        for frame in self.stack.iter_mut().rev() {
            if let Frame::Function { score, .. } = frame {
                *score += 1;
                return;
            }
        }
    }

    fn close_expr_frames(&mut self, at_depth: i64) {
        while let Some(Frame::Function {
            entry_depth: Some(entry),
            ..
        }) = self.stack.last()
        {
            if *entry != at_depth {
                break;
            }
            if let Some(Frame::Function {
                name,
                start_line,
                score,
                ..
            }) = self.stack.pop()
            {
                self.results.push(FunctionScore {
                    name,
                    start_line,
                    end_line: self.line,
                    score,
                });
            }
        }
    }

    /// Opens the function whose `(params)` closes at `close` and whose arrow `=>` is at `arrow`; the scan resumes in
    /// its body. `from` is where its name or parameters start.
    fn open_arrow(&mut self, name: String, start_line: usize, from: usize, arrow: usize) -> usize {
        let len = self.s.len();
        let mut body = arrow + 2;
        while body < len && is_space(self.s[body]) {
            body += 1;
        }
        self.line += newlines_between(self.s, from, body);
        if at(self.s, body) == '{' {
            self.stack.push(Frame::Function {
                name,
                start_line,
                score: 1,
                entry_depth: None,
            });
            self.depth += 1;
            return body + 1;
        }
        self.stack.push(Frame::Function {
            name,
            start_line,
            score: 1,
            entry_depth: Some(self.depth),
        });
        body
    }

    /// After `(`...`)` at `close`: an optional `: Type`, then the index of what follows.
    fn after_params(&self, close: usize) -> usize {
        let s = self.s;
        let mut p = skip_space(s, close + 1);
        if at(s, p) == ':'
            && let Some(after) = skip_type_expr(s, p)
        {
            p = skip_space(s, after);
        }
        p
    }

    /// At `k` after a name: skips `<T>` generics when a `(` follows them.
    fn skip_generics(&self, mut k: usize) -> usize {
        let s = self.s;
        if at(s, k) == '<'
            && let Some(after) = skip_generic_param_list(s, k)
        {
            let kk = skip_space(s, after);
            if at(s, kk) == '(' {
                k = kk;
            }
        }
        k
    }
}

pub fn scan_functions(s: &[char]) -> Vec<FunctionScore> {
    let len = s.len();
    let mut scan = Scan {
        s,
        line: 1,
        depth: 0,
        stack: Vec::new(),
        results: Vec::new(),
    };
    let mut i = 0;

    while i < len {
        let ch = s[i];

        if ch == '\n' {
            scan.line += 1;
            i += 1;
            continue;
        }
        if is_space(ch) {
            i += 1;
            continue;
        }

        if (ch == '&' && at(s, i + 1) == '&') || (ch == '|' && at(s, i + 1) == '|') {
            scan.bump();
            i += 2;
            continue;
        }
        if ch == '?' {
            if matches!(at(s, i + 1), '.' | '?') {
                i += 2;
                continue;
            }
            scan.bump();
            i += 1;
            continue;
        }

        if ch == '(' {
            if let Some(close) = find_matching_paren(s, i) {
                let k = scan.after_params(close);
                if at(s, k) == '=' && at(s, k + 1) == '>' {
                    let name = infer_arrow_name(s, i);
                    let start_line = scan.line;
                    i = scan.open_arrow(name, start_line, i, k);
                    continue;
                }
            }
            scan.depth += 1;
            i += 1;
            continue;
        }
        if ch == '[' {
            scan.depth += 1;
            i += 1;
            continue;
        }
        if ch == ')' || ch == ']' {
            scan.close_expr_frames(scan.depth);
            scan.depth -= 1;
            i += 1;
            continue;
        }
        if ch == '{' {
            scan.stack.push(Frame::Block);
            scan.depth += 1;
            i += 1;
            continue;
        }
        if ch == '}' {
            scan.close_expr_frames(scan.depth);
            scan.depth -= 1;
            if let Some(Frame::Function {
                name,
                start_line,
                score,
                entry_depth: None,
            }) = scan.stack.pop()
            {
                scan.results.push(FunctionScore {
                    name,
                    start_line,
                    end_line: scan.line,
                    score,
                });
            }
            i += 1;
            continue;
        }
        if ch == ';' || ch == ',' {
            scan.close_expr_frames(scan.depth);
            i += 1;
            continue;
        }

        if is_ident_start(ch) {
            let mut j = i;
            while j < len && is_ident(s[j]) {
                j += 1;
            }
            let word: String = s[i..j].iter().collect();
            let word_line = scan.line;

            if word == "function" {
                let mut k = skip_space(s, j);
                if at(s, k) == '*' {
                    k = skip_space(s, k + 1);
                }
                let mut name = String::new();
                if k < len && is_ident_start(s[k]) {
                    let mut m = k;
                    while m < len && is_ident(s[m]) {
                        m += 1;
                    }
                    name = s[k..m].iter().collect();
                    k = skip_space(s, m);
                }
                k = scan.skip_generics(k);
                if at(s, k) == '('
                    && let Some(close) = find_matching_paren(s, k)
                {
                    let p = scan.after_params(close);
                    if at(s, p) == '{' {
                        scan.line += newlines_between(s, i, p);
                        scan.stack.push(Frame::Function {
                            name,
                            start_line: word_line,
                            score: 1,
                            entry_depth: None,
                        });
                        scan.depth += 1;
                        i = p + 1;
                        continue;
                    }
                }
                i = j;
                continue;
            }

            if NON_METHOD_WORDS.contains(&word.as_str()) {
                if DECISION_KEYWORDS.contains(&word.as_str()) {
                    scan.bump();
                }
                i = j;
                continue;
            }

            let k = scan.skip_generics(skip_space(s, j));
            if at(s, k) == '(' {
                if let Some(close) = find_matching_paren(s, k)
                    && at(s, scan.after_params(close)) == '{'
                {
                    let p = scan.after_params(close);
                    scan.line += newlines_between(s, i, p);
                    scan.stack.push(Frame::Function {
                        name: word,
                        start_line: word_line,
                        score: 1,
                        entry_depth: None,
                    });
                    scan.depth += 1;
                    i = p + 1;
                    continue;
                }
            } else if at(s, k) == '=' && at(s, k + 1) == '>' {
                let name = infer_arrow_name(s, i);
                i = scan.open_arrow(name, word_line, i, k);
                continue;
            }

            i = j;
            continue;
        }

        i += 1;
    }

    scan.close_expr_frames(scan.depth);
    scan.results
}
