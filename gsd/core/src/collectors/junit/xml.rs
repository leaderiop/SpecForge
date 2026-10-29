//! The least XML a JUnit report needs: the open and close tags of its elements, with decoded attributes. Text,
//! comments, CDATA, processing instructions and a doctype are skipped (a test's output, which may hold anything,
//! is text or CDATA). It reads nothing it cannot read to the end: a tag or comment that never closes is an error,
//! so a truncated report is no report.

/// One tag of the document.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    /// `<name a="b">`, or `<name a="b"/>` when `empty`.
    Open {
        name: String,
        attrs: Vec<(String, String)>,
        empty: bool,
    },
    /// `</name>`.
    Close(String),
}

/// `&amp;` `&lt;` `&gt;` `&quot;` `&apos;` and numeric references, decoded; any other `&...;` is left as written.
pub fn decode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        // An entity is short: a `;` further than 12 characters away is not its end (and a search to the end of a
        // long value for every `&` would be quadratic).
        let end = rest.char_indices().take(12).find(|(_, c)| *c == ';');
        let decoded = end.map(|(end, _)| end).and_then(|end| {
            let name = &rest[1..end];
            let value = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => name.strip_prefix('#').and_then(|number| {
                    let code = match number.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                        None => number.parse().ok()?,
                    };
                    char::from_u32(code)
                }),
            }?;
            Some((value, end + 1))
        });
        match decoded {
            Some((value, used)) => {
                out.push(value);
                rest = &rest[used..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The tags of `text`, in order.
pub fn events(text: &str) -> Result<Vec<Event>, String> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(start) = text[at..].find('<').map(|i| at + i) {
        let rest = &text[start..];
        let skip_to = |end: &str| -> Result<usize, String> {
            rest.find(end)
                .map(|i| start + i + end.len())
                .ok_or_else(|| format!("`{end}` is missing after byte {start}"))
        };
        if rest.starts_with("<!--") {
            at = skip_to("-->")?;
        } else if rest.starts_with("<![CDATA[") {
            at = skip_to("]]>")?;
        } else if rest.starts_with("<?") {
            at = skip_to("?>")?;
        } else if rest.starts_with("<!") {
            at = skip_to(">")?;
        } else if let Some(closing) = rest.strip_prefix("</") {
            let end = closing
                .find('>')
                .ok_or_else(|| format!("a closing tag is not closed at byte {start}"))?;
            found.push(Event::Close(closing[..end].trim().to_string()));
            at = start + 2 + end + 1;
        } else {
            let (event, used) =
                open_tag(&rest[1..]).map_err(|why| format!("{why} at byte {start}"))?;
            found.push(event);
            at = start + 1 + used;
        }
    }
    Ok(found)
}

/// The tag after its `<`, and the bytes it took, `>` included.
fn open_tag(tag: &str) -> Result<(Event, usize), String> {
    let bytes = tag.as_bytes();
    let name_end = bytes
        .iter()
        .position(|b| b.is_ascii_whitespace() || *b == b'/' || *b == b'>')
        .ok_or("a tag is not closed")?;
    let name = tag[..name_end].to_string();
    let mut at = name_end;
    let mut attrs = Vec::new();
    loop {
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        match bytes.get(at) {
            Some(b'>') => {
                return Ok((
                    Event::Open {
                        name,
                        attrs,
                        empty: false,
                    },
                    at + 1,
                ));
            }
            Some(b'/') if bytes.get(at + 1) == Some(&b'>') => {
                return Ok((
                    Event::Open {
                        name,
                        attrs,
                        empty: true,
                    },
                    at + 2,
                ));
            }
            None => return Err("a tag is not closed".to_string()),
            _ => {}
        }
        let key_end = tag[at..]
            .find(|c: char| c == '=' || c.is_ascii_whitespace() || c == '>' || c == '/')
            .map(|i| at + i)
            .ok_or("an attribute is not closed")?;
        let key = tag[at..key_end].to_string();
        at = key_end;
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        if bytes.get(at) != Some(&b'=') {
            return Err(format!("attribute `{key}` has no value"));
        }
        at += 1;
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        let quote = match bytes.get(at) {
            Some(q @ (b'"' | b'\'')) => *q,
            _ => return Err(format!("attribute `{key}` is not quoted")),
        };
        let value_start = at + 1;
        let value_end = tag[value_start..]
            .bytes()
            .position(|b| b == quote)
            .map(|i| value_start + i)
            .ok_or_else(|| format!("attribute `{key}` is not closed"))?;
        attrs.push((key, decode(&tag[value_start..value_end])));
        at = value_end + 1;
    }
}
