//! One file parsed and its query run: the matches with their spans, parents and captures, and the error count.

use specforge_protocol_types::{
    NotParsed, SyntaxCapture, SyntaxMatch, SyntaxOutcome, SyntaxPoint, SyntaxRole, SyntaxSpan,
};
use std::cell::Cell;
use std::ops::ControlFlow;
use std::time::{Duration, Instant};

use tree_sitter::{
    Node, ParseOptions, Parser, Point, QueryCursor, QueryCursorOptions, StreamingIterator as _,
    Tree,
};

use crate::SourceQuery;

/// The most bytes of a capture's text answered; past it the text is cut on a character boundary.
pub const CAPTURE_TEXT_CAP: usize = 4096;

/// What bounds one file's parse and query run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deadline {
    /// Wall-clock time, from the parse's start: production's.
    Wall(Duration),
    /// A count of tree-sitter's progress callbacks: counted work, what tests use so none reads a clock.
    Progress(u32),
}

/// Each file's deadline in production (ADR 0344 D3).
pub const PARSE_DEADLINE: Deadline = Deadline::Wall(Duration::from_secs(5));

/// A counted deadline no file a test writes reaches: what every test parses within, so none reads a clock (ADR 0344
/// D3).
pub const COUNTED_DEADLINE: Deadline = Deadline::Progress(1 << 20);

/// One file's deadline as it is spent across its parse and its query run.
struct Budget {
    deadline: Deadline,
    started: Instant,
    ticks: Cell<u32>,
    spent: Cell<bool>,
}

impl Budget {
    fn new(deadline: Deadline) -> Self {
        Budget {
            deadline,
            started: Instant::now(),
            ticks: Cell::new(0),
            spent: Cell::new(false),
        }
    }

    /// One progress callback: break once the deadline is passed.
    fn tick(&self) -> ControlFlow<()> {
        let ticks = self.ticks.get().saturating_add(1);
        self.ticks.set(ticks);
        let past = match self.deadline {
            Deadline::Wall(most) => self.started.elapsed() > most,
            Deadline::Progress(most) => ticks > most,
        };
        if past {
            self.spent.set(true);
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

/// A parser set to `query`'s language.
pub(crate) fn parser_for(query: &SourceQuery) -> Parser {
    let mut parser = Parser::new();
    parser.set_language(&query.language().tree_sitter()).expect(
        "a grammar compiled into this binary has the runtime's ABI: the suite parses with each",
    );
    parser
}

/// `source` parsed by `parser` (set to `query`'s language) and `query` run over its tree, both within `deadline`:
/// past it, the file is `not_parsed(deadline)`.
pub(crate) fn parse(
    parser: &mut Parser,
    query: &SourceQuery,
    source: &[u8],
    deadline: Deadline,
) -> Result<SyntaxOutcome, NotParsed> {
    let budget = Budget::new(deadline);
    let mut progress = |_: &tree_sitter::ParseState| budget.tick();
    let options = ParseOptions::new().progress_callback(&mut progress);
    let mut text = |at: usize, _: Point| source.get(at..).unwrap_or_default();
    let Some(tree) = parser.parse_with_options(&mut text, None, Some(options)) else {
        // A stopped parse resumes on the next call unless the parser is reset.
        parser.reset();
        return Err(if budget.spent.get() {
            NotParsed::Deadline
        } else {
            NotParsed::Unreadable
        });
    };
    let matches = matches(&tree, query, source, &budget);
    if budget.spent.get() {
        return Err(NotParsed::Deadline);
    }
    Ok(SyntaxOutcome::Parsed {
        matches,
        errors: errors(&tree),
    })
}

/// The tree's ERROR and MISSING nodes.
fn errors(tree: &Tree) -> u32 {
    let root = tree.root_node();
    if !root.has_error() {
        return 0;
    }
    let mut count = 0u32;
    let mut cursor = root.walk();
    loop {
        let node = cursor.node();
        if node.is_error() || node.is_missing() {
            count += 1;
        }
        // An ERROR node's subtree may hold more; a node without an error below has none to count.
        if node.has_error() && cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return count;
            }
        }
    }
}

/// One match before its parent is known: its pattern breaks a tie of spans.
struct Found {
    pattern: usize,
    start: usize,
    end: usize,
    matched: SyntaxMatch,
}

fn matches(tree: &Tree, query: &SourceQuery, source: &[u8], budget: &Budget) -> Vec<SyntaxMatch> {
    let names = query.query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut found = Vec::new();
    let mut progress = |_: &tree_sitter::QueryCursorState| budget.tick();
    let options = QueryCursorOptions::new().progress_callback(&mut progress);
    let mut each = cursor.matches_with_options(&query.query, tree.root_node(), source, options);
    while let Some(matched) = each.next() {
        let role = &query.roles[matched.pattern_index];
        let Some(node) = matched
            .captures
            .iter()
            .find(|capture| capture.index == role.capture)
            .map(|capture| capture.node)
        else {
            continue;
        };
        let captures = matched
            .captures
            .iter()
            .filter(|capture| query.answered[capture.index as usize])
            .map(|capture| {
                let (text, cut) = text_of(capture.node, source);
                SyntaxCapture {
                    name: names[capture.index as usize].to_string(),
                    text,
                    cut,
                    span: span_of(capture.node, source),
                }
            })
            .collect();
        found.push(Found {
            pattern: matched.pattern_index,
            start: node.start_byte(),
            end: node.end_byte(),
            matched: SyntaxMatch {
                role: role.role,
                kind: role.kind.clone(),
                span: span_of(node, source),
                parent: None,
                captures,
            },
        });
    }
    found.sort_by(|a, b| {
        (a.start, std::cmp::Reverse(a.end), a.pattern).cmp(&(
            b.start,
            std::cmp::Reverse(b.end),
            b.pattern,
        ))
    });
    parented(found)
}

/// Each match's parent: the innermost definition whose span holds it strictly, or with the same span and an earlier
/// pattern. The matches are sorted by (start, end descending, pattern), so a holder comes first; tree nodes nest or
/// are disjoint, so a definition that does not hold a match holds none after it.
fn parented(found: Vec<Found>) -> Vec<SyntaxMatch> {
    let mut open: Vec<usize> = Vec::new();
    let mut spans: Vec<(usize, usize, usize)> = Vec::with_capacity(found.len());
    let mut matches = Vec::with_capacity(found.len());
    for Found {
        pattern,
        start,
        end,
        mut matched,
    } in found
    {
        while let Some(&top) = open.last() {
            let (top_start, top_end, _) = spans[top];
            if top_start <= start && end <= top_end {
                break;
            }
            open.pop();
        }
        matched.parent = open
            .iter()
            .rev()
            .find(|&&holder| {
                let (holder_start, holder_end, holder_pattern) = spans[holder];
                !(holder_start == start && holder_end == end && holder_pattern >= pattern)
            })
            .map(|&holder| holder as u32);
        let index = matches.len();
        spans.push((start, end, pattern));
        if matched.role == SyntaxRole::Definition {
            open.push(index);
        }
        matches.push(matched);
    }
    matches
}

/// A node's text: UTF-8 (lossy), cut at [`CAPTURE_TEXT_CAP`] bytes on a character boundary.
fn text_of(node: Node<'_>, source: &[u8]) -> (String, bool) {
    let mut text = String::from_utf8_lossy(&source[node.byte_range()]).into_owned();
    let cut = text.len() > CAPTURE_TEXT_CAP;
    if cut {
        text.truncate(text.floor_char_boundary(CAPTURE_TEXT_CAP));
    }
    (text, cut)
}

fn span_of(node: Node<'_>, source: &[u8]) -> SyntaxSpan {
    SyntaxSpan {
        start: point(node.start_byte(), node.start_position(), source),
        end: point(node.end_byte(), node.end_position(), source),
    }
}

/// A place: the byte offset, the 1-based line, the column in characters (each byte that does not continue a UTF-8
/// sequence starts one) and the column in bytes.
fn point(byte: usize, at: Point, source: &[u8]) -> SyntaxPoint {
    let line_start = byte.saturating_sub(at.column);
    let chars = source[line_start..byte]
        .iter()
        .filter(|b| (**b & 0xC0) != 0x80)
        .count();
    SyntaxPoint {
        byte: clamp(byte),
        line: clamp(at.row + 1),
        col: clamp(chars + 1),
        byte_col: clamp(at.column + 1),
    }
}

fn clamp(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

impl SourceQuery {
    /// `source` parsed and this query run over it within `deadline` ([`PARSE_DEADLINE`] in production, a counted
    /// one in a test), outside any reader: no cache, no count.
    pub fn parse(&self, source: &[u8], deadline: Deadline) -> SyntaxOutcome {
        parse(&mut parser_for(self), self, source, deadline)
            .unwrap_or_else(|reason| SyntaxOutcome::NotParsed { reason })
    }
}
