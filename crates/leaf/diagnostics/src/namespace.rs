//! A third-party code's text: `<namespace>/<letter><three digits>`. The host
//! computes the namespace from the extension's package name and adds it where
//! a code enters (rule registration, pass findings); core's and the
//! `@specforge/*` builtins' codes stay bare (ADR 0286).

/// The scope of the extensions the project ships and documents.
pub(crate) const FIRST_PARTY: &str = "@specforge/";

/// The namespace of extension `extension` (its declared name): `@gsd/core`
/// is `gsd`, `@gsd/security` is `gsd.security`, a local `lint` is `lint`, and
/// a `@specforge/*` builtin has none (its codes are bare). A name that is no
/// package name is lower-cased, its `@` dropped and its `/` read as `.`.
pub fn code_namespace(extension: &str) -> Option<String> {
    if extension.starts_with(FIRST_PARTY) {
        return None;
    }
    let name = extension.trim_start_matches('@').to_lowercase();
    Some(match name.split_once('/') {
        Some((scope, "core")) => scope.to_string(),
        _ => name.replace('/', "."),
    })
}

/// `extension`'s code `local` as it prints: `<namespace>/<local>`, or
/// `local` unchanged for a first-party extension.
pub fn qualify(extension: &str, local: &str) -> String {
    match code_namespace(extension) {
        Some(namespace) => format!("{namespace}/{local}"),
        None => local.to_string(),
    }
}

/// A code split at its `/`: `gsd/E101` is (`Some("gsd")`, `"E101"`), `E101`
/// is (`None`, `"E101"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeRef<'a> {
    pub namespace: Option<&'a str>,
    pub local: &'a str,
}

impl<'a> CodeRef<'a> {
    pub fn parse(text: &'a str) -> CodeRef<'a> {
        match text.split_once('/') {
            Some((namespace, local)) => CodeRef {
                namespace: Some(namespace),
                local,
            },
            None => CodeRef {
                namespace: None,
                local: text,
            },
        }
    }

    /// The code in its one spelling: namespace lower case, local upper case
    /// (`explain` takes any case).
    pub fn normalized(text: &str) -> String {
        let parsed = CodeRef::parse(text);
        match parsed.namespace {
            Some(namespace) => format!(
                "{}/{}",
                namespace.to_lowercase(),
                parsed.local.to_uppercase()
            ),
            None => parsed.local.to_uppercase(),
        }
    }
}

/// Whether `text` is a local code: `E`, `W` or `I` and three digits, no `/`.
pub fn is_local_code(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 4
        && matches!(bytes[0], b'E' | b'W' | b'I')
        && bytes[1..].iter().all(u8::is_ascii_digit)
}
