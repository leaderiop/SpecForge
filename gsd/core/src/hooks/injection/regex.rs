//! Ported from gsd-core v1.19.0 hooks/lib/injection-patterns.js and hooks/gsd-read-injection-scanner.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! A backtracking matcher for the JavaScript regular expressions of upstream's scanners, read from the very source
//! text they have there (`patterns::INJECTION_PATTERNS` pins each). The `regex` crate has no look-ahead, which the
//! scanners use, and the wasm guest has no other engine. Only what those patterns use is parsed: literals, `\s`, `^`,
//! classes of characters and `\s`, groups (`(?:`, `(`, `(?=`, `(?!`), `|`, and `?`, `+`, `*`; the `i` flag is ASCII
//! case folding, as JavaScript's is for these patterns. Anything else panics when the pattern is built, and every
//! pattern is built by the unit tests.
//!
//! The work is linear in the text, so no text can run a hook past its budget. Each pattern compiles to a small program
//! that a backtracker runs in JavaScript's leftmost-first order, with a visited bit per (split, position), RE2's
//! BitState. That order is JavaScript's only where no repeated group can match empty: JavaScript refuses an empty
//! iteration once the minimum is met (so `(?:|a)?` takes the `a`), and the visited bit would prune the retry instead.
//! No shipped pattern has one, and building one panics. The set is kept across start positions: nothing captures or refers back, so a state that failed fails
//! from any start. Each split and position is then expanded once, at most splits x (n + 1) times, with as many bits.
//! A start whose character none of the pattern's first characters accepts is skipped without a run. A `+` or `*` of one
//! character is one instruction that takes its run longest first, with one frame for all the shorter takes, and the
//! bitsets and stack are kept between searches, so a 4 MiB run or a text of empty lines allocates once.
//! A look-ahead's body has no `+` or `*` (asserted when built), and its answer is kept per position.

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
pub(super) enum Item {
    Ch(char),
    Space,
}

/// A parsed pattern, shared by the matcher and the test oracle of its differential test.
#[derive(Debug, Clone)]
pub(super) enum Node {
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
    /// The pattern's program first, then its look-ahead bodies.
    progs: Vec<Prog>,
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

/// The tree of `source`, the text of a JavaScript regular expression literal.
///
/// # Panics
///
/// On syntax this matcher does not support.
pub(super) fn parse(source: &str) -> Node {
    let chars: Vec<char> = source.chars().collect();
    let mut p = Parser { src: &chars, at: 0 };
    let node = p.alt();
    assert!(p.at == chars.len(), "unbalanced ')' in {source}");
    node
}

/// One instruction of a compiled pattern.
#[derive(Debug, Clone)]
enum Inst {
    /// A character the one-character node (`Char`, `Space` or `Class`) accepts.
    One(Node),
    Start,
    Match,
    /// Go on at `x`, and at `y` if that fails; `ord` numbers the splits of a program, the rows of the visited set.
    Split {
        x: usize,
        y: usize,
        ord: usize,
    },
    Jmp(usize),
    /// A run of `+` (`min` 1) or `*` (`min` 0) over a one-character node, taken longest first: one frame holds every
    /// shorter take, where a split per character would push a frame per character. `ord` is its row of the visited
    /// set, a bit per position it went on from.
    Star {
        node: Node,
        min: usize,
        ord: usize,
    },
    /// Whether program `prog` matches here (or does not, with `neg`); it consumes nothing.
    Look {
        neg: bool,
        prog: usize,
    },
}

#[derive(Debug, Clone, Default)]
struct Prog {
    insts: Vec<Inst>,
    splits: usize,
    /// The one-character nodes a match must start with; none when a match can start without a character.
    first: Option<Vec<Node>>,
    /// The ASCII characters of those nodes, a bit each.
    ascii: u128,
}

impl Prog {
    /// Whether a match can start at `at` of `s`, by its first character.
    fn may_start(&self, s: &[char], at: usize) -> bool {
        let Some(first) = &self.first else {
            return true;
        };
        match s.get(at) {
            None => false,
            Some(&c) if c.is_ascii() => self.ascii >> (c as u32) & 1 == 1,
            Some(&c) => first.iter().any(|n| Regex::one(n, c) == Some(true)),
        }
    }
}

/// The one-character nodes reachable from the first instruction before any is consumed; none if `Match` is.
fn firsts(insts: &[Inst]) -> Option<Vec<Node>> {
    let (mut out, mut todo, mut seen) = (Vec::new(), vec![0], vec![false; insts.len()]);
    while let Some(pc) = todo.pop() {
        if std::mem::replace(&mut seen[pc], true) {
            continue;
        }
        match &insts[pc] {
            Inst::One(n) => out.push(n.clone()),
            Inst::Star { node, min, .. } => {
                out.push(node.clone());
                if *min == 0 {
                    todo.push(pc + 1);
                }
            }
            Inst::Match => return None,
            Inst::Split { x, y, .. } => todo.extend([*x, *y]),
            Inst::Jmp(x) => todo.push(*x),
            // Both only restrict where a match may start.
            Inst::Start | Inst::Look { .. } => todo.push(pc + 1),
        }
    }
    Some(out)
}

/// Appends a split to `x` whose `y` [`to_here`] fills in later, and returns where it is.
fn split(p: &mut Prog, x: usize) -> usize {
    let (at, ord) = (p.insts.len(), p.splits);
    p.splits += 1;
    p.insts.push(Inst::Split { x, y: 0, ord });
    at
}

/// Points the split or jump at `at` to the next instruction.
fn to_here(p: &mut Prog, at: usize) {
    let next = p.insts.len();
    match &mut p.insts[at] {
        Inst::Split { y, .. } => *y = next,
        Inst::Jmp(x) => *x = next,
        _ => unreachable!("only a split or a jump is patched"),
    }
}

/// Compiles `node` into a new program of `progs`, with its look-aheads after it, and returns its index. A `bounded`
/// program, a look-ahead's body, has no `+` or `*`.
fn compile(node: &Node, bounded: bool, progs: &mut Vec<Prog>) -> usize {
    let id = progs.len();
    progs.push(Prog::default());
    let mut p = Prog::default();
    emit(node, bounded, &mut p, progs);
    p.insts.push(Inst::Match);
    p.first = firsts(&p.insts);
    p.ascii = (0..128u8)
        .filter(|b| {
            p.first
                .iter()
                .flatten()
                .any(|n| Regex::one(n, char::from(*b)) == Some(true))
        })
        .fold(0, |m, b| m | 1 << b);
    progs[id] = p;
    id
}

/// Whether `n` can match without consuming a character.
fn nullable(n: &Node) -> bool {
    match n {
        Node::Char(_) | Node::Space | Node::Class { .. } => false,
        Node::Start | Node::Look { .. } => true,
        Node::Seq(v) => v.iter().all(nullable),
        Node::Alt(v) => v.iter().any(nullable),
        Node::Rep { node, min, .. } => *min == 0 || nullable(node),
    }
}

fn emit(n: &Node, bounded: bool, p: &mut Prog, progs: &mut Vec<Prog>) {
    match n {
        Node::Char(_) | Node::Space | Node::Class { .. } => p.insts.push(Inst::One(n.clone())),
        Node::Start => p.insts.push(Inst::Start),
        Node::Seq(v) => v.iter().for_each(|x| emit(x, bounded, p, progs)),
        Node::Alt(v) => {
            let (last, rest) = v.split_last().expect("an alternative");
            let mut ends = Vec::new();
            for a in rest {
                let s = split(p, p.insts.len() + 1);
                emit(a, bounded, p, progs);
                ends.push(p.insts.len());
                p.insts.push(Inst::Jmp(0));
                to_here(p, s);
            }
            emit(last, bounded, p, progs);
            ends.into_iter().for_each(|e| to_here(p, e));
        }
        Node::Rep { node, min, max } => {
            assert!(!bounded || *max == 1, "a + or * inside a look-ahead");
            assert!(!nullable(node), "a repeated group that can match empty");
            let top = p.insts.len();
            if *max == usize::MAX && Regex::one(node, ' ').is_some() {
                p.insts.push(Inst::Star {
                    node: (**node).clone(),
                    min: *min,
                    ord: p.splits,
                });
                p.splits += 1;
                return;
            }
            match (*min, *max) {
                (0, 1) => {
                    let s = split(p, top + 1);
                    emit(node, bounded, p, progs);
                    to_here(p, s);
                }
                (1, usize::MAX) => {
                    emit(node, bounded, p, progs);
                    let s = split(p, top);
                    to_here(p, s);
                }
                (0, usize::MAX) => {
                    let s = split(p, top + 1);
                    emit(node, bounded, p, progs);
                    p.insts.push(Inst::Jmp(top));
                    to_here(p, s);
                }
                other => unreachable!("the parser makes only ?, + and *, not {other:?}"),
            }
        }
        Node::Look { neg, node } => {
            let prog = compile(node, true, progs);
            p.insts.push(Inst::Look { neg: *neg, prog });
        }
    }
}

/// Sets bit `k` of `bits` and says whether it was set already.
fn mark(bits: &mut [u64], k: usize) -> bool {
    let seen = is_set(bits, k);
    bits[k / 64] |= 1 << (k % 64);
    seen
}

fn is_set(bits: &[u64], k: usize) -> bool {
    bits[k / 64] & (1 << (k % 64)) != 0
}

/// A state still to try.
#[derive(Debug, Clone, Copy)]
enum Frame {
    /// An instruction at a position.
    At(usize, usize),
    /// The star at `pc`, to go on from positions `hi` down to `lo`.
    Takes { pc: usize, lo: usize, hi: usize },
}

thread_local! {
    /// The visited set, the look-ahead answers and the stack of a search, kept between searches: a scan searches once
    /// per pattern and, for the link rules, once per line.
    static SCRATCH: std::cell::RefCell<(Vec<u64>, Vec<u64>, Vec<Frame>)> = std::cell::RefCell::default();
}

/// One search of a text.
struct Run<'a> {
    progs: &'a [Prog],
    s: &'a [char],
    /// The pattern's (split, position) states already tried, a row of `s.len() + 1` bits per split. Each one failed,
    /// whatever the start: nothing captures, so where a match began does not change what follows a state.
    seen: &'a mut [u64],
    /// Per (look-ahead, position), two bits: worked out, and matched.
    looks: &'a mut [u64],
    /// The states still to try, each search above the base it found.
    stack: &'a mut Vec<Frame>,
}

impl Run<'_> {
    /// Where program `prog`, started at `start`, first reaches `Match`, in JavaScript's leftmost-first order.
    fn run(&mut self, prog: usize, start: usize) -> Option<usize> {
        let (progs, s) = (self.progs, self.s);
        let (cells, base) = (s.len() + 1, self.stack.len());
        self.stack.push(Frame::At(0, start));
        let found = 'search: loop {
            if self.stack.len() == base {
                break None;
            }
            let (mut pc, mut i) = match self.stack.pop().expect("a state above the base") {
                Frame::At(pc, i) => (pc, i),
                Frame::Takes { pc, lo, mut hi } => {
                    let Inst::Star { ord, .. } = &progs[prog].insts[pc] else {
                        unreachable!("only a star leaves takes")
                    };
                    // A take another entry into the same run went on from already is passed over.
                    while hi > lo && is_set(self.seen, ord * cells + hi) {
                        #[cfg(test)]
                        STEPS.with(|n| n.set(n.get() + 1));
                        hi -= 1;
                    }
                    if mark(self.seen, ord * cells + hi) {
                        continue;
                    }
                    if hi > lo {
                        self.stack.push(Frame::Takes { pc, lo, hi: hi - 1 });
                    }
                    (pc + 1, hi)
                }
            };
            loop {
                #[cfg(test)]
                STEPS.with(|n| n.set(n.get() + 1));
                match &progs[prog].insts[pc] {
                    Inst::One(n) if i < s.len() && Regex::one(n, s[i]) == Some(true) => {
                        (pc, i) = (pc + 1, i + 1);
                    }
                    Inst::Start if i == 0 => pc += 1,
                    Inst::One(_) | Inst::Start => break,
                    Inst::Match => break 'search Some(i),
                    Inst::Jmp(x) => pc = *x,
                    Inst::Split { x, y, ord } => {
                        // A look-ahead body has no loop, and its runs, one per position, share no states.
                        if prog == 0 && mark(self.seen, ord * cells + i) {
                            break;
                        }
                        self.stack.push(Frame::At(*y, i));
                        pc = *x;
                    }
                    Inst::Star { node, min, ord } => {
                        // The run ends where a take was gone on from already: every longer take was then tried too,
                        // since the takes of a run are tried from its end down.
                        let mut end = i;
                        while end < s.len()
                            && Regex::one(node, s[end]) == Some(true)
                            && !is_set(self.seen, ord * cells + end + 1)
                        {
                            #[cfg(test)]
                            STEPS.with(|n| n.set(n.get() + 1));
                            end += 1;
                        }
                        if end < i + min || mark(self.seen, ord * cells + end) {
                            break;
                        }
                        if end > i + min {
                            self.stack.push(Frame::Takes {
                                pc,
                                lo: i + min,
                                hi: end - 1,
                            });
                        }
                        (pc, i) = (pc + 1, end);
                    }
                    Inst::Look { neg, prog: body } => {
                        if self.look(*body, i) == *neg {
                            break;
                        }
                        pc += 1;
                    }
                }
            }
        };
        self.stack.truncate(base);
        found
    }

    /// Whether look-ahead body `body` matches at `i`, worked out once per position.
    fn look(&mut self, body: usize, i: usize) -> bool {
        let k = 2 * ((body - 1) * (self.s.len() + 1) + i);
        if !mark(self.looks, k)
            && self.progs[body].may_start(self.s, i)
            && self.run(body, i).is_some()
        {
            mark(self.looks, k + 1);
        }
        is_set(self.looks, k + 1)
    }
}

impl Regex {
    /// Compiles `source`, the text of a JavaScript regular expression literal, for the `i` flag.
    ///
    /// # Panics
    ///
    /// On syntax this matcher does not support, and on a `+` or `*` inside a look-ahead.
    pub fn new(source: &str) -> Regex {
        let mut progs = Vec::new();
        compile(&parse(source), false, &mut progs);
        Regex { progs }
    }

    fn same(a: char, b: char) -> bool {
        a == b || a.eq_ignore_ascii_case(&b)
    }

    /// Whether the one-character node `n` accepts `c`.
    pub(super) fn one(n: &Node, c: char) -> Option<bool> {
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

    /// The bytes of the visited set a search of `chars` characters allocates.
    #[cfg(test)]
    pub(super) fn seen_bytes(&self, chars: usize) -> usize {
        (self.progs[0].splits * (chars + 1)).div_ceil(64) * 8
    }

    /// The first match in `text`: where it starts and ends, in characters.
    pub fn find(&self, text: &[char]) -> Option<(usize, usize)> {
        let cells = text.len() + 1;
        SCRATCH.with_borrow_mut(|(seen, looks, stack)| {
            for (bits, n) in [
                (&mut *seen, self.progs[0].splits * cells),
                (&mut *looks, 2 * (self.progs.len() - 1) * cells),
            ] {
                bits.clear();
                bits.resize(n.div_ceil(64), 0);
            }
            stack.clear();
            let mut run = Run {
                progs: &self.progs,
                s: text,
                seen,
                looks,
                stack,
            };
            // A start whose character no first node accepts cannot begin a match: it is skipped unrun.
            let may_start = |at: &usize| {
                #[cfg(test)]
                STEPS.with(|n| n.set(n.get() + 1));
                self.progs[0].may_start(text, *at)
            };
            (0..cells)
                .filter(may_start)
                .find_map(|start| run.run(0, start).map(|end| (start, end)))
        })
    }

    pub fn is_match(&self, text: &[char]) -> bool {
        self.find(text).is_some()
    }

    /// The splits of the pattern outside its look-aheads: one per `?`, `+`, `*` and extra alternative.
    #[cfg(test)]
    pub(super) fn splits(&self) -> usize {
        self.progs[0].splits
    }
}
