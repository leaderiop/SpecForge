//! The code book: the static catalog plus the loaded extensions' own
//! catalogs (their `codes` category). Every surface that titles, explains or
//! links a code reads it (ADR 0286).

use crate::extension::{CodeMisuse, check_extension_code};
use crate::namespace::{CodeRef, FIRST_PARTY, code_namespace};
use crate::{Level, lookup};
use std::borrow::Cow;
use std::collections::BTreeMap;

/// One code an extension catalogs. `local` is the code without its
/// namespace (`E101`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionCode {
    pub local: String,
    pub title: String,
    pub explanation: String,
    /// An absolute `http(s)` URL, when the extension documents the code.
    pub docs: Option<String>,
}

/// Two loaded extensions share one code namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedNamespace {
    pub namespace: String,
    /// The extension that owns the namespace (the first added).
    pub owner: String,
    /// The extension that was refused.
    pub other: String,
}

#[derive(Debug, Clone, Default)]
struct Owned {
    extension: String,
    codes: BTreeMap<String, ExtensionCode>,
}

/// Every code the host can describe. `Codebook::default()` reads the static
/// catalog only.
#[derive(Debug, Clone, Default)]
pub struct Codebook {
    namespaces: BTreeMap<String, Owned>,
}

/// What a code book says about one code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Described<'a> {
    /// The code as it prints.
    pub code: Cow<'a, str>,
    pub title: &'a str,
    /// `"core"`, a `@specforge/*` builtin, or the extension that owns the
    /// namespace.
    pub owner: &'a str,
    pub level: Level,
    pub explanation: &'a str,
    /// The page that documents the code: the `docs/diagnostics.md` anchor of
    /// a static code, the declared URL of an extension's, else none.
    pub docs: Option<Cow<'a, str>>,
}

/// The level a local code's letter states.
fn letter_level(local: &str) -> Option<Level> {
    match local.as_bytes().first()? {
        b'E' => Some(Level::Error),
        b'W' => Some(Level::Warning),
        b'I' => Some(Level::Info),
        _ => None,
    }
}

/// `[EWI]` and three digits.
fn is_local_code(code: &str) -> bool {
    let bytes = code.as_bytes();
    bytes.len() == 4
        && matches!(bytes[0], b'E' | b'W' | b'I')
        && bytes[1..].iter().all(u8::is_ascii_digit)
}

impl Codebook {
    /// Add `extension`'s catalog. A first-party extension has no namespace
    /// and adds nothing (its codes are in the static catalog). The first
    /// extension of a namespace owns it.
    pub fn add(
        &mut self,
        extension: &str,
        codes: Vec<ExtensionCode>,
    ) -> Result<(), SharedNamespace> {
        let Some(namespace) = code_namespace(extension) else {
            return Ok(());
        };
        let owned = self.namespaces.entry(namespace.clone()).or_default();
        if owned.extension.is_empty() {
            owned.extension = extension.to_string();
        } else if owned.extension != extension {
            return Err(SharedNamespace {
                namespace,
                owner: owned.extension.clone(),
                other: extension.to_string(),
            });
        }
        for code in codes {
            owned.codes.insert(code.local.to_uppercase(), code);
        }
        Ok(())
    }

    fn owned_by(&self, extension: &str) -> Option<&Owned> {
        let owned = self.namespaces.get(&code_namespace(extension)?)?;
        (owned.extension == extension).then_some(owned)
    }

    /// The entry that describes a diagnostic of `code` (any case) reported by
    /// `origin` (the extension that reported it; `None` when the host asks,
    /// as `explain` does): a static entry for the host or its owner, an
    /// extension's own entry for the owner of the code's namespace. A code an
    /// extension squats (W150) is not described by another's entry.
    pub fn describes(&self, code: &str, origin: Option<&str>) -> Option<Described<'_>> {
        let parsed = CodeRef::parse(code);
        let Some(namespace) = parsed.namespace else {
            let entry = lookup(code)?;
            if origin.is_some_and(|extension| entry.owner != extension) {
                return None;
            }
            return Some(Described {
                code: Cow::Borrowed(entry.code),
                title: entry.title,
                owner: entry.owner,
                level: entry.level,
                explanation: entry.explanation,
                docs: Some(Cow::Owned(entry.docs_href())),
            });
        };
        let namespace = namespace.to_lowercase();
        let owned = self.namespaces.get(&namespace)?;
        if origin.is_some_and(|extension| owned.extension != extension) {
            return None;
        }
        let local = parsed.local.to_uppercase();
        let entry = owned.codes.get(&local)?;
        Some(Described {
            code: Cow::Owned(format!("{namespace}/{local}")),
            title: &entry.title,
            owner: &owned.extension,
            level: letter_level(&local)?,
            explanation: &entry.explanation,
            docs: entry.docs.as_deref().map(Cow::Borrowed),
        })
    }

    /// Whether extension `extension` may report `code` at `level`.
    ///
    /// - A first-party extension is checked against the static catalog
    ///   (`check_extension_code`).
    /// - Any other extension reports a local code (`[EWI]` and three digits,
    ///   no `/`) its own catalog lists ([`CodeMisuse::Uncatalogued`]), at the
    ///   level its letter states ([`CodeMisuse::PrefixContradictsLevel`]).
    pub fn check(&self, extension: &str, code: &str, level: Level) -> Result<(), CodeMisuse> {
        if extension.starts_with(FIRST_PARTY) {
            return check_extension_code(extension, code, level);
        }
        if !is_local_code(code) {
            return Err(CodeMisuse::NotALocalCode);
        }
        if !self.catalogued(extension, code) {
            return Err(CodeMisuse::Uncatalogued);
        }
        match letter_level(code) {
            Some(letter) if letter == level => Ok(()),
            _ => Err(CodeMisuse::PrefixContradictsLevel),
        }
    }

    /// Whether `extension`'s own catalog lists `local`. False for a
    /// first-party extension (it declares no `codes`) and for an extension
    /// that does not own its namespace.
    pub fn catalogued(&self, extension: &str, local: &str) -> bool {
        self.owned_by(extension)
            .is_some_and(|owned| owned.codes.contains_key(&local.to_uppercase()))
    }

    /// The level a diagnostic of `code` (as it prints: `gsd/W304`, `E001`)
    /// is reported at: a static entry's catalogued level (`None` for `A###`,
    /// whose pass sets it), a namespaced code's letter when its namespace's
    /// owner catalogues it, else `None`.
    pub fn letter(&self, code: &str) -> Option<Level> {
        match CodeRef::parse(code).namespace {
            None => match lookup(code)?.level {
                Level::SetByPass => None,
                level => Some(level),
            },
            Some(_) => self.describes(code, None).map(|described| described.level),
        }
    }
}
