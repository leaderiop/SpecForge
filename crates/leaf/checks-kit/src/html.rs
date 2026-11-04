//! The anchors of a static HTML page: a fragment's targets as the page was served, with no script run.

use std::collections::BTreeSet;

/// The elements whose content is text, so a `<` inside one starts no tag.
const RAW_TEXT: [&[u8]; 4] = [b"script", b"style", b"textarea", b"title"];

/// Every `id`, and every `name` of an `<a>`, in `bytes`. It never fails: what it cannot read names no anchor. Linear in
/// `bytes`, with no entity decoded, since a fragment is matched as written.
pub fn anchors(bytes: &[u8]) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut at = 0;
    while let Some(open) = find(bytes, at, b"<") {
        at = open + 1;
        if bytes[at..].starts_with(b"!--") {
            at = find(bytes, at + 3, b"-->").map_or(bytes.len(), |end| end + 3);
            continue;
        }
        let Some(first) = bytes.get(at) else { break };
        if !first.is_ascii_alphabetic() {
            continue;
        }
        let name_end = at
            + bytes[at..]
                .iter()
                .position(|b| b.is_ascii_whitespace() || *b == b'>' || *b == b'/')
                .unwrap_or(bytes.len() - at);
        let tag = bytes[at..name_end].to_ascii_lowercase();
        at = attributes(bytes, name_end, tag == b"a", &mut found);
        if RAW_TEXT.contains(&tag.as_slice()) {
            at = after_raw_text(bytes, at, &tag);
        }
    }
    found
}

/// Where the tag ends, having kept its anchors.
fn attributes(bytes: &[u8], mut at: usize, is_a: bool, found: &mut BTreeSet<String>) -> usize {
    loop {
        while bytes
            .get(at)
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b'/')
        {
            at += 1;
        }
        match bytes.get(at) {
            None => return at,
            Some(b'>') => return at + 1,
            Some(_) => {}
        }
        let name_from = at;
        // Past the first byte: a stray `=` is its own attribute name, and always moves on.
        at += 1;
        while bytes
            .get(at)
            .is_some_and(|b| !b.is_ascii_whitespace() && !matches!(b, b'=' | b'>' | b'/'))
        {
            at += 1;
        }
        let name = bytes[name_from..at].to_ascii_lowercase();
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        if bytes.get(at) != Some(&b'=') {
            continue;
        }
        at += 1;
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        let (value, next) = value_at(bytes, at);
        at = next;
        if name == b"id" || (is_a && name == b"name") {
            found.insert(String::from_utf8_lossy(value).into_owned());
        }
    }
}

/// The attribute value at `at`, and where it ends.
fn value_at(bytes: &[u8], at: usize) -> (&[u8], usize) {
    match bytes.get(at) {
        Some(quote @ (b'"' | b'\'')) => {
            let from = at + 1;
            match bytes[from..].iter().position(|b| b == quote) {
                Some(length) => (&bytes[from..from + length], from + length + 1),
                None => (&bytes[from..], bytes.len()),
            }
        }
        _ => {
            let length = bytes[at.min(bytes.len())..]
                .iter()
                .position(|b| b.is_ascii_whitespace() || *b == b'>')
                .unwrap_or(bytes.len().saturating_sub(at));
            (&bytes[at..at + length], at + length)
        }
    }
}

/// Past the `</tag>` that closes a raw-text element.
fn after_raw_text(bytes: &[u8], mut at: usize, tag: &[u8]) -> usize {
    while let Some(close) = find(bytes, at, b"</") {
        let name = close + 2;
        at = name;
        if bytes
            .get(name..name + tag.len())
            .is_some_and(|written| written.eq_ignore_ascii_case(tag))
        {
            return find(bytes, name, b">").map_or(bytes.len(), |end| end + 1);
        }
    }
    bytes.len()
}

fn find(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    let tail = bytes.get(from..)?;
    tail.windows(needle.len())
        .position(|window| window == needle)
        .map(|at| from + at)
}
