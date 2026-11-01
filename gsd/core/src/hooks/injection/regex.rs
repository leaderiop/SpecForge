//! Ported from gsd-core v1.19.0 hooks/lib/injection-patterns.js and hooks/gsd-read-injection-scanner.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! A backtracking matcher for the JavaScript regular expressions of upstream's scanners, read from the very source
//! text they have there (`patterns::INJECTION_PATTERNS` pins each). The `regex` crate has no look-ahead, which the
//! scanners use, and the wasm guest has no other engine. Only what those patterns use is parsed: literals, `\s`, `^`,
//! classes of characters and `\s`, groups (`(?:`, `(`, `(?=`, `(?!`), `|`, and `?`, `+`, `*`; the `i` flag is ASCII
//! case folding, as JavaScript's is for these patterns. Anything else panics when the pattern is built, and every
//! pattern is built by the unit tests.

/// `\s` as JavaScript means it.
fn js_space(c: char) -> bool {
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

#[derive(Debug, Clone)]
enum Item {
    Ch(char),
    Space,
}

#[derive(Debug, Clone)]
enum Node {
    Char(char),
    Space,
    Class {
        neg: bool,
        items: Vec<Item>,
    },
    Start,
    Seq(Vec<Node>),
    Alt(Vec<Node>),
    Rep {
        node: Box<Node>,
        min: usize,
        max: usize,
    },
    Look {
        neg: bool,
        node: Box<Node>,
    },
}

#[cfg(test)]
thread_local! {
    /// The steps the matcher took on this thread: a test bounds its work by them, not by the clock.
    pub(super) static STEPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// A compiled pattern of the `i` flag.
#[derive(Debug, Clone)]
pub struct Regex {
    node: Node,
}

struct Parser<'a> {
    src: &'a [char],
    at: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.src.get(self.at).copied()
    }

    fn eat(&mut self, c: char) -> bool {
        let hit = self.peek() == Some(c);
        self.at += usize::from(hit);
        hit
    }

    fn alt(&mut self) -> Node {
        let mut alts = vec![self.seq()];
        while self.eat('|') {
            alts.push(self.seq());
        }
        if alts.len() == 1 {
            alts.remove(0)
        } else {
            Node::Alt(alts)
        }
    }

    fn seq(&mut self) -> Node {
        let mut items = Vec::new();
        while !matches!(self.peek(), None | Some('|' | ')')) {
            let atom = self.atom();
            items.push(self.quantified(atom));
        }
        Node::Seq(items)
    }

    fn quantified(&mut self, atom: Node) -> Node {
        let (min, max) = match self.peek() {
            Some('?') => (0, 1),
            Some('+') => (1, usize::MAX),
            Some('*') => (0, usize::MAX),
            _ => return atom,
        };
        self.at += 1;
        Node::Rep {
            node: Box::new(atom),
            min,
            max,
        }
    }

    fn atom(&mut self) -> Node {
        let c = self.peek().expect("an atom");
        self.at += 1;
        match c {
            '(' => {
                let mut look = None;
                if self.eat('?') {
                    let kind = self.peek().expect("a group kind");
                    self.at += 1;
                    look = match kind {
                        ':' => None,
                        '=' => Some(false),
                        '!' => Some(true),
                        other => panic!("unsupported group (?{other}"),
                    };
                }
                let inner = self.alt();
                assert!(self.eat(')'), "an unclosed group");
                match look {
                    Some(neg) => Node::Look {
                        neg,
                        node: Box::new(inner),
                    },
                    None => inner,
                }
            }
            '[' => {
                let neg = self.eat('^');
                let mut items = Vec::new();
                while !self.eat(']') {
                    let c = self.peek().expect("an unclosed class");
                    self.at += 1;
                    items.push(if c == '\\' {
                        let e = self.peek().expect("an escape");
                        self.at += 1;
                        if e == 's' { Item::Space } else { Item::Ch(e) }
                    } else {
                        Item::Ch(c)
                    });
                }
                Node::Class { neg, items }
            }
            '\\' => {
                let e = self.peek().expect("an escape");
                self.at += 1;
                assert!(
                    e == 's' || !e.is_ascii_alphanumeric(),
                    "unsupported escape \\{e}"
                );
                if e == 's' { Node::Space } else { Node::Char(e) }
            }
            '^' => Node::Start,
            '.' | '$' | '{' | '}' | '*' | '+' | '?' => panic!("unsupported '{c}'"),
            c => Node::Char(c),
        }
    }
}

impl Regex {
    /// Compiles `source`, the text of a JavaScript regular expression literal, for the `i` flag.
    ///
    /// # Panics
    ///
    /// On syntax this matcher does not support.
    pub fn new(source: &str) -> Regex {
        let chars: Vec<char> = source.chars().collect();
        let mut p = Parser { src: &chars, at: 0 };
        let node = p.alt();
        assert!(p.at == chars.len(), "unbalanced ')' in {source}");
        Regex { node }
    }

    fn same(a: char, b: char) -> bool {
        a == b || a.eq_ignore_ascii_case(&b)
    }

    /// Whether the one-character node `n` accepts `c`.
    fn one(n: &Node, c: char) -> Option<bool> {
        match n {
            Node::Char(x) => Some(Self::same(c, *x)),
            Node::Space => Some(js_space(c)),
            Node::Class { neg, items } => {
                let hit = items.iter().any(|i| match i {
                    Item::Ch(x) => Self::same(c, *x),
                    Item::Space => js_space(c),
                });
                Some(hit != *neg)
            }
            _ => None,
        }
    }

    fn go(&self, n: &Node, s: &[char], i: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        #[cfg(test)]
        STEPS.with(|n| n.set(n.get() + 1));
        match n {
            Node::Char(_) | Node::Space | Node::Class { .. } => {
                i < s.len() && Self::one(n, s[i]) == Some(true) && k(i + 1)
            }
            Node::Start => i == 0 && k(i),
            Node::Seq(v) => self.seq(v, s, i, k),
            Node::Alt(v) => {
                for a in v {
                    if self.go(a, s, i, k) {
                        return true;
                    }
                }
                false
            }
            Node::Rep { node, min, max } => {
                // A run of one-character atoms is counted, not recursed into: a megabyte of spaces is input.
                if Self::one(node, ' ').is_some() {
                    let mut run = 0;
                    while run < *max
                        && i + run < s.len()
                        && Self::one(node, s[i + run]) == Some(true)
                    {
                        run += 1;
                    }
                    return (*min..=run).rev().any(|take| k(i + take));
                }
                self.rep(node, (*min, *max), 0, s, i, k)
            }
            Node::Look { neg, node } => {
                let hit = self.go(node, s, i, &mut |_| true);
                hit != *neg && k(i)
            }
        }
    }

    fn seq(&self, v: &[Node], s: &[char], i: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        match v.split_first() {
            None => k(i),
            Some((head, tail)) => self.go(head, s, i, &mut |j| self.seq(tail, s, j, k)),
        }
    }

    fn rep(
        &self,
        n: &Node,
        (min, max): (usize, usize),
        count: usize,
        s: &[char],
        i: usize,
        k: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        if count < max
            && self.go(n, s, i, &mut |j| {
                (j != i || count < min) && self.rep(n, (min, max), count + 1, s, j, k)
            })
        {
            return true;
        }
        count >= min && k(i)
    }

    /// The first match in `text`: where it starts and ends, in characters.
    pub fn find(&self, text: &[char]) -> Option<(usize, usize)> {
        (0..=text.len()).find_map(|start| {
            let mut end = None;
            self.go(&self.node, text, start, &mut |e| {
                end = Some(e);
                true
            })
            .then(|| (start, end.unwrap_or(start)))
        })
    }

    pub fn is_match(&self, text: &[char]) -> bool {
        self.find(text).is_some()
    }

    /// The choice points of the pattern outside its look-aheads: one per `?`, `+`, `*` and extra alternative.
    #[cfg(test)]
    pub(super) fn splits(&self) -> usize {
        fn count(n: &Node) -> usize {
            match n {
                Node::Seq(v) => v.iter().map(count).sum(),
                Node::Alt(v) => v.len() - 1 + v.iter().map(count).sum::<usize>(),
                Node::Rep { node, .. } => 1 + count(node),
                _ => 0,
            }
        }
        count(&self.node)
    }
}
