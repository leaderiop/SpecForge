//! A URL read by the WHATWG parser the host requests it with, with the bytes of the text a finding points at.

use std::ops::Range;

use url::Url;

use crate::Invalid;

/// A URL that parsed, and where its parts are written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlParts {
    pub url: Url,
    pub scheme: Range<usize>,
    /// The bytes of a user name or password, when it has one.
    pub credentials: Option<Range<usize>>,
    /// The bytes after `#`, when it has a fragment.
    pub fragment: Option<Range<usize>>,
}

impl UrlParts {
    /// `text` as an absolute URL. The parser drops spaces and control characters from a value, which would then ask for
    /// another URL than the one written, so those are refused here.
    pub fn parse(text: &str) -> Result<UrlParts, Invalid> {
        if let Some((at, ch)) = text
            .char_indices()
            .find(|(_, ch)| ch.is_control() || ch.is_whitespace())
        {
            return Err(Invalid::at(
                "a URL holds no spaces or control characters",
                at..at + ch.len_utf8(),
            ));
        }
        let url = Url::parse(text).map_err(|error| Invalid::new(error.to_string()))?;
        let scheme = 0..text.find(':').unwrap_or(0);
        let credentials = (!url.username().is_empty() || url.password().is_some()).then(|| {
            let from = text.find("//").map_or(scheme.end + 1, |at| at + 2);
            let authority = text[from..]
                .find(['/', '\\', '?', '#'])
                .map_or(text.len(), |end| from + end);
            from..text[from..authority]
                .rfind('@')
                .map_or(from, |at| from + at)
        });
        let fragment = text.find('#').map(|at| at + 1..text.len());
        Ok(UrlParts {
            url,
            scheme,
            credentials,
            fragment,
        })
    }
}

/// Whether `host` is `pattern`, or under it when `pattern` is `*.<domain>`. Both are compared as a parsed URL prints them.
pub fn host_matches(pattern: &str, host: &str) -> bool {
    let pattern = pattern.to_ascii_lowercase();
    let host = host.to_ascii_lowercase();
    match pattern.strip_prefix("*.") {
        Some(domain) => host
            .strip_suffix(domain)
            .is_some_and(|under| under.ends_with('.') && under.len() > 1),
        None => pattern == host,
    }
}
