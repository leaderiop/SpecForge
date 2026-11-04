//! An SVG file read as XML with `quick-xml`: whether it is well formed, whether its root element is `svg`, and the first
//! active content it carries: a `script` element, an event attribute (`on…`), a `javascript:` URL in an `href` or in
//! what an animation sets, or an entity this reader does not expand where one could hide either.
//!
//! The file is untrusted: no entity is expanded and nesting is held to [`MAX_DEPTH`].

use quick_xml::Reader;
use quick_xml::escape::unescape;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};

/// The deepest elements nest.
pub const MAX_DEPTH: usize = 256;

/// A place in the file, both counted from 1; the column in characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct At {
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for At {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Why a file is no SVG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// It is not well-formed XML.
    NotXml { reason: String, at: At },
    /// Its root element is another.
    NotSvg { root: String },
    /// It has no element.
    NoRoot,
}

/// Active content, and where it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Active {
    pub what: String,
    pub at: At,
}

/// The place of byte `offset` of `bytes`.
fn at(bytes: &[u8], offset: u64) -> At {
    let offset = usize::try_from(offset)
        .unwrap_or(usize::MAX)
        .min(bytes.len());
    let before = &bytes[..offset];
    let line_start = before
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(0, |i| i + 1);
    At {
        line: before.iter().filter(|b| **b == b'\n').count() + 1,
        column: String::from_utf8_lossy(&before[line_start..])
            .chars()
            .count()
            + 1,
    }
}

/// A name's local part (after any prefix), as text.
fn local(name: &str) -> String {
    name.rsplit(':').next().unwrap_or(name).to_string()
}

/// Whether a written URL runs script: its scheme, once tabs and newlines are dropped and leading spaces and controls
/// trimmed as a browser does, is `javascript:`.
fn runs_script(value: &str) -> bool {
    let kept: String = value
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .skip_while(|c| *c <= ' ')
        .take("javascript:".len())
        .collect();
    kept.eq_ignore_ascii_case("javascript:")
}

/// Whether `raw` refers to an entity other than XML's five and character references.
fn unknown_entity(raw: &str) -> Option<String> {
    raw.split('&').skip(1).find_map(|rest| {
        let name = rest.split(';').next().unwrap_or(rest);
        let known = matches!(name, "lt" | "gt" | "amp" | "apos" | "quot") || name.starts_with('#');
        (!known).then(|| format!("&{name};"))
    })
}

/// The active content of one attribute of `element`, if any.
fn active_attribute(element: &str, attribute: &Attribute<'_>) -> Option<String> {
    let name = local(attribute.key.as_ref());
    let raw = attribute.value.as_ref();
    // Unescaped but not whitespace-normalized: an SVG inlined in HTML keeps a tab or a newline, which a URL drops.
    let value = unescape(raw).map_or_else(|_| raw.to_string(), |v| v.into_owned());
    let lower = name.to_ascii_lowercase();
    if lower.len() > 2 && lower.starts_with("on") {
        return Some(format!("an event attribute '{name}' on <{element}>"));
    }
    let url = lower == "href";
    let animated = matches!(lower.as_str(), "to" | "from" | "values");
    if url && runs_script(&value) {
        return Some(format!("a javascript: URL in '{name}' on <{element}>"));
    }
    if animated && value.split(';').any(runs_script) {
        return Some(format!(
            "a javascript: URL that <{element}> sets in '{name}'"
        ));
    }
    if (url || animated)
        && let Some(entity) = unknown_entity(raw)
    {
        return Some(format!(
            "the entity '{entity}' in '{name}' on <{element}>, which is not expanded to be read"
        ));
    }
    None
}

/// The active content of an element: itself, else its first attribute that is.
fn active_element(element: &BytesStart<'_>) -> Result<Option<String>, String> {
    let name = local(element.name().as_ref());
    if name.eq_ignore_ascii_case("script") {
        return Ok(Some("a <script> element".to_string()));
    }
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|e| e.to_string())?;
        if let Some(what) = active_attribute(&name, &attribute) {
            return Ok(Some(what));
        }
    }
    Ok(None)
}

/// Reads `bytes` as an SVG: its first active content, or none; or why it is no SVG.
pub fn scan(bytes: &[u8]) -> Result<Option<Active>, Refusal> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_comments = true;
    let mut depth = 0usize;
    let mut rooted = false;
    let mut first: Option<Active> = None;
    let refused = |reason: String, offset: u64| Refusal::NotXml {
        reason,
        at: at(bytes, offset),
    };
    loop {
        let start = reader.buffer_position();
        let event = reader
            .read_event()
            .map_err(|e| refused(e.to_string(), reader.error_position()))?;
        let opens = matches!(event, Event::Start(_));
        match event {
            Event::Start(element) | Event::Empty(element) => {
                if depth == 0 {
                    if rooted {
                        return Err(refused("a second root element".to_string(), start));
                    }
                    rooted = true;
                    let root = local(element.name().as_ref());
                    if root != "svg" {
                        return Err(Refusal::NotSvg { root });
                    }
                }
                let what = active_element(&element).map_err(|reason| refused(reason, start))?;
                if let (None, Some(what)) = (&first, what) {
                    first = Some(Active {
                        what,
                        at: at(bytes, start),
                    });
                }
                if opens {
                    depth += 1;
                    if depth > MAX_DEPTH {
                        return Err(refused(
                            format!("elements nest deeper than {MAX_DEPTH}"),
                            start,
                        ));
                    }
                }
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            Event::Text(text) if depth == 0 => {
                if !text.bytes().all(|b| b.is_ascii_whitespace()) {
                    return Err(refused("text outside the root element".to_string(), start));
                }
            }
            Event::CData(_) | Event::GeneralRef(_) if depth == 0 => {
                return Err(refused("text outside the root element".to_string(), start));
            }
            Event::GeneralRef(reference) => {
                let name = String::from(&*reference);
                let known = matches!(name.as_str(), "lt" | "gt" | "amp" | "apos" | "quot")
                    || name.starts_with('#');
                if !known && first.is_none() {
                    first = Some(Active {
                        what: format!("the entity '&{name};', which is not expanded to be read"),
                        at: at(bytes, start),
                    });
                }
            }
            Event::Eof if depth > 0 => {
                return Err(refused("it ends inside an element".to_string(), start));
            }
            Event::Eof if !rooted => return Err(Refusal::NoRoot),
            Event::Eof => return Ok(first),
            _ => {}
        }
    }
}

/// Whether `head`, the first bytes of a file, starts an SVG: its first element, after any declaration, comment,
/// processing instruction or doctype, is `svg`.
pub fn is_svg(head: &[u8]) -> bool {
    let mut reader = Reader::from_reader(head);
    loop {
        match reader.read_event() {
            Ok(Event::Start(element) | Event::Empty(element)) => {
                return local(element.name().as_ref()) == "svg";
            }
            Ok(Event::Text(text)) if text.bytes().all(|b| b.is_ascii_whitespace()) => {}
            Ok(Event::Decl(_) | Event::Comment(_) | Event::PI(_) | Event::DocType(_)) => {}
            _ => return false,
        }
    }
}
