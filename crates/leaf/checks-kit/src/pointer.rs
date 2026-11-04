//! JSON Pointers (RFC 6901) in their URI-fragment form (§6): the fragment is percent-decoded, then each reference token
//! reads `~1` as `/` and `~0` as `~`.

use percent_encoding::percent_decode_str;
use serde_json::Value;

use crate::Invalid;

/// The reference tokens of `fragment`, the text after a `#`. An empty fragment points at the whole document.
pub fn tokens(fragment: &str) -> Result<Vec<String>, Invalid> {
    let decoded = percent_decode_str(fragment).decode_utf8().map_err(|_| {
        Invalid::new("the pointer is not UTF-8 once its percent escapes are decoded")
    })?;
    if decoded.is_empty() {
        return Ok(Vec::new());
    }
    let Some(rest) = decoded.strip_prefix('/') else {
        return Err(Invalid::at(
            "a JSON pointer starts with '/' (RFC 6901)",
            0..fragment.len(),
        ));
    };
    rest.split('/').map(unescape).collect()
}

/// One reference token: `~1` is `/`, `~0` is `~`, and any other `~` is malformed.
fn unescape(token: &str) -> Result<String, Invalid> {
    let mut out = String::with_capacity(token.len());
    let mut chars = token.chars();
    while let Some(c) = chars.next() {
        if c != '~' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('0') => out.push('~'),
            Some('1') => out.push('/'),
            _ => {
                return Err(Invalid::new(format!(
                    "'~' in the token '{token}' is not '~0' or '~1' (RFC 6901)"
                )));
            }
        }
    }
    Ok(out)
}

/// The value `tokens` point at in `document`; else how many tokens lead to a value that is there.
pub fn resolve<'v>(document: &'v Value, tokens: &[String]) -> Result<&'v Value, usize> {
    let mut at = document;
    for (index, token) in tokens.iter().enumerate() {
        let next = match at {
            Value::Object(members) => members.get(token),
            Value::Array(items) => index_of(token).and_then(|i| items.get(i)),
            _ => None,
        };
        at = next.ok_or(index)?;
    }
    Ok(at)
}

/// An array index: `0`, or digits with no leading zero.
fn index_of(token: &str) -> Option<usize> {
    let digits = !token.is_empty() && token.bytes().all(|b| b.is_ascii_digit());
    (digits && (token == "0" || !token.starts_with('0')))
        .then(|| token.parse().ok())
        .flatten()
}

/// The pointer of the first `count` tokens, written back in RFC 6901's form.
pub fn written(tokens: &[String], count: usize) -> String {
    tokens[..count.min(tokens.len())]
        .iter()
        .map(|token| format!("/{}", token.replace('~', "~0").replace('/', "~1")))
        .collect()
}
