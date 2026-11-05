//! Research identifiers and their resolvers: a DOI in any spelling a citation writes it (`10.1000/182`, `doi:…`,
//! `https://doi.org/…`, `dx.doi.org`), case-folded, and an ORCID iD, bare or as its URL, with its ISO 7064 MOD 11-2
//! check digit.

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use crate::Invalid;
use crate::digits::iso7064::mod11_2;

/// The prefixes a DOI is written after, matched without case.
const DOI_PREFIXES: [&str; 5] = [
    "https://doi.org/",
    "http://doi.org/",
    "https://dx.doi.org/",
    "http://dx.doi.org/",
    "doi:",
];

/// What a DOI keeps unencoded in the handle API's path: the DOI is one path segment, its `/` encoded (the handle API
/// reads `%2F` as `/`), so no `..` in a suffix can climb out of `api/handles/` to name another DOI or endpoint.
const DOI_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// The prefix of `text` among `prefixes`, matched without case: its length.
fn prefixed(text: &str, prefixes: &[&str]) -> usize {
    prefixes
        .iter()
        .find(|p| {
            text.len() >= p.len()
                && text.is_char_boundary(p.len())
                && text[..p.len()].eq_ignore_ascii_case(p)
        })
        .map_or(0, |p| p.len())
}

/// The DOI `text` writes, case-folded: `10.` and a registrant code of digits (with dots), `/`, and a suffix.
pub fn doi(text: &str) -> Result<String, Invalid> {
    let start = prefixed(text, &DOI_PREFIXES);
    let name = &text[start..];
    let at = start..text.len();
    let Some((prefix, suffix)) = name.split_once('/') else {
        return Err(Invalid::at("a DOI is '10.<registrant>/<suffix>'", at));
    };
    let registrant = prefix.strip_prefix("10.").unwrap_or("");
    let digits_and_dots = !registrant.is_empty()
        && registrant
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()));
    if !digits_and_dots {
        return Err(Invalid::at(
            "a DOI's prefix is '10.' and a registrant code of digits",
            start..start + prefix.len(),
        ));
    }
    if suffix.is_empty() || suffix.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(Invalid::at(
            "a DOI's suffix is not empty and holds no space",
            start + prefix.len()..text.len(),
        ));
    }
    Ok(name.to_ascii_lowercase())
}

/// Where the DOI handle API answers whether `doi` is registered (`responseCode` 1) or not (100). The DOI is one
/// segment of the path, which starts `10.`, so it is never a dot segment either.
pub fn doi_url(doi: &str) -> String {
    format!(
        "https://doi.org/api/handles/{}",
        utf8_percent_encode(doi, DOI_SEGMENT)
    )
}

/// Whether the handle API's answer says the DOI is registered: `responseCode` 1 is, 100 is not, and another code is
/// taken as registered. An answer with no code does not read.
pub fn doi_registered(body: &[u8]) -> Result<bool, String> {
    #[derive(serde::Deserialize)]
    struct Handle {
        #[serde(rename = "responseCode")]
        response_code: i64,
    }
    let handle: Handle = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    Ok(handle.response_code != 100)
}

/// The ORCID iD `text` writes, bare or after `https://orcid.org/`: four groups of four, the last ending in a digit or
/// `X`, whose MOD 11-2 check holds.
pub fn orcid(text: &str) -> Result<String, Invalid> {
    let start = prefixed(
        text,
        &["https://orcid.org/", "http://orcid.org/", "orcid.org/"],
    );
    let id = &text[start..];
    let at = start..text.len();
    let shaped = id.len() == 19
        && id.char_indices().all(|(i, c)| match i {
            4 | 9 | 14 => c == '-',
            18 => c.is_ascii_digit() || c == 'X',
            _ => c.is_ascii_digit(),
        });
    if !shaped {
        return Err(Invalid::at(
            "an ORCID iD is four groups of four digits, the last digit or 'X'",
            at,
        ));
    }
    mod11_2(id).map_err(|invalid| invalid.shifted(start, at))?;
    Ok(id.to_string())
}

/// Where ORCID's public API answers the record of `id`.
pub fn orcid_url(id: &str) -> String {
    format!("https://pub.orcid.org/v3.0/{id}/record")
}
