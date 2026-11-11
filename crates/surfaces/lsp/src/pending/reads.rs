//! Whether a file's text still reads a name where an earlier text of it read it (ADR 0346 D7): each place of the
//! earlier text carried into the text now through what changed between them, each read with what is beside it, so a
//! name grown or cut at its edge is not taken for it. Edits elsewhere (another rename, typing, even beside the name
//! with what cannot extend it) move a place and leave what it reads.
//!
//! The work is counted, never timed: the lines no other line of either text repeats anchor the two texts (the longest
//! run of them in the same order in both), the lines both share at the edges of the stretch between two anchors are
//! trimmed, and only a stretch left holding a place is compared byte by byte, when it is at most [`HUNK`] bytes; a
//! place in a larger one reads as gone. So a keystroke costs a pass over each text and a bounded byte diff.

use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Range;

use similar::DiffTag;
use similar::algorithms::{Capture, myers};
use specforge_project::facts::Span;

/// The most bytes (of both texts) a stretch of changed lines holding a place may have to be compared byte by byte:
/// the byte diff compares at most about its square.
const HUNK: usize = 1024;

/// Whether `now` reads `name` at each of `spans`, places of `then` that read it there.
pub(super) fn reads(then: &str, now: &str, spans: &[Span], name: &str) -> bool {
    counted(then, now, spans, name, &Cell::new(0))
}

/// [`reads`], adding to `work` each byte it compares.
fn counted(then: &str, now: &str, spans: &[Span], name: &str, work: &Cell<usize>) -> bool {
    let Some(ranges) = spans
        .iter()
        .map(|span| range(then, span))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    if ranges.is_empty() || ranges.iter().any(|r| then.get(r.clone()) != Some(name)) {
        return false;
    }
    if then == now {
        return true;
    }
    let aligned = Aligned::of(then, now, work);
    ranges.iter().all(|r| aligned.keeps(r, work))
}

/// The byte range of `span` (1-based lines, 1-based byte columns, end exclusive) in `text`.
pub(super) fn range(text: &str, span: &Span) -> Option<Range<usize>> {
    let at = |line: usize, col: usize| -> Option<usize> {
        let start: usize = text
            .split_inclusive('\n')
            .take(line.checked_sub(1)?)
            .map(str::len)
            .sum();
        let offset = start + col.checked_sub(1)?;
        (offset <= text.len()).then_some(offset)
    };
    let (from, to) = (at(span.0, span.1)?, at(span.2, span.3)?);
    (from < to).then_some(from..to)
}

/// Where each line of a text starts, and the text's length last.
fn starts(text: &str) -> Vec<usize> {
    let mut at = vec![0];
    for line in text.split_inclusive('\n') {
        at.push(at[at.len() - 1] + line.len());
    }
    at
}

/// Two texts, and the lines that anchor one to the other.
struct Aligned<'t> {
    then: &'t str,
    now: &'t str,
    /// Where each line starts in `then` and in `now`, each text's length last.
    old: Vec<usize>,
    new: Vec<usize>,
    /// Lines (of `then`, of `now`) no other line of either text repeats, the longest run of them in the same order in
    /// both.
    anchors: Vec<(usize, usize)>,
}

impl<'t> Aligned<'t> {
    fn of(then: &'t str, now: &'t str, work: &Cell<usize>) -> Aligned<'t> {
        let (old, new) = (starts(then), starts(now));
        // How often each line is in `then` and in `now`, and where last.
        let mut seen: HashMap<&str, [usize; 4]> = HashMap::new();
        for (text, at, side) in [(then, &old, 0), (now, &new, 2)] {
            for i in 0..at.len() - 1 {
                let line = &text[at[i]..at[i + 1]];
                add(work, 2 * line.len());
                let count = seen.entry(line).or_default();
                count[side] += 1;
                count[side + 1] = i;
            }
        }
        let mut unique: Vec<(usize, usize)> = seen
            .into_values()
            .filter(|count| count[0] == 1 && count[2] == 1)
            .map(|count| (count[1], count[3]))
            .collect();
        unique.sort_unstable();
        Aligned {
            then,
            now,
            old,
            new,
            anchors: increasing(&unique),
        }
    }

    fn old_line(&self, i: usize) -> &'t str {
        &self.then[self.old[i]..self.old[i + 1]]
    }

    fn new_line(&self, j: usize) -> &'t str {
        &self.now[self.new[j]..self.new[j + 1]]
    }

    /// Whether line `i` of `then` and line `j` of `now` are the same, counting the bytes compared.
    fn same(&self, i: usize, j: usize, work: &Cell<usize>) -> bool {
        let (a, b) = (self.old_line(i), self.new_line(j));
        let common = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
        add(work, common + 1);
        a == b
    }

    /// Whether `now` reads what `then` reads at `r` (on one line): its line anchored or shared at the edge of the
    /// stretch it is in, else the stretch's changed lines compared byte by byte when they are at most [`HUNK`] bytes.
    fn keeps(&self, r: &Range<usize>, work: &Cell<usize>) -> bool {
        let line = |at: usize| self.old.partition_point(|&start| start <= at) - 1;
        let at = line(r.start);
        if line(r.end - 1) != at {
            return false;
        }
        let next = self.anchors.partition_point(|&(i, _)| i < at);
        if self.anchors.get(next).is_some_and(|&(i, _)| i == at) {
            return true;
        }
        let (mut a, mut b) = next
            .checked_sub(1)
            .map_or((0, 0), |k| (self.anchors[k].0 + 1, self.anchors[k].1 + 1));
        let (mut c, mut d) = self
            .anchors
            .get(next)
            .copied()
            .unwrap_or((self.old.len() - 1, self.new.len() - 1));
        while a < c && b < d && self.same(a, b, work) {
            if a == at {
                return true;
            }
            (a, b) = (a + 1, b + 1);
        }
        while c > a && d > b && self.same(c - 1, d - 1, work) {
            if c - 1 == at {
                return true;
            }
            (c, d) = (c - 1, d - 1);
        }
        let (then, now) = (self.old[a]..self.old[c], self.new[b]..self.new[d]);
        if then.len() + now.len() > HUNK {
            return false;
        }
        Moved::between(self.then, self.now, then, now, work).keeps(self.then, self.now, r)
    }
}

/// The longest run of `pairs` (sorted by their first) whose seconds increase too.
fn increasing(pairs: &[(usize, usize)]) -> Vec<(usize, usize)> {
    // The pair ending the best run of each length found so far, and each pair's one before it in its run.
    let mut tails: Vec<usize> = Vec::new();
    let mut before: Vec<Option<usize>> = vec![None; pairs.len()];
    for (k, &(_, j)) in pairs.iter().enumerate() {
        let len = tails.partition_point(|&t| pairs[t].1 < j);
        before[k] = len.checked_sub(1).map(|l| tails[l]);
        match tails.get_mut(len) {
            Some(tail) => *tail = k,
            None => tails.push(k),
        }
    }
    let mut run = Vec::new();
    let mut at = tails.last().copied();
    while let Some(k) = at {
        run.push(pairs[k]);
        at = before[k];
    }
    run.reverse();
    run
}

fn add(work: &Cell<usize>, bytes: usize) {
    work.set(work.get() + bytes);
}

/// What changed from one stretch of a text to one of another, byte by byte: runs of the first text's bytes, each with
/// where it starts in the second when it is kept there.
struct Moved(Vec<(Range<usize>, Option<usize>)>);

impl Moved {
    fn between(
        then: &str,
        now: &str,
        a: Range<usize>,
        b: Range<usize>,
        work: &Cell<usize>,
    ) -> Moved {
        let counted = |bytes: &[u8]| -> Vec<Counted<'_>> {
            bytes.iter().map(|&byte| Counted(byte, work)).collect()
        };
        let (old, new) = (
            counted(&then.as_bytes()[a.clone()]),
            counted(&now.as_bytes()[b.clone()]),
        );
        let mut ops = Capture::new();
        let Ok(()) = myers::diff(&mut ops, &old, 0..old.len(), &new, 0..new.len());
        let runs = ops.into_ops().into_iter().map(|op| {
            let (tag, x, y) = op.as_tag_tuple();
            let kept = (tag == DiffTag::Equal).then_some(b.start + y.start);
            (a.start + x.start..a.start + x.end, kept)
        });
        Moved(runs.collect())
    }

    /// Where byte `at` of the first text is in the second, when it is kept.
    fn at(&self, at: usize) -> Option<usize> {
        let (run, kept) = self.0.iter().find(|(run, _)| run.contains(&at))?;
        kept.map(|start| start + at - run.start)
    }

    /// Whether `now` reads what `then` reads at `r`: its bytes kept, side by side, and each byte beside them either
    /// kept beside them or one that cannot extend a name (`T2,` reads `T2`; `T2x` and `xT2` do not).
    fn keeps(&self, then: &str, now: &str, r: &Range<usize>) -> bool {
        let (Some(first), Some(last)) = (self.at(r.start), self.at(r.end - 1)) else {
            return false;
        };
        if now.as_bytes().get(first..last + 1) != then.as_bytes().get(r.clone()) {
            return false;
        }
        let beside = |was: Option<usize>, is: usize| was.and_then(|at| self.at(at)) == Some(is);
        let left = first == 0
            || beside(r.start.checked_sub(1), first - 1)
            || now
                .get(..first)
                .is_some_and(|text| !extends(text.chars().next_back()));
        let right = beside((r.end < then.len()).then_some(r.end), last + 1)
            || now
                .get(last + 1..)
                .is_some_and(|text| !extends(text.chars().next()));
        left && right
    }
}

/// Whether `c` would extend a name it is written beside: a letter, a digit or `_` (no collector names a rule of its
/// own).
fn extends(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// A byte that counts each comparison of it.
struct Counted<'w>(u8, &'w Cell<usize>);

impl PartialEq for Counted<'_> {
    fn eq(&self, other: &Self) -> bool {
        add(self.1, 1);
        self.0 == other.0
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::{counted, reads};

    const THEN: &str = "// covers T2 U1\ntest('adds', () => {});\n";
    const AT: [(usize, usize, usize, usize); 1] = [(1, 11, 1, 13)];

    #[test]
    fn a_name_reads_through_edits_around_it() {
        assert!(reads(THEN, THEN, &AT, "T2"));
        assert!(reads(
            THEN,
            "// covers T2 U2\ntest('adds', () => {});\n",
            &AT,
            "T2"
        ));
        assert!(reads(
            THEN,
            &format!("// top\n{THEN}test('more', () => {{}});\n"),
            &AT,
            "T2"
        ));
    }

    #[test]
    fn a_name_changed_grown_or_cut_does_not_read() {
        for now in [
            "// covers T3 U1\ntest('adds', () => {});\n",
            "// covers T2x U1\ntest('adds', () => {});\n",
            "// covers xT2 U1\ntest('adds', () => {});\n",
            "// covers T U1\ntest('adds', () => {});\n",
            "test('adds', () => {});\n",
        ] {
            assert!(!reads(THEN, now, &AT, "T2"), "{now}");
        }
        assert!(!reads("T2", "xT2", &[(1, 1, 1, 3)], "T2"));
        assert!(!reads("T2", "T2x", &[(1, 1, 1, 3)], "T2"));
        assert!(
            !reads(THEN, THEN, &AT, "T3"),
            "the earlier text must read the name"
        );
    }

    #[test]
    fn a_name_reads_with_text_typed_beside_it_that_cannot_extend_it() {
        let then = "// covers T2\ntest('adds', () => {});\n";
        for now in [
            "// covers T2, T5\ntest('adds', () => {});\n",
            "// covers T2 // and more\ntest('adds', () => {});\n",
            "// covers (T2)\ntest('adds', () => {});\n",
        ] {
            assert!(reads(then, now, &AT, "T2"), "{now}");
        }
        for now in [
            "// covers T2x\ntest('adds', () => {});\n",
            "// covers xT2\ntest('adds', () => {});\n",
            "// covers T2_\ntest('adds', () => {});\n",
        ] {
            assert!(!reads(then, now, &AT, "T2"), "{now}");
        }
    }

    /// Thirty lines no other line of either text repeats.
    fn lines() -> String {
        (0..30).map(|i| format!("const a{i} = {i};\n")).collect()
    }

    #[test]
    fn a_line_moved_whole_carries_its_name_and_one_moved_and_edited_does_not() {
        let block = "// covers T2\ntest('adds', () => {});\ntest('more', () => {});\n";
        let then = format!("const a = 1;\n{block}");
        let now = format!("{block}const a = 1;\n");
        assert!(reads(&then, &now, &[(2, 11, 2, 13)], "T2"));

        let swapped = "test('adds', () => {});\n// covers T2 U1\n";
        assert!(reads(THEN, swapped, &AT, "T2"), "an adjacent line swap");
        let then = "// covers T2\nconst a = 1;\n";
        assert!(reads(then, "const a = 1;\n// covers T2\n", &AT, "T2"));

        let then = format!("// covers T2\n{}", lines());
        let now = format!("{}// covers T2\n", lines());
        assert!(reads(&then, &now, &AT, "T2"), "moved whole, far");
        let now = format!("{}// covers T2, T5\n", lines());
        assert!(!reads(&then, &now, &AT, "T2"), "moved far and edited");
    }

    /// Seventy test lines under the name: over 2 KB.
    fn tests_file() -> String {
        let mut text = String::from("// covers T2\n");
        for i in 0..70 {
            text.push_str(&format!("    expect(add({i}, 1)).toBe({});\n", i + 1));
        }
        assert!(text.len() > 2000);
        text
    }

    /// About 2 KB of new tests.
    fn pasted() -> String {
        (0..60)
            .map(|i| format!("  it('new {i}', () => {{}});\n"))
            .collect()
    }

    #[test]
    fn a_name_reads_through_a_reformat_and_a_paste_beside_it() {
        let then = tests_file();
        let reindented: String = then
            .lines()
            .map(|line| format!("  {}\n", line.trim_start()))
            .collect();
        assert!(reads(&then, &reindented, &AT, "T2"), "re-indented");
        let formatted: String = then
            .lines()
            .map(|line| format!("\t{}\n", line.trim_start().replace(", ", ",")))
            .collect();
        assert!(reads(&then, &formatted, &AT, "T2"), "formatted");
        let edited = then.replacen(
            "// covers T2\n",
            &format!("// covers T2, T5\n{}", pasted()),
            1,
        );
        assert!(
            reads(&then, &edited, &AT, "T2"),
            "edited beside, 2 KB pasted"
        );
    }

    #[test]
    fn a_name_typed_over_grown_or_written_twice_among_changed_lines_reads_as_gone() {
        let then = tests_file();
        for (now, what) in [
            ("// covers T3\n", "typed over"),
            ("// covers T2x\n", "grown"),
            ("// covers T2, T2\n", "written twice"),
        ] {
            let now = then.replacen("// covers T2\n", &format!("{now}{}", pasted()), 1);
            assert!(!reads(&then, &now, &AT, "T2"), "{what}");
        }
    }

    /// A text of more than 200 KB, every line distinct: the name on its first line.
    fn large() -> String {
        let mut text = String::from("// covers T2\n");
        for i in 0..4000 {
            text.push_str(&format!(
                "    let value_{i} = compute_something({i}, \"text\");\n"
            ));
        }
        assert!(text.len() > 200_000);
        text
    }

    /// Work no more than a few passes over both texts.
    fn linear(then: &str, now: &str, work: &Cell<usize>) {
        assert!(
            work.get() <= 4 * (then.len() + now.len()),
            "{} bytes compared",
            work.get()
        );
    }

    #[test]
    fn a_large_text_reindented_reads_in_work_linear_in_its_size() {
        let then = large();
        let now: String = then
            .lines()
            .map(|line| format!("  {}\n", line.trim_start()))
            .collect();
        let work = Cell::new(0);
        assert!(counted(&then, &now, &AT, "T2", &work));
        linear(&then, &now, &work);
    }

    #[test]
    fn a_large_text_with_the_name_line_reindented_reads_in_work_linear_in_its_size() {
        let then = large();
        let now = format!("  {then}");
        let work = Cell::new(0);
        assert!(counted(&then, &now, &[(1, 11, 1, 13)], "T2", &work));
        linear(&then, &now, &work);
    }

    #[test]
    fn a_name_on_a_long_changed_line_reads_in_work_linear_in_its_size() {
        let (a, b) = ("a".repeat(600), "b".repeat(600));
        let then = format!("// covers T2 {a}\n");
        let now = format!("// covers T2 {b}\n");
        let work = Cell::new(0);
        assert!(counted(&then, &now, &AT, "T2", &work));
        linear(&then, &now, &work);
    }
}
