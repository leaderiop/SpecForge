//! `specforge explain` and `specforge.explain`: what the diagnostic catalog says of one code, else
//! what the loaded extensions' rules declare of it (ADR 0278), one operation whose outcome is the
//! document both surfaces print (`McpExplainResult`, ADR 0052).

use serde::Serialize;
use specforge_common::shape::Shape;
use specforge_diagnostics::{CodeEntry, lookup, retired};
use specforge_protocol_types::{ExtensionDeclaration, ValidationRuleDescriptor};
use specforge_registry::declared_level;

use crate::{OpError, OpErrorKind, Refusal};

/// The refusal of a code the catalog neither has nor retired.
pub const UNKNOWN_CODE: &str = "unknown_code";

/// The forms a code takes: the suggestion of the refusal of an unknown one.
const CODE_FORMS: &str = "codes are E### (error), W### (warning), I### (info), A### (analyze finding), \
     R### and R-<AREA>-### (registry client); E900-E998, W900-W998 and I900-I998 are \
     third-party extensions' (explain them inside a project that loads the extension, or see its \
     documentation)";

/// What the catalog says of a code: its entry, or that it was retired and what replaced it; else
/// what the loaded extensions' rules declare of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
#[serde(untagged)]
pub enum Explanation {
    /// A code the catalog has.
    Catalogued(Catalogued),
    /// A code the catalog retired.
    Retired(Retired),
    /// A code the catalog does not have that the loaded extensions' rules declare.
    Declared(Declared),
}

/// A code the loaded extensions declare: upper-cased as asked, the level its rules report at,
/// each rule that reports it, `retired: false`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct Declared {
    pub code: String,
    /// `error`, `warning` or `info`.
    pub level: String,
    pub rules: Vec<DeclaredRule>,
    pub retired: bool,
}

/// One declared rule as both surfaces print it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct DeclaredRule {
    /// The extension that declares it (the origin of what it reports).
    pub extension: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_kind: Option<String>,
    /// Its check (`no_outgoing_edges`, `custom`, …).
    pub check: String,
    /// Its message template, as declared (`{id}` and the rest unexpanded).
    pub message: String,
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
    /// Its section of `docs/diagnostics.md`.
    pub docs: String,
}

impl Entry {
    fn of(entry: &CodeEntry) -> Self {
        Entry {
            code: entry.code.to_string(),
            title: entry.title.to_string(),
            owner: entry.owner.to_string(),
            level: entry.level.describe().to_string(),
            explanation: entry.explanation.to_string(),
            docs: entry.docs_href(),
        }
    }
}

/// The rules among `declarations` that declare `code` (any case, trimmed) and whose code the host
/// takes from their extension (no W150), in load order, with their extension's name.
pub fn declared<'d>(
    code: &str,
    declarations: &'d [ExtensionDeclaration],
) -> impl Iterator<Item = (&'d str, &'d ValidationRuleDescriptor)> {
    let code = code.trim().to_uppercase();
    declarations.iter().flat_map(move |declaration| {
        let extension = declaration.name();
        let code = code.clone();
        declaration
            .validation_rules
            .iter()
            .filter(move |rule| {
                rule.code.eq_ignore_ascii_case(&code) && declared_level(extension, rule).is_ok()
            })
            .map(move |rule| (extension, rule))
    })
}

/// The catalog's answer for `code`: any case, surrounding whitespace ignored; else the answer of
/// the rules `declarations` declare for it ([`declared`]). A code neither has is `unknown_code`
/// (invalid input), with the code forms as its suggestion.
pub fn explain(
    code: &str,
    declarations: &[ExtensionDeclaration],
) -> Result<Explanation, Refusal<ExplainField>> {
    let code = code.trim();
    if let Some(replacement) = retired(code) {
        return Ok(Explanation::Retired(Retired {
            code: code.to_uppercase(),
            retired: true,
            replaced_by: replacement.and_then(lookup).map(Entry::of),
        }));
    }
    match lookup(code) {
        Some(entry) => Ok(Explanation::Catalogued(Catalogued {
            entry: Entry::of(entry),
            retired: false,
        })),
        None => declared_answer(code, declarations).ok_or_else(|| {
            OpError::new(
                OpErrorKind::InvalidInput,
                UNKNOWN_CODE,
                format!("unknown diagnostic code: {code}"),
            )
            .with_suggestion(CODE_FORMS)
            .about(ExplainField::Code)
        }),
    }
}

/// The [`Declared`] answer of the rules that declare `code`, if any does.
fn declared_answer(code: &str, declarations: &[ExtensionDeclaration]) -> Option<Explanation> {
    // Accepted rules of one code agree on its level: the first one's.
    let (extension, first) = declared(code, declarations).next()?;
    let level = declared_level(extension, first).ok()?;
    let rules = declared(code, declarations)
        .map(|(extension, rule)| DeclaredRule {
            extension: extension.to_string(),
            target_kind: rule.target_kind.clone(),
            check: rule.check.clone(),
            message: rule.message_template.clone(),
        })
        .collect();
    Some(Explanation::Declared(Declared {
        code: code.to_uppercase(),
        level: level.describe().to_string(),
        rules,
        retired: false,
    }))
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
    use specforge_test_macros::test as specforge_test;

    #[specforge_test(
        behavior = "explain_diagnostic_code",
        verify = "specforge explain reads the code as specforge.explain does: any case, surrounding whitespace ignored"
    )]
    fn explain_answers_the_entry_any_case_and_trimmed() {
        let Explanation::Catalogued(found) = explain(" w018 ", &[]).unwrap() else {
            panic!("W018 is catalogued");
        };
        assert_eq!(found.entry.code, "W018");
        assert!(!found.retired);
        assert_eq!(
            found.entry.docs,
            "https://github.com/leaderiop/SpecForge/blob/main/docs/diagnostics.md#w018"
        );
    }

    #[test]
    fn a_retired_code_names_its_replacement_or_none() {
        let Explanation::Retired(renumbered) = explain("E047", &[]).unwrap() else {
            panic!("E047 is retired");
        };
        assert_eq!(renumbered.replaced_by.unwrap().code, "W139");

        let gone = serde_json::to_value(explain("W024", &[]).unwrap()).unwrap();
        assert_eq!(gone, serde_json::json!({"code": "W024", "retired": true}));
    }

    #[specforge_test(
        behavior = "explain_diagnostic_code",
        verify = "an uncatalogued code is refused with unknown_code on both surfaces"
    )]
    fn an_uncatalogued_code_is_unknown_code() {
        for code in ["Z123", "E901"] {
            let error = explain(code, &[]).unwrap_err();
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

    /// `extension` declaring `code` at `severity` on `campaign`.
    fn declaring(
        extension: &str,
        code: &str,
        severity: specforge_protocol_types::ValidationSeverity,
    ) -> ExtensionDeclaration {
        use specforge_extension_sdk::prelude::*;
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(extension, "0.1.0"));
        c.rule(code, |r| {
            r.check(CheckKind::MissingRequiredField)
                .target_kind("campaign")
                .field("ends")
                .severity(severity)
                .message_template("campaign '{id}' has no end");
        });
        c.declaration()
    }

    use specforge_protocol_types::ValidationSeverity::{Error, Warning};

    #[specforge_test(
        behavior = "explain_diagnostic_code",
        verify = "explain answers a code the loaded declarations declare from their rules, the catalog first, and only codes the host takes from their extension"
    )]
    #[ignore = "K10 T6"]
    fn a_declared_code_is_answered_by_its_rules() {
        let launch = [declaring("@acme/launch", "E903", Error)];
        let answer = serde_json::to_value(explain(" e903 ", &launch).unwrap()).unwrap();
        assert_eq!(
            answer,
            serde_json::json!({
                "code": "E903",
                "level": "error",
                "retired": false,
                "rules": [{
                    "extension": "@acme/launch",
                    "target_kind": "campaign",
                    "check": "missing_required_field",
                    "message": "campaign '{id}' has no end",
                    "description": "Ends before it starts.",
                }],
            })
        );
    }

    // Pins the pass-code gap of ADR 0278: a catalogued code no rule declares is unknown_code
    // today; T6 un-ignores it.
    #[specforge_test(
        behavior = "explain_diagnostic_code",
        verify = "explain answers a code an extension catalogs, whether a rule or a pass reports it"
    )]
    #[ignore = "K10 T6"]
    fn explain_answers_a_catalogued_code_no_rule_declares() {
        use specforge_extension_sdk::prelude::*;
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/audit", "0.1.0"));
        c.code("W950", |d| {
            d.title("Audit failed").explanation("Fix the audit.");
        });
        let declarations = [c.declaration()];
        assert!(explain("acme.audit/W950", &declarations).is_ok());
    }

    #[test]
    fn the_catalog_answers_a_code_whoever_declares_it() {
        let squat = [declaring("@acme/x", "W041", Warning)];
        let Explanation::Catalogued(found) = explain("W041", &squat).unwrap() else {
            panic!("W041 is catalogued");
        };
        assert_eq!(found.entry.owner, "@specforge/product");
        let retired = [declaring("@acme/x", "E047", Error)];
        assert!(matches!(
            explain("E047", &retired).unwrap(),
            Explanation::Retired(_)
        ));
    }

    #[test]
    fn a_declared_code_the_host_does_not_take_is_unknown() {
        for (extension, code, severity) in [
            ("@acme/x", "W500", Warning),
            ("@acme/x", "W950", Error),
            ("@specforge/rust", "W950", Warning),
        ] {
            let declarations = [declaring(extension, code, severity)];
            let error = explain(code, &declarations).unwrap_err();
            assert_eq!(error.error.code, UNKNOWN_CODE, "{extension} {code}");
        }
    }

    #[test]
    fn every_extension_declaring_a_code_is_listed() {
        let both = [
            declaring("@a/one", "E950", Error),
            declaring("@b/two", "E950", Error),
        ];
        let Explanation::Declared(answer) = explain("E950", &both).unwrap() else {
            panic!("E950 is declared");
        };
        assert_eq!(answer.level, "error");
        let extensions: Vec<&str> = answer.rules.iter().map(|r| r.extension.as_str()).collect();
        assert_eq!(extensions, ["@a/one", "@b/two"]);
    }

    #[specforge_test(
        invariant = "mcp_structured_error_responses",
        verify = "an operation states the request field each refusal is about, and a refusal of the project or the environment is about none"
    )]
    fn explain_states_which_field_a_refusal_is_about() {
        assert_eq!(
            explain("Z123", &[]).unwrap_err().subject,
            Some(ExplainField::Code)
        );
    }
}
