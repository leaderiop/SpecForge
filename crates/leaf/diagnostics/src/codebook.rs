//! The code book: the static catalog plus the loaded extensions' own
//! catalogs (their `codes` category). Every surface that titles, explains or
//! links a code reads it (ADR 0286).

use crate::extension::CodeMisuse;
use crate::namespace::{CodeRef, FIRST_PARTY, code_namespace, is_local_code};
use crate::{Level, lookup, retired};
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
    /// - A first-party extension reports only the static catalog's codes it
    ///   owns, at their catalogued level (at any level for an `A###` code,
    ///   whose pass sets it); core's or another owner's code is
    ///   [`CodeMisuse::NotItsCode`], a retired one [`CodeMisuse::Retired`], a
    ///   code the catalog lacks [`CodeMisuse::Uncatalogued`].
    /// - Any other extension reports a local code (`[EWI]` and three digits,
    ///   no `/`; [`CodeMisuse::NotALocalCode`]) its own catalog lists
    ///   ([`CodeMisuse::Uncatalogued`]), at the level its letter states
    ///   ([`CodeMisuse::PrefixContradictsLevel`]). Its code never reaches
    ///   core's or another extension's: the host prints it under its
    ///   namespace.
    pub fn check(&self, extension: &str, code: &str, level: Level) -> Result<(), CodeMisuse> {
        if extension.starts_with(FIRST_PARTY) {
            return first_party(extension, code, level);
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

    /// The qualified forms (`gsd/W304`) of a bare `local` code that loaded
    /// extensions catalog, namespace by namespace: what a bare code asked
    /// for most likely meant.
    pub fn qualified(&self, local: &str) -> Vec<String> {
        let local = local.to_uppercase();
        self.namespaces
            .iter()
            .filter(|(_, owned)| owned.codes.contains_key(&local))
            .map(|(namespace, _)| format!("{namespace}/{local}"))
            .collect()
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

/// The static catalog's verdict on a `@specforge/*` extension reporting `code`.
fn first_party(extension: &str, code: &str, level: Level) -> Result<(), CodeMisuse> {
    if let Some(entry) = lookup(code) {
        if entry.owner != extension {
            return Err(CodeMisuse::NotItsCode {
                owner: entry.owner,
                title: entry.title,
            });
        }
        return match entry.level {
            Level::SetByPass => Ok(()),
            catalogued if catalogued == level => Ok(()),
            catalogued => Err(CodeMisuse::WrongLevel { catalogued }),
        };
    }
    match retired(code) {
        Some(replaced_by) => Err(CodeMisuse::Retired { replaced_by }),
        None => Err(CodeMisuse::Uncatalogued),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;

    fn book(extension: &str, codes: &[&str]) -> Codebook {
        let mut book = Codebook::default();
        let entries = codes
            .iter()
            .map(|code| ExtensionCode {
                local: (*code).into(),
                title: "t".into(),
                explanation: "e".into(),
                docs: None,
            })
            .collect();
        book.add(extension, entries).unwrap();
        book
    }

    #[specforge_test(
        invariant = "diagnostic_code_uniqueness",
        verify = "an extension reports only its own catalogued codes, at the level the catalog or the letter states"
    )]
    fn a_core_code_is_not_a_first_party_extensions() {
        let none = Codebook::default();
        let misuse = none
            .check("@specforge/product", "E001", Level::Error)
            .unwrap_err();
        assert_eq!(
            misuse,
            CodeMisuse::NotItsCode {
                owner: "core",
                title: "Parse error"
            }
        );
        let text = misuse.to_string();
        assert!(text.contains("core owns"), "{text}");
        assert!(text.contains("Parse error"), "{text}");
        let other = none
            .check("@specforge/rust", "W041", Level::Warning)
            .unwrap_err();
        assert!(
            other.to_string().contains("@specforge/product owns"),
            "{other}"
        );
    }

    #[specforge_test(
        invariant = "diagnostic_code_uniqueness",
        verify = "an extension reports only its own catalogued codes, at the level the catalog or the letter states"
    )]
    fn a_first_party_extensions_own_code_at_its_level_is_accepted() {
        let none = Codebook::default();
        assert_eq!(
            none.check("@specforge/product", "W041", Level::Warning),
            Ok(())
        );
        assert_eq!(
            none.check("@specforge/product", "I068", Level::Info),
            Ok(())
        );
    }

    #[specforge_test(
        invariant = "diagnostic_code_uniqueness",
        verify = "an extension reports only its own catalogued codes, at the level the catalog or the letter states"
    )]
    fn a_first_party_code_at_another_level_is_refused() {
        let misuse = Codebook::default()
            .check("@specforge/product", "W041", Level::Error)
            .unwrap_err();
        assert_eq!(
            misuse,
            CodeMisuse::WrongLevel {
                catalogued: Level::Warning
            }
        );
        assert!(misuse.to_string().contains("warning"), "{misuse}");
    }

    #[specforge_test(
        invariant = "diagnostic_code_uniqueness",
        verify = "an extension reports only its own catalogued codes, at the level the catalog or the letter states"
    )]
    fn a_graded_code_accepts_any_level() {
        // The testing extension's A-codes are graded by its pass.
        let entry = lookup("A001").expect("A001 is catalogued");
        assert_eq!(entry.level, Level::SetByPass);
        for level in [Level::Error, Level::Warning, Level::Info] {
            assert_eq!(
                Codebook::default().check(entry.owner, "A001", level),
                Ok(()),
                "{level:?}"
            );
        }
    }

    #[specforge_test(
        invariant = "diagnostic_code_uniqueness",
        verify = "an extension reports only its own catalogued codes, at the level the catalog or the letter states"
    )]
    fn a_retired_code_is_refused_to_a_first_party_extension() {
        let none = Codebook::default();
        assert_eq!(
            none.check("@specforge/product", "E047", Level::Error),
            Err(CodeMisuse::Retired {
                replaced_by: Some("W139")
            })
        );
        assert_eq!(
            none.check("@specforge/product", "W024", Level::Warning),
            Err(CodeMisuse::Retired { replaced_by: None })
        );
    }

    #[specforge_test(
        invariant = "diagnostic_code_uniqueness",
        verify = "an extension reports only its own catalogued codes, at the level the catalog or the letter states"
    )]
    fn a_first_party_uncatalogued_code_is_refused() {
        let none = Codebook::default();
        for code in ["W500", "W950"] {
            assert_eq!(
                none.check("@specforge/rust", code, Level::Warning),
                Err(CodeMisuse::Uncatalogued),
                "{code}"
            );
        }
    }

    // Flips K10-D3: an extension owns its whole number space, `E101` and `E001` included.
    #[specforge_test(
        invariant = "diagnostic_code_uniqueness",
        verify = "a third-party extension's code is its namespace and its own local code"
    )]
    fn a_third_party_extension_owns_its_whole_number_space() {
        let gsd = book("@gsd/core", &["E101", "E001"]);
        assert_eq!(gsd.check("@gsd/core", "E101", Level::Error), Ok(()));
        // Its own `E001`, never core's: the host prints it `gsd/E001`.
        assert_eq!(gsd.check("@gsd/core", "E001", Level::Error), Ok(()));
        assert_eq!(
            gsd.check("@gsd/core", "E102", Level::Error),
            Err(CodeMisuse::Uncatalogued)
        );
        assert_eq!(
            gsd.check("@gsd/core", "gsd/E101", Level::Error),
            Err(CodeMisuse::NotALocalCode)
        );
        for code in ["", "A001", "e101", "E1011", "X101"] {
            assert_eq!(
                gsd.check("@gsd/core", code, Level::Error),
                Err(CodeMisuse::NotALocalCode),
                "{code:?}"
            );
        }
        // Another extension's catalog is not this one's.
        assert_eq!(
            gsd.check("@acme/x", "E101", Level::Error),
            Err(CodeMisuse::Uncatalogued)
        );
    }

    #[specforge_test(
        invariant = "diagnostic_code_uniqueness",
        verify = "an extension reports only its own catalogued codes, at the level the catalog or the letter states"
    )]
    fn a_third_party_letter_states_its_level() {
        let acme = book("@acme/x", &["W950", "E950", "I950"]);
        for (code, level) in [
            ("W950", Level::Error),
            ("E950", Level::Info),
            ("I950", Level::Warning),
        ] {
            assert_eq!(
                acme.check("@acme/x", code, level),
                Err(CodeMisuse::PrefixContradictsLevel),
                "{code}"
            );
        }
        assert_eq!(acme.check("@acme/x", "W950", Level::Warning), Ok(()));
    }
}
