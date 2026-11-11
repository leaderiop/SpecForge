//! What renames write in one text, as places (ADR 0346 D7): each a span of the text read, the name read there and the
//! name written. Two facts may hold names one inside the other (`v1/T1` read whole by one collector, its `T1` by
//! another): their places nest, and a rename moving what both name writes both. The outermost place of a nest is what
//! is written; a place nested in it must read, inside what the outer one writes, the name it writes, found by the text
//! after it (a respelled path keeps what follows the segment renamed) or else by the text before it. Places that cross
//! one another, or nested places whose names disagree, leave no one text: no rename writes them. [`Writes`] is the one
//! owner of how a text's places move: the derivation of what the renames wrote (`stale`) and a rename's plan read it.

/// A span of a text, by its start and end (line, byte column), 1-based, end exclusive.
pub type Span = (usize, usize, usize, usize);

/// A point of a text: (line, byte column).
type Point = (usize, usize);

fn start(span: &Span) -> Point {
    (span.0, span.1)
}

fn end(span: &Span) -> Point {
    (span.2, span.3)
}

/// One place written: its span of the text read, the name read there (`held`) and the name written (`now`, a name:
/// no newline in it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Write {
    pub span: Span,
    pub held: String,
    pub now: String,
}

/// A place nested in an outer one: its span, the outer one, and where its written name starts in what that one writes.
#[derive(Debug, Clone)]
struct Nested {
    span: Span,
    outer: usize,
    from: usize,
    len: usize,
}

/// Which part of the name read at a place written a point inside it lies in, kept as the name written keeps it: the
/// text after the point (the name written ends with it) or before it (the name written starts with it).
#[derive(Debug, Clone, Copy)]
enum Kept {
    After,
    Before,
}

/// What renames write in one text: its outermost places, which are what is written, and those nested in them.
#[derive(Debug, Clone, Default)]
pub struct Writes {
    /// In the order of the text, none overlapping another.
    outer: Vec<Write>,
    nested: Vec<Nested>,
}

impl Writes {
    /// The places `writes`, whatever their order: `None` when two of them cross, two share a span and write
    /// differently, or one nested in another does not read inside it the name it reads and write inside what it
    /// writes the name it writes.
    pub fn of(writes: impl IntoIterator<Item = Write>) -> Option<Writes> {
        let mut writes: Vec<Write> = writes.into_iter().collect();
        // By start, the longest first: an outer place comes before every place nested in it.
        writes.sort_by(|a, b| {
            start(&a.span)
                .cmp(&start(&b.span))
                .then(end(&b.span).cmp(&end(&a.span)))
        });
        let mut nest = Writes::default();
        for write in writes {
            match nest.outer.last() {
                Some(outer) if start(&write.span) < end(&outer.span) => {
                    if end(&write.span) > end(&outer.span) {
                        return None;
                    }
                    if write.span == outer.span {
                        if write != *outer {
                            return None;
                        }
                        continue;
                    }
                    let from = within(outer, &write)?;
                    nest.nested.push(Nested {
                        span: write.span,
                        outer: nest.outer.len() - 1,
                        from,
                        len: write.now.len(),
                    });
                }
                _ => nest.outer.push(write),
            }
        }
        Some(nest)
    }

    /// The outermost places: what is written, one edit each.
    pub fn outer(&self) -> &[Write] {
        &self.outer
    }

    /// Where the place written at `span` (outermost or nested) is in the text the writes leave; `None` for a span no
    /// place written has.
    pub fn written(&self, span: &Span) -> Option<Span> {
        let (outer, from, len) = match self.outer.iter().position(|w| w.span == *span) {
            Some(at) => (at, 0, self.outer[at].now.len()),
            None => {
                let nested = self.nested.iter().find(|n| n.span == *span)?;
                (nested.outer, nested.from, nested.len)
            }
        };
        let (line, col) = self.point(start(&self.outer[outer].span), Kept::Before)?;
        Some((line, col + from, line, col + from + len))
    }

    /// Where `span`, the place of something no place written is, lies in the text the writes leave; `None` when the
    /// writes change its text: a place holding a name (`named`) that a write inside it changes, or one whose start (or
    /// end) is inside a place written whose name written does not keep what follows (or precedes) it there.
    pub fn place(&self, span: Span, named: bool) -> Option<Span> {
        let (from, to) = (start(&span), end(&span));
        let inside = |at: Point| {
            self.outer
                .iter()
                .position(|w| start(&w.span) < at && at < end(&w.span))
        };
        let (from, to) = match (inside(from), inside(to)) {
            // Wholly inside one place written: kept as the text after it, or as the text before it.
            (Some(a), Some(b)) if a == b => [Kept::After, Kept::Before]
                .into_iter()
                .find_map(|kept| Some((self.point(from, kept)?, self.point(to, kept)?)))?,
            _ => {
                let changed =
                    |w: &Write| from <= start(&w.span) && end(&w.span) <= to && w.held != w.now;
                if named && self.outer.iter().any(changed) {
                    return None;
                }
                (
                    self.point(from, Kept::After)?,
                    self.point(to, Kept::Before)?,
                )
            }
        };
        Some((from.0, from.1, to.0, to.1))
    }

    /// The point `at` of the text read in the text the writes leave: unchanged before a place written, carried by the
    /// length a write changes after it, and inside one where the name written keeps the text `kept` (`None` when it
    /// does not). The places are taken from the last: each moves only what lies after it.
    fn point(&self, at: Point, kept: Kept) -> Option<Point> {
        let (mut line, mut col) = at;
        for write in self.outer.iter().rev() {
            let (from, to) = (start(&write.span), end(&write.span));
            if (line, col) <= from {
                continue;
            }
            if (line, col) < to {
                let at = offset(write, (line, col))?;
                let into = match kept {
                    Kept::After => {
                        let after = write.held.get(at..)?;
                        write.now.strip_suffix(after)?.len()
                    }
                    Kept::Before => {
                        let before = write.held.get(..at)?;
                        write.now.strip_prefix(before)?;
                        at
                    }
                };
                (line, col) = (from.0, from.1 + into);
            } else if line == to.0 {
                (line, col) = (from.0, from.1 + write.now.len() + (col - to.1));
            } else {
                line -= to.0 - from.0;
            }
        }
        Some((line, col))
    }
}

/// Where the name `inner` writes starts in what `outer`, the place it is nested in, writes: by the text after it
/// (`outer`'s name written ends with it, after `inner`'s), else by the text before it; `None` when neither, or when
/// `outer` does not read at `inner`'s place the name `inner` reads.
fn within(outer: &Write, inner: &Write) -> Option<usize> {
    let (from, to) = (
        offset(outer, start(&inner.span))?,
        offset(outer, end(&inner.span))?,
    );
    if outer.held.get(from..to)? != inner.held {
        return None;
    }
    let after = outer.held.get(to..)?;
    if let Some(lead) = outer.now.strip_suffix(after)
        && lead.ends_with(&inner.now)
    {
        return Some(lead.len() - inner.now.len());
    }
    let before = outer.held.get(..from)?;
    let rest = outer.now.strip_prefix(before)?;
    rest.starts_with(&inner.now).then_some(before.len())
}

/// The byte offset of the point `at` inside the name `write` reads, from its start.
fn offset(write: &Write, at: Point) -> Option<usize> {
    let (line, col) = start(&write.span);
    if at.0 == line {
        return at.1.checked_sub(col);
    }
    let down = at.0.checked_sub(line)?;
    let starts = write.held.match_indices('\n').nth(down - 1)?.0 + 1;
    Some(starts + at.1.checked_sub(1)?)
}

#[cfg(test)]
mod tests;
