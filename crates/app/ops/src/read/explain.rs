//! `specforge explain` and `specforge.explain`: what the code book says of one code, the static
//! catalog's or a loaded extension's own (ADR 0286), one operation whose outcome is the document
//! both surfaces print (`McpExplainResult`, ADR 0052).

use serde::Serialize;
use specforge_common::shape::Shape;
use specforge_diagnostics::{CodeRef, Codebook, Described, retired};

use crate::{OpError, OpErrorKind, Refusal};

/// The refusal of a code the code book neither has nor retired.
pub const UNKNOWN_CODE: &str = "unknown_code";

/// The forms a code takes: the suggestion of the refusal of an unknown one.
const CODE_FORMS: &str = "codes are E### (error), W### (warning), I### (info), A### (analyze finding), \
     R### and R-<AREA>-### (registry client); an extension's code is its namespace and a code \
     (`acme.lint/W101`), explained inside a project that loads the extension";

/// What the code book says of a code: its entry (the catalog's, or its extension's own), or that
/// it was retired and what replaced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
#[serde(untagged)]
pub enum Explanation {
    /// A code the catalog has.
    Catalogued(Catalogued),
    /// A code the catalog retired.
    Retired(Retired),
}

/// A catalogued code: its entry, `retired: false`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct Catalogued {
    #[serde(flatten)]
    pub entry: Entry,
    pub retired: bool,
}

/// A retired code, upper-cased, `retired: true`, and the entry of the code that replaced it, if
/// one did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct Retired {
    pub code: String,
    pub retired: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaced_by: Option<Entry>,
}

/// One catalogue entry as both surfaces print it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct Entry {
    pub code: String,
    pub title: String,
    /// `core`, or the extension that owns the code.
    pub owner: String,
    /// The level's description (`error`, `warning`, `info`, `set by the analyze pass`).
    pub level: String,
    pub explanation: String,
    /// Its section of `docs/diagnostics.md`, or the page its extension documents it on; an
    /// extension's code may have none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
}

impl Entry {
    fn of(described: Described<'_>) -> Self {
        Entry {
            code: described.code.into_owned(),
            title: described.title.to_string(),
            owner: described.owner.to_string(),
            level: described.level.describe().to_string(),
            explanation: described.explanation.to_string(),
            docs: described.docs.map(std::borrow::Cow::into_owned),
        }
    }
}

/// The code book's answer for `code`: any case, surrounding whitespace ignored; a retired code
/// names what replaced it. A code the book lacks is `unknown_code` (invalid input), with the code
/// forms as its suggestion, or the loaded `<namespace>/<code>` forms when a bare local code asked
/// is one an extension catalogs (ADR 0286).
pub fn explain(code: &str, codes: &Codebook) -> Result<Explanation, Refusal<ExplainField>> {
    let code = code.trim();
    let normalized = CodeRef::normalized(code);
    if let Some(replacement) = retired(&normalized) {
        return Ok(Explanation::Retired(Retired {
            code: normalized,
            retired: true,
            replaced_by: replacement
                .and_then(|r| codes.describes(r, None))
                .map(Entry::of),
        }));
    }
    match codes.describes(&normalized, None) {
        Some(described) => Ok(Explanation::Catalogued(Catalogued {
            entry: Entry::of(described),
            retired: false,
        })),
        None => {
            let suggestion = match codes.qualified(&normalized).as_slice() {
                [] => CODE_FORMS.to_string(),
                forms => format!("{CODE_FORMS}; did you mean {}?", forms.join(" or ")),
            };
            Err(OpError::new(
                OpErrorKind::InvalidInput,
                UNKNOWN_CODE,
                format!("unknown diagnostic code: {code}"),
            )
            .with_suggestion(suggestion)
            .about(ExplainField::Code))
        }
    }
}

/// The field of an explain request a refusal can be about (ADR 0072).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplainField {
    /// The diagnostic code asked about.
    Code,
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_diagnostics::ExtensionCode;
    use specforge_test_macros::test as specforge_test;

    /// A code book where each extension catalogs its locals, each titled "Audit failed".
    fn book(extensions: &[(&str, &[&str])]) -> Codebook {
        let mut book = Codebook::default();
        for (extension, locals) in extensions {
            let entries = locals
                .iter()
                .map(|local| ExtensionCode {
                    local: (*local).into(),
                    title: "Audit failed".into(),
                    explanation: "Fix the audit.".into(),
                    docs: None,
                })
                .collect();
            book.add(extension, entries).unwrap();
        }
        book
    }

    fn none() -> Codebook {
        Codebook::default()
    }

    #[specforge_test(
        behavior = "explain_diagnostic_code",
        verify = "specforge explain reads the code as specforge.explain does: any case, surrounding whitespace ignored"
    )]
    fn explain_answers_the_entry_any_case_and_trimmed() {
        let Explanation::Catalogued(found) = explain(" w018 ", &none()).unwrap() else {
            panic!("W018 is catalogued");
        };
        assert_eq!(found.entry.code, "W018");
        assert!(!found.retired);
        assert_eq!(
            found.entry.docs.as_deref(),
            Some("https://github.com/leaderiop/SpecForge/blob/main/docs/diagnostics.md#w018")
        );
    }

    #[test]
    fn a_retired_code_names_its_replacement_or_none() {
        let Explanation::Retired(renumbered) = explain("E047", &none()).unwrap() else {
            panic!("E047 is retired");
        };
        assert_eq!(renumbered.replaced_by.unwrap().code, "W139");

        let gone = serde_json::to_value(explain("W024", &none()).unwrap()).unwrap();
        assert_eq!(gone, serde_json::json!({"code": "W024", "retired": true}));
    }

    #[specforge_test(
        behavior = "explain_diagnostic_code",
        verify = "an uncatalogued code is refused with unknown_code on both surfaces"
    )]
    fn an_uncatalogued_code_is_unknown_code() {
        for code in ["Z123", "E901"] {
            let error = explain(code, &none()).unwrap_err();
            assert_eq!(error.error.kind, OpErrorKind::InvalidInput, "{code}");
            assert_eq!(error.error.code, UNKNOWN_CODE, "{code}");
            assert_eq!(
                error.error.message,
                format!("unknown diagnostic code: {code}"),
                "{code}"
            );
            assert!(
                error
                    .error
                    .suggestion
                    .as_deref()
                    .unwrap()
                    .starts_with("codes are E###"),
                "{code}"
            );
        }
    }

    #[specforge_test(
        behavior = "explain_diagnostic_code",
        verify = "explain answers a code an extension catalogs, whether a rule or a pass reports it"
    )]
    fn explain_answers_a_catalogued_code_no_rule_declares() {
        let answer = explain(" Acme.Audit/w950 ", &book(&[("@acme/audit", &["W950"])])).unwrap();
        let Explanation::Catalogued(found) = answer else {
            panic!("acme.audit/W950 is catalogued");
        };
        assert_eq!(found.entry.code, "acme.audit/W950");
        assert_eq!(found.entry.title, "Audit failed");
        assert_eq!(found.entry.owner, "@acme/audit");
        assert_eq!(found.entry.level, "warning");
        assert_eq!(found.entry.docs, None);
    }

    #[specforge_test(
        behavior = "explain_diagnostic_code",
        verify = "explain answers a code an extension catalogs under its namespace, the static catalog first, and a bare code only from the static catalog"
    )]
    fn the_static_catalog_answers_a_bare_code_whatever_an_extension_catalogs() {
        let squat = book(&[("@acme/x", &["W041", "E047", "W950"])]);
        let Explanation::Catalogued(found) = explain("W041", &squat).unwrap() else {
            panic!("W041 is catalogued");
        };
        assert_eq!(found.entry.owner, "@specforge/product");
        assert!(matches!(
            explain("E047", &squat).unwrap(),
            Explanation::Retired(_)
        ));
        assert!(explain("acme.x/W041", &squat).is_ok());
        let error = explain("W950", &squat).unwrap_err();
        assert_eq!(error.error.code, UNKNOWN_CODE);
    }

    #[test]
    fn a_bare_code_an_extension_catalogs_suggests_its_namespaced_forms() {
        let both = book(&[("@a/one", &["E950"]), ("@b/two", &["E950"])]);
        let error = explain("e950", &both).unwrap_err();
        let suggestion = error.error.suggestion.unwrap();
        assert!(
            suggestion.ends_with("did you mean a.one/E950 or b.two/E950?"),
            "{suggestion}"
        );
        // The qualified form of an uncatalogued code is unknown, with no guess.
        let error = explain("a.one/E951", &both).unwrap_err();
        assert!(!error.error.suggestion.unwrap().contains("did you mean"));
    }

    #[specforge_test(
        invariant = "mcp_structured_error_responses",
        verify = "an operation states the request field each refusal is about, and a refusal of the project or the environment is about none"
    )]
    fn explain_states_which_field_a_refusal_is_about() {
        assert_eq!(
            explain("Z123", &none()).unwrap_err().subject,
            Some(ExplainField::Code)
        );
    }
}
