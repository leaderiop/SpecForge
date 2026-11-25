//! A Markdown file's sections, from the gsd family's one Markdown reader (`gsd_kit::markdown::headings`, ADR 0332 D7):
//! a `#` line inside a fence or an HTML comment never starts a section, and a heading under an XML section tag does.
//!
//! Lines are 0-based, as `gsd_kit::markdown` gives them; a reader adds 1 for a report entry. Give a file with a
//! frontmatter as `frontmatter::body`, so its `---` lines read as nothing.

use std::ops::Range;

use gsd_kit::markdown;

/// One heading and the lines it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub level: u8,
    /// The heading's text, its code spans in backticks.
    pub title: String,
    /// The heading's line.
    pub line: usize,
    /// The lines after the heading, up to the next heading of the same or a higher level (sub-sections included).
    pub body: Range<usize>,
}

/// Every section of `text`, in line order.
pub fn sections(text: &str) -> Vec<Section> {
    let headings = markdown::headings(text);
    let total = text.split('\n').count();
    let mut out: Vec<Section> = headings
        .iter()
        .map(|h| Section {
            level: h.level,
            title: h.text.clone(),
            line: h.line,
            body: h.line + 1..total,
        })
        .collect();
    // One pass: each heading closes the open sections of its level or deeper.
    let mut open: Vec<usize> = Vec::new();
    for i in 0..out.len() {
        while let Some(&j) = open.last() {
            if out[j].level < out[i].level {
                break;
            }
            out[j].body.end = out[i].line;
            open.pop();
        }
        open.push(i);
    }
    out
}

/// Whether each line of `text` is part of a fenced code block (its marks included).
pub fn fenced(text: &str) -> Vec<bool> {
    let mut fenced = vec![false; text.split('\n').count()];
    for fence in markdown::fences(text) {
        let end = (fence.code.end + 1).min(fenced.len());
        fenced[fence.open.min(end)..end].fill(true);
    }
    fenced
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_runs_to_the_next_heading_of_its_level_or_higher() {
        let text = "# A\nx\n## B\ny\n### C\nz\n## D\n```\n# no\n```\n";
        let all = sections(text);
        let found: Vec<(u8, &str, usize, Range<usize>)> = all
            .iter()
            .map(|s| (s.level, s.title.as_str(), s.line, s.body.clone()))
            .collect();
        assert_eq!(
            found,
            [
                (1, "A", 0, 1..11),
                (2, "B", 2, 3..6),
                (3, "C", 4, 5..6),
                (2, "D", 6, 7..11)
            ]
        );
        assert_eq!(
            fenced(text),
            [
                false, false, false, false, false, false, false, true, true, true, false
            ]
        );
    }

    #[test]
    fn a_frontmatter_given_as_its_body_starts_no_section() {
        let text = "---\na: 1\n---\n# Title\n";
        let (_, start) = crate::frontmatter::split(text);
        let found = sections(&crate::frontmatter::body(text, start));
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].title.as_str(), found[0].line), ("Title", 3));
    }
}
