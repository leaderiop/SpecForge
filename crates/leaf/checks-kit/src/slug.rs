//! The anchors of a Markdown file, as GitHub renders them: each heading's slug, an explicit `{#id}`, and every `id`
//! and `<a name>` of its HTML. The headings are found by a CommonMark block scanner: ATX and setext headings, with
//! fenced and indented code, HTML blocks, block quotes and list items read as CommonMark reads them, so a `#` line in
//! a code block names no anchor. Linear in the text.

use std::collections::{BTreeSet, HashMap};

/// Every anchor `markdown` defines: heading slugs in order, duplicates numbered (`-1`, `-2`), then HTML anchors.
pub fn anchors(markdown: &str) -> BTreeSet<String> {
    let mut found: BTreeSet<String> = BTreeSet::new();
    found.extend(heading_anchors(markdown));
    found.extend(crate::html::anchors(markdown.as_bytes()));
    found
}

/// The heading anchors alone, in document order, as [`anchors`] numbers them.
pub fn heading_anchors(markdown: &str) -> Vec<String> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    headings(markdown)
        .into_iter()
        .map(|heading| match explicit_id(&heading) {
            Some(id) => id.to_string(),
            None => numbered(slug(&heading), &mut seen),
        })
        .collect()
}

/// `slug`, or `slug-1`, `slug-2`… when it was taken: GitHub's rule.
fn numbered(slug: String, seen: &mut HashMap<String, usize>) -> String {
    let mut result = slug.clone();
    while seen.contains_key(&result) {
        let count = seen.entry(slug.clone()).or_default();
        *count += 1;
        result = format!("{slug}-{count}");
    }
    seen.insert(result.clone(), 0);
    result
}

/// A heading's explicit `{#id}`, written at its end.
fn explicit_id(heading: &str) -> Option<&str> {
    let inner = heading.trim_end().strip_suffix('}')?;
    let open = inner.rfind("{#")?;
    let id = &inner[open + 2..];
    let plain = !id.is_empty() && !id.contains(|c: char| c.is_whitespace() || c == '}');
    plain.then_some(id)
}

/// A heading's GitHub slug: its text with inline markup dropped (a link reads as its text), lowercased, every
/// character but letters, digits, `-`, `_` and spaces dropped, and spaces made `-`.
pub fn slug(heading: &str) -> String {
    text_of(heading)
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            '-' | '_' => Some(c),
            c if c.is_alphanumeric() => Some(c),
            _ => None,
        })
        .collect()
}

/// The text of inline Markdown: a link or image reads as its text, an HTML tag as nothing, an emphasis `_` as
/// nothing; a code span reads as written.
fn text_of(inline: &str) -> String {
    let mut out = String::with_capacity(inline.len());
    let mut code = false;
    let mut chars = inline.char_indices().peekable();
    let mut previous = ' ';
    // Once a search for a closing `>` or `)` failed, every later one fails too: never search again, so a line of
    // `<<<<` or `](](` is read once.
    let (mut no_angle, mut no_paren) = (false, false);
    while let Some((at, c)) = chars.next() {
        let next = chars.peek().map_or(' ', |(_, c)| *c);
        match c {
            '`' => code = !code,
            _ if code => out.push(c),
            '<' if no_angle => out.push(c),
            '<' => match inline[at..].find('>') {
                // An autolink keeps its address; a tag is dropped.
                Some(end) => {
                    let inside = &inline[at + 1..at + end];
                    if inside.contains("://") {
                        out.push_str(inside);
                    }
                    while chars.peek().is_some_and(|(i, _)| *i <= at + end) {
                        chars.next();
                    }
                }
                None => {
                    no_angle = true;
                    out.push(c);
                }
            },
            ']' if next == '(' && !no_paren => match inline[at..].find(')') {
                Some(end) => {
                    while chars.peek().is_some_and(|(i, _)| *i <= at + end) {
                        chars.next();
                    }
                }
                None => no_paren = true,
            },
            ']' if next == '(' => {}
            // An underscore at a word's edge is emphasis; inside a word it is text.
            '_' if !previous.is_alphanumeric() || !next.is_alphanumeric() => {}
            _ => out.push(c),
        }
        previous = c;
    }
    out
}

/// A fenced code block's opening fence: its character and length.
#[derive(Clone, Copy)]
struct Fence {
    marker: u8,
    len: usize,
}

/// What the scanner is inside, across lines.
enum Block {
    Plain,
    Fenced(Fence),
    /// An HTML block that ends at a line containing its end marker, or at a blank line when it has none.
    Html(Option<&'static str>),
}

/// The text of each heading of `markdown`, in order.
pub fn headings(markdown: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut block = Block::Plain;
    // The paragraph being read, which a setext underline makes a heading, and whether it opened in a container.
    let mut paragraph: Option<String> = None;
    let mut contained = false;
    for raw in markdown.lines() {
        let line = container_content(raw);
        match block {
            Block::Fenced(fence) => {
                if closes(line, fence) {
                    block = Block::Plain;
                }
                continue;
            }
            Block::Html(end) => {
                let ended = match end {
                    Some(marker) => line.to_ascii_lowercase().contains(marker),
                    None => line.trim().is_empty(),
                };
                if ended {
                    block = Block::Plain;
                }
                continue;
            }
            Block::Plain => {}
        }
        if line.trim().is_empty() {
            paragraph = None;
            continue;
        }
        let indent = indent_of(line);
        if indent >= 4 {
            // Indented code, unless it continues a paragraph.
            if let Some(text) = &mut paragraph {
                text.push(' ');
                text.push_str(line.trim());
            }
            continue;
        }
        let rest = &line[indent..];
        if let Some(fence) = opens_fence(rest) {
            paragraph = None;
            block = Block::Fenced(fence);
            continue;
        }
        if let Some(text) = atx(rest) {
            paragraph = None;
            found.push(text.to_string());
            continue;
        }
        // An underline outside the container its paragraph is in breaks the container instead.
        let same_container = contained == (line.len() != raw.len());
        if let (Some(text), true, true) = (&paragraph, setext(rest), same_container) {
            found.push(text.clone());
            paragraph = None;
            continue;
        }
        if thematic_break(rest) {
            paragraph = None;
            continue;
        }
        if let Some(end) = html_start(rest)
            && (paragraph.is_none() || end.is_some())
        {
            paragraph = None;
            block = Block::Html(end);
            // A block whose end marker is on its first line ends there.
            if let Some(marker) = end {
                let after = rest.to_ascii_lowercase();
                let first = after.find('>').map_or(0, |at| at + 1);
                if after[first..].contains(marker) {
                    block = Block::Plain;
                }
            }
            continue;
        }
        match &mut paragraph {
            Some(text) => {
                text.push(' ');
                text.push_str(rest.trim());
            }
            None => {
                paragraph = Some(rest.trim().to_string());
                contained = line.len() != raw.len();
            }
        }
    }
    found
}

/// The columns of leading spaces, a tab counting to the next multiple of 4.
fn indent_of(line: &str) -> usize {
    let mut columns = 0;
    for byte in line.bytes() {
        match byte {
            b' ' => columns += 1,
            b'\t' => columns += 4 - columns % 4,
            _ => break,
        }
    }
    columns.min(line.len())
}

/// What a line holds once its block quote markers and its list item marker are read: a heading in either is one.
fn container_content(line: &str) -> &str {
    let mut rest = line;
    loop {
        let indent = rest.len() - rest.trim_start_matches(' ').len();
        let after = &rest[indent..];
        if indent <= 3 && after.starts_with('>') {
            rest = after[1..].strip_prefix(' ').unwrap_or(&after[1..]);
            continue;
        }
        if indent <= 3
            && let Some(content) = list_item(after)
        {
            rest = content;
            continue;
        }
        return rest;
    }
}

/// The content after a list item's marker (`-`, `*`, `+`, or one to nine digits and `.` or `)`), followed by a space.
fn list_item(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let marker = match bytes.first()? {
        b'-' | b'*' | b'+' => 1,
        b'0'..=b'9' => {
            let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
            let closer = *bytes.get(digits)?;
            (digits <= 9 && matches!(closer, b'.' | b')')).then_some(digits + 1)?
        }
        _ => return None,
    };
    // A thematic break (`- - -`, `***`) is no list item.
    if thematic_break(text) {
        return None;
    }
    match bytes.get(marker) {
        Some(b' ' | b'\t') => Some(&text[marker + 1..]),
        _ => None,
    }
}

/// An ATX heading's text: one to six `#`, then a space or the end, its closing `#` sequence dropped.
fn atx(text: &str) -> Option<&str> {
    let level = text.bytes().take_while(|b| *b == b'#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let after = &text[level..];
    if !(after.is_empty() || after.starts_with([' ', '\t'])) {
        return None;
    }
    let content = after.trim();
    let closed = content.trim_end_matches('#');
    let content = if closed.is_empty() {
        ""
    } else if closed.ends_with([' ', '\t']) {
        closed.trim_end()
    } else {
        content
    };
    Some(content)
}

/// A setext underline: `=` or `-` characters only, trailing spaces allowed. An anchor does not depend on its level.
fn setext(text: &str) -> bool {
    let line = text.trim_end();
    line.as_bytes()
        .first()
        .is_some_and(|first| matches!(first, b'=' | b'-') && line.bytes().all(|b| b == *first))
}

/// Three or more `*`, `-` or `_`, with spaces between them and nothing else.
fn thematic_break(text: &str) -> bool {
    let mut marks = text.bytes().filter(|b| !matches!(b, b' ' | b'\t'));
    let Some(first) = marks.next() else {
        return false;
    };
    let count = 1 + marks.clone().count();
    matches!(first, b'*' | b'-' | b'_') && count >= 3 && marks.all(|b| b == first)
}

/// A code fence: three or more backticks or tildes; a backtick fence's info string has no backtick.
fn opens_fence(text: &str) -> Option<Fence> {
    let marker = *text.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let len = text.bytes().take_while(|b| *b == marker).count();
    let info = &text[len..];
    (len >= 3 && !(marker == b'`' && info.contains('`'))).then_some(Fence { marker, len })
}

/// Whether `line` closes `fence`: at most three spaces, then at least as many of its marker, then spaces only.
fn closes(line: &str, fence: Fence) -> bool {
    let indent = indent_of(line);
    if indent >= 4 {
        return false;
    }
    let text = line.trim_start();
    let len = text.bytes().take_while(|b| *b == fence.marker).count();
    len >= fence.len && text[len..].trim().is_empty()
}

/// The start of an HTML block, and its end marker when it has one (CommonMark's kinds 1 to 5); a tag of kind 6 or 7
/// ends at a blank line. A kind-7 tag does not interrupt a paragraph, so `None` with a paragraph open is plain text.
fn html_start(text: &str) -> Option<Option<&'static str>> {
    let lower = text.to_ascii_lowercase();
    if !lower.starts_with('<') {
        return None;
    }
    for (open, end) in [
        ("<script", "</script>"),
        ("<pre", "</pre>"),
        ("<style", "</style>"),
        ("<textarea", "</textarea>"),
    ] {
        if lower.starts_with(open)
            && (lower.len() == open.len() || lower[open.len()..].starts_with([' ', '>', '\t']))
        {
            return Some(Some(end));
        }
    }
    for (open, end) in [("<!--", "-->"), ("<?", "?>"), ("<![cdata[", "]]>")] {
        if lower.starts_with(open) {
            return Some(Some(end));
        }
    }
    if lower.starts_with("<!") && lower.as_bytes().get(2).is_some_and(u8::is_ascii_alphabetic) {
        return Some(Some(">"));
    }
    let name = lower[1..].strip_prefix('/').unwrap_or(&lower[1..]);
    let tag: String = name
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    (!tag.is_empty() && tag.as_bytes()[0].is_ascii_alphabetic()).then_some(None)
}
