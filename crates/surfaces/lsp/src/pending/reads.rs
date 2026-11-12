//! Whether a file's text still reads a name where an earlier text of it read it (ADR 0346 D7): each place of the
//! earlier text carried into the text now through what changed between them. A line kept (anchored, shared at the
//! edge of the stretch it is in, or moved whole: the only line of its text in either) keeps its names. Inside a
//! changed stretch the name is counted where it reads whole (nothing beside it that could extend it: `T2,` reads
//! `T2`, `T2x` does not) in the stretch of each text: when the counts match, the place is the occurrence of the same
//! ordinal, and reads; otherwise it reads as gone (typed over, grown, or written once more in the stretch).
//!
//! The work is counted, never timed, and is linear in the two texts whatever their size and however many places are
//! read: the lines no other line of either text repeats anchor the two texts (the longest run of them in the same
//! order in both), and each changed stretch holding a place is trimmed of the lines both share at its edges and
//! scanned once in each text, for every place in it.

use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Range;

use specforge_project::facts::Span;

/// Whether `now` reads `name` at each of `spans`, places of `then` that read it there.
pub(super) fn reads(then: &str, now: &str, spans: &[Span], name: &str) -> bool {
    counted(then, now, spans, name, &Cell::new(0))
}

/// [`reads`], adding to `work` each byte it compares or scans.
fn counted(then: &str, now: &str, spans: &[Span], name: &str, work: &Cell<usize>) -> bool {
    let lines = starts(then);
    let Some(ranges) = spans
        .iter()
        .map(|span| range_in(&lines, span))
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
    let mut stretches = HashMap::new();
    ranges
        .iter()
        .all(|r| aligned.keeps(r, name, &mut stretches, work))
}

/// The byte range of `span` (1-based lines, 1-based byte columns, end exclusive) in `text`.
pub(super) fn range(text: &str, span: &Span) -> Option<Range<usize>> {
    range_in(&starts(text), span)
}

/// The byte range of `span` in the text whose lines start at `lines` ([`starts`]).
fn range_in(lines: &[usize], span: &Span) -> Option<Range<usize>> {
    let at = |line: usize, col: usize| -> Option<usize> {
        let offset = lines.get(line.checked_sub(1)?)? + col.checked_sub(1)?;
        (offset <= lines[lines.len() - 1]).then_some(offset)
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
    /// The lines of `then` no other line of either text repeats, in order: each is in `now` once, whole.
    whole: Vec<usize>,
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
            whole: unique.iter().map(|&(i, _)| i).collect(),
        }
    }

    /// Whether line `i` of `then` and line `j` of `now` are the same, counting the bytes compared.
    fn same(&self, i: usize, j: usize, work: &Cell<usize>) -> bool {
        let a = &self.then[self.old[i]..self.old[i + 1]];
        let b = &self.now[self.new[j]..self.new[j + 1]];
        let common = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
        add(work, common + 1);
        a == b
    }

    /// Whether `now` reads `name`, which `then` reads at `r` (on one line): its line kept, or else the name read whole
    /// as often in the changed stretch of each text, `r` among them. Each stretch is trimmed and scanned once, for every place in it (`stretches`, by the
    /// anchor after it).
    fn keeps(
        &self,
        r: &Range<usize>,
        name: &str,
        stretches: &mut HashMap<usize, Stretch>,
        work: &Cell<usize>,
    ) -> bool {
        let line = |at: usize| self.old.partition_point(|&start| start <= at) - 1;
        let at = line(r.start);
        if line(r.end - 1) != at {
            return false;
        }
        if self.whole.binary_search(&at).is_ok() {
            return true;
        }
        let next = self.anchors.partition_point(|&(i, _)| i < at);
        let stretch = stretches
            .entry(next)
            .or_insert_with(|| self.stretch(next, name, work));
        if !stretch.lines.contains(&at) {
            return true;
        }
        stretch.read.binary_search(&r.start).is_ok() && stretch.read.len() == stretch.now.len()
    }

    /// The changed stretch of lines before anchor `next`, trimmed of the lines both texts share at its edges, and
    /// where `name` reads whole in it in each text.
    fn stretch(&self, next: usize, name: &str, work: &Cell<usize>) -> Stretch {
        let (mut a, mut b) = next
            .checked_sub(1)
            .map_or((0, 0), |k| (self.anchors[k].0 + 1, self.anchors[k].1 + 1));
        let (mut c, mut d) = self
            .anchors
            .get(next)
            .copied()
            .unwrap_or((self.old.len() - 1, self.new.len() - 1));
        while a < c && b < d && self.same(a, b, work) {
            (a, b) = (a + 1, b + 1);
        }
        while c > a && d > b && self.same(c - 1, d - 1, work) {
            (c, d) = (c - 1, d - 1);
        }
        Stretch {
            lines: a..c,
            read: whole(self.then, self.old[a]..self.old[c], name, work),
            now: whole(self.now, self.new[b]..self.new[d], name, work),
        }
    }
}

/// A changed stretch: its lines in `then`, and where the name reads whole in it in each text, in order.
struct Stretch {
    lines: Range<usize>,
    read: Vec<usize>,
    now: Vec<usize>,
}

/// Where in `within` of `text` the name reads whole: neither byte beside it one that would extend it.
fn whole(text: &str, within: Range<usize>, name: &str, work: &Cell<usize>) -> Vec<usize> {
    add(work, within.len());
    let start = within.start;
    text[within]
        .match_indices(name)
        .map(|(at, _)| start + at)
        .filter(|&at| {
            !extends(text[..at].chars().next_back())
                && !extends(text[at + name.len()..].chars().next())
        })
        .collect()
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

/// Whether `c` would extend a name it is written beside: a letter, a digit or `_` (no collector names a rule of its
/// own).
fn extends(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
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

    /// The span of the first `name` on line `line` (1-based) of `text`.
    fn on(text: &str, line: usize, name: &str) -> [(usize, usize, usize, usize); 1] {
        let col = text
            .lines()
            .nth(line - 1)
            .and_then(|l| l.find(name))
            .expect("the name")
            + 1;
        [(line, col, line, col + name.len())]
    }

    #[test]
    fn a_name_counted_in_a_changed_stretch_reads_only_beside_what_it_was_beside() {
        for (then, now, what) in [
            ("a T2\n", "a T1 T2\n", "typed over, then typed after it"),
            (
                "// covers T2\n",
                "// covers T1 T2\n",
                "typed over, then typed after it",
            ),
            (
                "// covers T2\ntest('a');\n",
                "// covers T1\n// see T2\ntest('a');\n",
                "typed over, then typed on a line under it",
            ),
            (
                "x\n// covers T2\n// note\ny\n",
                "x\n// covers T1\n// note T2\ny\n",
                "typed over, then typed on the next line",
            ),
            (
                "// covers T2\nz\n",
                "// covers T1\nT2\nz\n",
                "moved onto a line of its own",
            ),
        ] {
            let line = then.lines().position(|l| l.contains("T2")).expect("T2") + 1;
            assert!(!reads(then, now, &on(then, line, "T2"), "T2"), "{what}");
        }
        // Limits of reading by count and by what is beside the name (D7): one written over and another added beside
        // what the first was beside reads; so does a line moved whole, though a copy pasted with the first undone is
        // the same texts. W198 says so at the next collect.
        let then = "// covers T2\n// covers T2\nz\n";
        let now = "// covers T9\n// covers T2 T2\nz\n";
        assert!(reads(then, now, &on(then, 1, "T2"), "T2"));
        let then = "a\n// covers T2\nb\n";
        let now = "a\n// covers T1\nb\n// covers T2\n";
        assert!(reads(then, now, &on(then, 2, "T2"), "T2"));
    }

    #[test]
    fn many_places_in_one_changed_stretch_read_in_work_linear_in_the_texts() {
        let mut then = String::new();
        for i in 0..1500 {
            then.push_str(&format!(
                "// covers T2\n    test('case {i}', () => {{ expect(f({i})).toBe(1); }});\n"
            ));
        }
        assert!(then.len() > 100_000);
        let now: String = then
            .lines()
            .map(|line| format!("  {}\n", line.trim_start()))
            .collect();
        let spans: Vec<_> = (0..1500).map(|i| (2 * i + 1, 11, 2 * i + 1, 13)).collect();
        let work = Cell::new(0);
        assert!(counted(&then, &now, &spans, "T2", &work));
        linear(&then, &now, &work);
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
