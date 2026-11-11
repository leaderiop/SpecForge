//! Whether a file's text still reads a name where an earlier text of it read it (ADR 0346 D7): the earlier text's
//! places carried into the text now through what changed between them (by line, then by byte inside the lines that
//! changed), each read with a byte of context on either side, so a name grown or cut at its edge is not taken for it.
//! Edits elsewhere (another rename, typing) move a place and leave what it reads.

use std::ops::Range;

use similar::{Algorithm, DiffTag, capture_diff_slices};
use specforge_project::facts::Span;

/// Whether `now` reads `name` at each of `spans`, places of `then` that read it there.
pub(super) fn reads(then: &str, now: &str, spans: &[Span], name: &str) -> bool {
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
    let moved = Moved::between(then, now);
    ranges.iter().all(|r| moved.keeps(then, now, r))
}

/// The byte range of `span` (1-based lines, 1-based byte columns, end exclusive) in `text`.
fn range(text: &str, span: &Span) -> Option<Range<usize>> {
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

/// What changed from one text to another: runs of the first text's bytes, each with where it starts in the second
/// when it is kept there.
struct Moved(Vec<(Range<usize>, Option<usize>)>);

impl Moved {
    fn between(then: &str, now: &str) -> Moved {
        let old: Vec<&str> = then.split_inclusive('\n').collect();
        let new: Vec<&str> = now.split_inclusive('\n').collect();
        let starts = |lines: &[&str]| -> Vec<usize> {
            let mut at = vec![0];
            for line in lines {
                at.push(at.last().copied().unwrap_or(0) + line.len());
            }
            at
        };
        let (o, n) = (starts(&old), starts(&new));
        let mut runs = Vec::new();
        for op in capture_diff_slices(Algorithm::Myers, &old, &new) {
            let (tag, a, b) = op.as_tag_tuple();
            let (a, b) = (o[a.start]..o[a.end], n[b.start]..n[b.end]);
            if tag == DiffTag::Equal {
                runs.push((a, Some(b.start)));
                continue;
            }
            let (then, now) = (&then.as_bytes()[a.clone()], &now.as_bytes()[b.clone()]);
            for op in capture_diff_slices(Algorithm::Myers, then, now) {
                let (tag, x, y) = op.as_tag_tuple();
                let kept = (tag == DiffTag::Equal).then_some(b.start + y.start);
                runs.push((a.start + x.start..a.start + x.end, kept));
            }
        }
        Moved(runs)
    }

    /// Where byte `at` of the first text is in the second, when it is kept.
    fn at(&self, at: usize) -> Option<usize> {
        let (run, kept) = self.0.iter().find(|(run, _)| run.contains(&at))?;
        kept.map(|start| start + at - run.start)
    }

    /// Whether `now` reads what `then` reads at `r`, with a byte of context either side (none past a text's edge, where
    /// nothing may be typed beside it either).
    fn keeps(&self, then: &str, now: &str, r: &Range<usize>) -> bool {
        let (from, to) = (r.start.saturating_sub(1), (r.end + 1).min(then.len()));
        let (Some(first), Some(last)) = (self.at(from), self.at(to - 1)) else {
            return false;
        };
        if (r.start == 0 && first != 0) || (r.end == then.len() && last + 1 != now.len()) {
            return false;
        }
        now.as_bytes().get(first..last + 1) == then.as_bytes().get(from..to)
    }
}

#[cfg(test)]
mod tests {
    use super::reads;

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
}
