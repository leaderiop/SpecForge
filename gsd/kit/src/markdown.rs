//! The gsd family's one Markdown reader (ADR 0332 D7): CommonMark through pulldown-cmark, read the way an agent reads
//! a prompt. No other gsd code finds a fence, a code span or a table cell of Markdown by hand.
//!
//! Plain CommonMark swallows the text under an XML section tag (`<step name="x">` and the lines after it, up to a
//! blank line) into one HTML block, and an agent reads that text as Markdown. So when an HTML block that is not a
//! comment ends, its leading tag-only lines are skipped and the rest is read again as Markdown, at its own lines,
//! at any depth. A comment is never read again. GFM tables are on for [`tables`] alone: everywhere else they would
//! split a code span at a `|`.
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

/// A GFM table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The text of the nearest heading before it (its code spans in backticks), or empty.
    pub heading: String,
    /// The header row's cells.
    pub header: Vec<String>,
    /// The body rows, each its cells.
    pub rows: Vec<Vec<String>>,
    /// The table's first line.
    pub line: usize,
}

/// The inline text of one leaf block: a paragraph, a heading, a list item's own text, or the lines of an HTML block
/// that are not read again as Markdown.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InlineBlock {
    /// Its text as an agent reads it: code spans kept in backticks, a line break as a space, a link as its text.
    pub text: String,
    /// The line each byte of `text` comes from.
    pub lines: Vec<usize>,
    /// Each link's byte range in `text`, with its destination.
    pub links: Vec<(Range<usize>, String)>,
}

impl InlineBlock {
    fn push(&mut self, text: &str, line: usize) {
        self.text.push_str(text);
        self.lines.resize(self.text.len(), line);
    }
}

/// Every fenced code block of `text`, from every level, in line order.
pub fn fences(text: &str) -> Vec<Fence> {
    read(text, Options::empty()).fences
}

/// The line ranges of the HTML blocks of `text`, from every level, in line order.
pub fn html_blocks(text: &str) -> Vec<Range<usize>> {
    read(text, Options::empty()).html
}

/// Each code span of `text` outside a fence, with the line it starts on. A span may cross lines; its content is
/// CommonMark's (a line break reads as a space), double-backtick spans whole.
pub fn inline_code(text: &str) -> Vec<(usize, String)> {
    read(text, Options::empty()).code
}

/// Each link of `text` (inline, reference or autolink), as its byte range in `text` and its destination.
pub fn links(text: &str) -> Vec<(Range<usize>, String)> {
    read(text, Options::empty()).links
}

/// The inline text of every leaf block of `text` outside a fence, from every level, in line order.
pub fn inline_blocks(text: &str) -> Vec<InlineBlock> {
    read(text, Options::empty()).blocks
}

/// Every GFM table of `text`, from every level, in line order. A cell is its source text, trimmed, its code spans
/// kept in backticks and each `\|` unescaped, as GFM reads a cell.
pub fn tables(text: &str) -> Vec<Table> {
    read(text, Options::ENABLE_TABLES).tables
}

#[derive(Default)]
struct Reading {
    fences: Vec<Fence>,
    html: Vec<Range<usize>>,
    code: Vec<(usize, String)>,
    tables: Vec<Table>,
    links: Vec<(Range<usize>, String)>,
    blocks: Vec<InlineBlock>,
    /// The text of the last heading read, kept across the fragments read again.
    heading: String,
}

fn read(text: &str, options: Options) -> Reading {
    let mut out = Reading::default();
    read_at(text, 0, 0, options, &mut out);
    out
}

/// Reads `frag`, whose first line is line `base` of the whole text and whose first byte is its byte `at`, into `out`.
fn read_at(frag: &str, base: usize, at: usize, options: Options, out: &mut Reading) {
    let starts: Vec<usize> = std::iter::once(0)
        .chain(frag.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let lines: Vec<&str> = frag.split('\n').collect();
    let line = |offset: usize| starts.partition_point(|&s| s <= offset) - 1;
    let last = |r: &Range<usize>| line(r.end.saturating_sub(1).max(r.start));
    let mut fence: Option<Fence> = None;
    let mut html: Option<usize> = None;
    let mut heading: Option<String> = None;
    let mut table: Option<Table> = None;
    let mut row: Vec<String> = Vec::new();
    let mut block = InlineBlock::default();
    let mut link: Vec<(usize, String)> = Vec::new();
    for (event, r) in Parser::new_ext(frag, options).into_offset_iter() {
        let here = base + line(r.start);
        match &event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                link.push((block.text.len(), dest_url.to_string()));
            }
            Event::End(TagEnd::Link) => {
                if let Some((start, dest)) = link.pop() {
                    block.links.push((start..block.text.len(), dest));
                }
            }
            Event::Start(tag) if !inline(tag) => flush(&mut block, out),
            Event::End(tag) if !inline_end(*tag) => flush(&mut block, out),
            Event::Text(text) if fence.is_none() && html.is_none() => block.push(text, here),
            Event::Code(code) => block.push(&format!("`{code}`"), here),
            Event::InlineHtml(text) => block.push(text, here),
            Event::SoftBreak | Event::HardBreak => block.push(" ", here),
            _ => {}
        }
        match event {
            Event::Start(Tag::Heading { .. }) => heading = Some(String::new()),
            Event::End(TagEnd::Heading(_)) => {
                if let Some(h) = heading.take() {
                    out.heading = h.trim().to_string();
                }
            }
            Event::Text(text) if heading.is_some() => {
                heading.get_or_insert_default().push_str(&text);
            }
            Event::Code(code) if heading.is_some() => {
                heading
                    .get_or_insert_default()
                    .push_str(&format!("`{code}`"));
                out.code.push((base + line(r.start), code.into_string()));
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                out.links
                    .push((at + r.start..at + r.end, dest_url.into_string()));
            }
            Event::Start(Tag::Table(_)) => {
                table = Some(Table {
                    heading: out.heading.clone(),
                    header: Vec::new(),
                    rows: Vec::new(),
                    line: base + line(r.start),
                });
            }
            Event::End(TagEnd::TableCell) => row.push(frag[r].trim().replace("\\|", "|")),
            Event::End(TagEnd::TableHead) => {
                if let Some(t) = table.as_mut() {
                    t.header = std::mem::take(&mut row);
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(t) = table.as_mut() {
                    t.rows.push(std::mem::take(&mut row));
                }
            }
            Event::End(TagEnd::Table) => out.tables.extend(table.take()),
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
                let comment = lines[a].trim_start().starts_with("<!--");
                let k = if comment {
                    b + 1
                } else {
                    (a..=b).find(|&l| !tag_only(lines[l])).unwrap_or(b + 1)
                };
                // the lines not read again are read as they stand
                for (l, raw) in lines.iter().enumerate().take(k).skip(a) {
                    block.push(raw, base + l);
                    block.push(" ", base + l);
                }
                flush(&mut block, out);
                if comment {
                    continue;
                }
                // `k > a`: the fragment read again is always shorter, so the reading ends
                if k > a && k <= b {
                    let end = starts.get(b + 1).copied().unwrap_or(frag.len());
                    read_at(
                        &frag[starts[k]..end],
                        base + k,
                        at + starts[k],
                        options,
                        out,
                    );
                }
            }
            _ => {}
        }
    }
    flush(&mut block, out);
}

/// Whether `tag` sits inside a leaf block's text rather than starting a block.
fn inline(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Emphasis
            | Tag::Strong
            | Tag::Strikethrough
            | Tag::Superscript
            | Tag::Subscript
            | Tag::Link { .. }
            | Tag::Image { .. }
    )
}

/// [`inline`] for a closing tag.
fn inline_end(tag: TagEnd) -> bool {
    matches!(
        tag,
        TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript
            | TagEnd::Link
            | TagEnd::Image
    )
}

/// Ends the leaf block being read: a block with text joins `out`.
fn flush(block: &mut InlineBlock, out: &mut Reading) {
    let done = std::mem::take(block);
    if !done.text.trim().is_empty() {
        out.blocks.push(done);
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
    use super::{fences, html_blocks, inline_blocks, inline_code, links, tables};

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

    #[test]
    fn a_table_cell_keeps_its_code_and_unescapes_a_pipe() {
        let text = "# Intro\n\n<rules>\n## The `gsd/E101` table\n\n| Code | Means |\n|---|---|\n| `gsd/E101` | `a \\| b` or *c* |\n| x |  |\n\n</rules>\n";
        let found = tables(text);
        assert_eq!(found.len(), 1, "{found:?}");
        let t = &found[0];
        assert_eq!((t.heading.as_str(), t.line), ("The `gsd/E101` table", 5));
        assert_eq!(t.header, ["Code", "Means"]);
        assert_eq!(
            t.rows,
            [
                vec!["`gsd/E101`".to_string(), "`a | b` or *c*".to_string()],
                vec!["x".to_string(), String::new()]
            ]
        );
        // only `tables` reads GFM tables: elsewhere the `|` stays in its code span
        assert!(inline_code(text).contains(&(7, "a \\| b".to_string())));
    }

    #[test]
    fn a_link_gives_its_range_and_destination() {
        let text = "<step>\nSee [the glossary](https://example.com/CONTEXT.md) and <https://a.b/c>.\n</step>\n";
        let found = links(text);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(
            &text[found[0].0.clone()],
            "[the glossary](https://example.com/CONTEXT.md)"
        );
        assert_eq!(found[0].1, "https://example.com/CONTEXT.md");
        assert_eq!(&text[found[1].0.clone()], "<https://a.b/c>");
        assert_eq!(found[1].1, "https://a.b/c");
    }

    #[test]
    fn a_leaf_block_gives_its_inline_text_by_line() {
        let text = "<step>\nWrite `STATE.md`\nnow.\n</step>\n\n- one *item*\n  - two [x](https://a.b/x)\n\n```md\nnot prose\n```\n<!-- a note -->\n";
        let found = inline_blocks(text);
        let texts: Vec<&str> = found.iter().map(|b| b.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "<step> ",
                // a tag line cannot interrupt a paragraph: it is the paragraph's own text
                "Write `STATE.md` now. </step>",
                "one item",
                "two x",
                "<!-- a note --> "
            ]
        );
        let write = &found[1];
        assert_eq!(write.lines.len(), write.text.len());
        let now = write.text.find("now").expect("now");
        assert_eq!(
            (
                write.lines[0],
                write.lines[now],
                write.lines[write.text.len() - 1]
            ),
            (1, 2, 3)
        );
        assert_eq!(found[3].links, [(4..5, "https://a.b/x".to_string())]);
        assert_eq!(found[3].lines[0], 6);
    }
}
