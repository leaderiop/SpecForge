//! The gsd family's one Markdown reader (ADR 0332 D7): CommonMark through pulldown-cmark, with no extension, read
//! the way an agent reads a prompt. No other gsd code finds a fence or a code span of Markdown by hand; a table row is
//! still cut on `|` by hand (and `core/tests/common/mod.rs::input_of` reads one table cell's own grammar), as tables
//! are not read here.
//!
//! Plain CommonMark swallows the text under an XML section tag (`<step name="x">` and the lines after it, up to a
//! blank line) into one HTML block, and an agent reads that text as Markdown. So when an HTML block that is not a
//! comment ends, its leading tag-only lines are skipped and the rest is read again as Markdown, at its own lines,
//! at any depth. A comment is never read again. GFM tables stay off: they would split a code span at a `|`.
//!
//! Lines are 0-based indices over `text.split('\n')`.

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// A fenced code block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fence {
    /// The info string's first word (`bash`), or empty.
    pub language: String,
    /// The opening mark's line.
    pub open: usize,
    /// The code lines, without the marks.
    pub code: Range<usize>,
    /// The code as CommonMark gives it, with the container's indentation removed.
    pub text: String,
    /// Whether the block's last line, after `open`, is a run of `mark` at least `width` long and nothing else.
    pub closed: bool,
    /// The fence character: a backtick or a tilde.
    pub mark: char,
    /// The opening run's length.
    pub width: usize,
}

/// Every fenced code block of `text`, from every level, in line order.
pub fn fences(text: &str) -> Vec<Fence> {
    read(text).fences
}

/// The line ranges of the HTML blocks of `text`, from every level, in line order.
pub fn html_blocks(text: &str) -> Vec<Range<usize>> {
    read(text).html
}

/// Each code span of `text` outside a fence, with the line it starts on. A span may cross lines; its content is
/// CommonMark's (a line break reads as a space), double-backtick spans whole.
pub fn inline_code(text: &str) -> Vec<(usize, String)> {
    read(text).code
}

#[derive(Default)]
struct Reading {
    fences: Vec<Fence>,
    html: Vec<Range<usize>>,
    code: Vec<(usize, String)>,
}

fn read(text: &str) -> Reading {
    let mut out = Reading::default();
    read_at(text, 0, &mut out);
    out
}

/// Reads `frag`, whose first line is line `base` of the whole text, into `out`.
fn read_at(frag: &str, base: usize, out: &mut Reading) {
    let starts: Vec<usize> = std::iter::once(0)
        .chain(frag.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let lines: Vec<&str> = frag.split('\n').collect();
    let line = |offset: usize| starts.partition_point(|&s| s <= offset) - 1;
    let last = |r: &Range<usize>| line(r.end.saturating_sub(1).max(r.start));
    let mut fence: Option<Fence> = None;
    let mut html: Option<usize> = None;
    for (event, r) in Parser::new_ext(frag, Options::empty()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                let (a, b) = (line(r.start), last(&r));
                // container marks (`>`, `-`, `1.`) hold neither fence character
                let opening = lines[a].find(['`', '~']).map_or("", |i| &lines[a][i..]);
                let mark = opening.chars().next().unwrap_or('`');
                let width = opening.chars().take_while(|&c| c == mark).count();
                let closing = lines[b].trim().trim_start_matches(['>', ' ']);
                let closed =
                    b > a && closing.chars().all(|c| c == mark) && closing.chars().count() >= width;
                fence = Some(Fence {
                    language: info.split_whitespace().next().unwrap_or("").to_string(),
                    open: base + a,
                    code: base + a + 1..base + if closed { b } else { b + 1 },
                    text: String::new(),
                    closed,
                    mark,
                    width,
                });
            }
            Event::Text(text) => {
                if let Some(f) = fence.as_mut() {
                    f.text.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(f) = fence.take() {
                    out.fences.push(f);
                }
            }
            Event::Code(code) => out.code.push((base + line(r.start), code.into_string())),
            Event::Start(Tag::HtmlBlock) => html = Some(r.start),
            Event::End(TagEnd::HtmlBlock) => {
                let Some(start) = html.take() else { continue };
                let (a, b) = (line(start), last(&(start..r.end)));
                out.html.push(base + a..base + b + 1);
                if lines[a].trim_start().starts_with("<!--") {
                    continue;
                }
                let k = (a..=b).find(|&l| !tag_only(lines[l])).unwrap_or(b + 1);
                // `k > a`: the fragment read again is always shorter, so the reading ends
                if k > a && k <= b {
                    let end = starts.get(b + 1).copied().unwrap_or(frag.len());
                    read_at(&frag[starts[k]..end], base + k, out);
                }
            }
            _ => {}
        }
    }
}

/// Whether `line` is a tag and nothing else: not blank, not a comment, and only HTML when parsed alone.
fn tag_only(line: &str) -> bool {
    let t = line.trim();
    !t.is_empty()
        && !t.starts_with("<!--")
        && Parser::new(t).all(|e| {
            matches!(
                e,
                Event::Start(Tag::HtmlBlock) | Event::End(TagEnd::HtmlBlock) | Event::Html(_)
            )
        })
}

#[cfg(test)]
mod tests {
    use super::{fences, html_blocks, inline_code};

    #[test]
    fn a_fence_under_a_section_tag_is_a_fence() {
        let text = "<step name=\"x\">\nRun it:\n```bash\nls\n```\n</step>\n";
        let found = fences(text);
        assert_eq!(found.len(), 1, "{found:?}");
        let f = &found[0];
        assert_eq!(
            (f.language.as_str(), f.open, f.code.clone()),
            ("bash", 2, 3..4)
        );
        assert!(f.closed);
        assert_eq!(f.text, "ls\n");
        assert_eq!((f.mark, f.width), ('`', 3));
    }

    #[test]
    fn a_mark_as_long_as_the_fence_closes_it() {
        // a nested block's info string does not open it (CommonMark): its bare closing mark closes the outer fence
        let text = "```\na:\n```yaml\nb: 1\n```\nc\n```\n";
        let found = fences(text);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!((found[0].code.clone(), found[0].closed), (1..4, true));
        assert_eq!(found[0].text, "a:\n```yaml\nb: 1\n");
        assert_eq!((found[1].open, found[1].closed), (6, false));
    }

    #[test]
    fn an_unclosed_fence_is_marked() {
        let found = fences("text\n\n~~~~sh\nls\n~~~\n");
        assert_eq!(found.len(), 1);
        assert!(!found[0].closed);
        assert_eq!(
            (found[0].mark, found[0].width, found[0].code.clone()),
            ('~', 4, 3..5)
        );
    }

    #[test]
    fn a_code_span_may_cross_lines_and_holds_its_pipes() {
        let text = "Run `a |\nb` and ``c ` d``.\n\n```\n`no`\n```\n";
        assert_eq!(
            inline_code(text),
            [(0, "a | b".to_string()), (1, "c ` d".to_string())]
        );
    }

    #[test]
    fn a_comment_is_not_read_again() {
        let text = "<!--\n```bash\nls\n```\n-->\n";
        assert!(fences(text).is_empty());
        assert_eq!(html_blocks(text), vec![(0..5)]);
    }

    #[test]
    fn fence_text_drops_the_list_item_indent() {
        let text =
            "1. Do this:\n\n   ```bash\n   ls -la\n     cd x\n   ```\n2. ```sh\n   pwd\n   ```\n";
        let found = fences(text);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(found[0].text, "ls -la\n  cd x\n");
        assert_eq!((found[0].open, found[0].code.clone()), (2, 3..5));
        assert!(found[0].closed);
        assert_eq!(
            (found[1].language.as_str(), found[1].text.as_str()),
            ("sh", "pwd\n")
        );
        assert!(found[1].closed);
    }

    #[test]
    fn a_tagged_block_is_read_again_below_its_tags() {
        let text = "<task>\n<name>x</name>\nSee `a`.\n<!-- note -->\n</task>\n";
        assert_eq!(inline_code(text), [(2, "a".to_string())]);
        assert_eq!(html_blocks(text), [0..5, 3..4, 4..5]);
    }
}
