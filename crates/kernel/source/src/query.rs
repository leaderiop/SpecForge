//! A pack's query, compiled and held to the tags convention (ADR 0344 D1): one role capture per pattern,
//! `definition.<kind>` or `reference.<kind>`; every other capture not starting with `_` answered; only the text
//! predicates tree-sitter evaluates itself.

use sha2::Digest as _;
use specforge_protocol_types::SyntaxRole;
use tree_sitter::{CaptureQuantifier, Query};

use crate::Language;

/// Why a query was refused: tree-sitter's own error, or the convention's, at a 1-based row and column of the query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryFault {
    pub row: usize,
    pub column: usize,
    pub message: String,
}

impl std::fmt::Display for QueryFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.row, self.column, self.message)
    }
}

/// A pattern's role capture.
#[derive(Debug, Clone)]
pub(crate) struct Role {
    pub(crate) role: SyntaxRole,
    pub(crate) kind: String,
    pub(crate) capture: u32,
}

/// A compiled query that keeps the convention.
#[derive(Debug)]
pub struct SourceQuery {
    language: Language,
    pub(crate) query: Query,
    digest: String,
    /// Each pattern's role capture, by pattern index.
    pub(crate) roles: Vec<Role>,
    /// Whether a capture is answered, by capture index: not a role capture, not `_`-prefixed.
    pub(crate) answered: Vec<bool>,
}

/// A role capture's name read: `definition.function` is `(Definition, "function")`.
fn role_of(name: &str) -> Option<(SyntaxRole, &str)> {
    let (role, kind) = if let Some(kind) = name.strip_prefix("definition.") {
        (SyntaxRole::Definition, kind)
    } else {
        (SyntaxRole::Reference, name.strip_prefix("reference.")?)
    };
    Some((role, kind))
}

/// A role's kind: a word with `_` or `.` inside.
fn is_kind(kind: &str) -> bool {
    !kind.is_empty()
        && !kind.starts_with('.')
        && !kind.ends_with('.')
        && kind
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

/// `sha256:<hex>` of a query's text.
pub(crate) fn digest_of(text: &str) -> String {
    format!("sha256:{:x}", sha2::Sha256::digest(text.as_bytes()))
}

/// The 1-based row and column of byte `offset` of `text`, the column in characters.
fn place(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..text.floor_char_boundary(offset.min(text.len()))];
    let row = before.matches('\n').count() + 1;
    let line = before.rfind('\n').map_or(before, |at| &before[at + 1..]);
    (row, line.chars().count() + 1)
}

impl SourceQuery {
    /// `text` compiled for `language`, or why it cannot be: tree-sitter's error, a pattern without exactly one role
    /// capture (or with it optional or repeated), a role kind that is no word, or a predicate tree-sitter does not
    /// evaluate as text (`#set!`, `#is?`, `#strip!`, `#select-adjacent!`, any other).
    pub fn compile(language: Language, text: &str) -> Result<SourceQuery, QueryFault> {
        Self::compile_digested(language, text, digest_of(text))
    }

    /// [`Self::compile`], `digest` being [`digest_of`] `text`.
    pub(crate) fn compile_digested(
        language: Language,
        text: &str,
        digest: String,
    ) -> Result<SourceQuery, QueryFault> {
        let query = Query::new(&language.tree_sitter(), text).map_err(|error| QueryFault {
            row: error.row + 1,
            column: error.column + 1,
            message: error.message,
        })?;
        let names = query.capture_names();
        let mut roles = Vec::with_capacity(query.pattern_count());
        for pattern in 0..query.pattern_count() {
            let fault = |message: String| {
                let (row, column) = place(text, query.start_byte_for_pattern(pattern));
                QueryFault {
                    row,
                    column,
                    message,
                }
            };
            let mut found: Vec<(u32, SyntaxRole, &str, CaptureQuantifier)> = Vec::new();
            for (capture, quantifier) in query.capture_quantifiers(pattern).iter().enumerate() {
                if *quantifier == CaptureQuantifier::Zero {
                    continue;
                }
                let name = names[capture];
                if let Some((role, kind)) = role_of(name) {
                    if !is_kind(kind) {
                        return Err(fault(format!(
                            "the role capture `@{name}` names no kind: `<kind>` is a word with `_` or `.`"
                        )));
                    }
                    found.push((capture as u32, role, kind, *quantifier));
                }
            }
            let role = match found.as_slice() {
                [(capture, role, kind, CaptureQuantifier::One)] => Role {
                    role: *role,
                    kind: (*kind).to_string(),
                    capture: *capture,
                },
                [(_, _, kind, _)] => {
                    return Err(fault(format!(
                        "the role capture of a pattern is captured exactly once (`{kind}` is optional or repeated)"
                    )));
                }
                [] => {
                    return Err(fault(
                        "a pattern has no role capture: capture its node `@definition.<kind>` or `@reference.<kind>`"
                            .to_string(),
                    ));
                }
                _ => {
                    return Err(fault(format!(
                        "a pattern has {} role captures; it has exactly one",
                        found.len()
                    )));
                }
            };
            let general = query
                .general_predicates(pattern)
                .iter()
                .map(|predicate| predicate.operator.to_string())
                .chain(
                    query
                        .property_settings(pattern)
                        .iter()
                        .map(|_| "set!".to_string()),
                )
                .chain(
                    query
                        .property_predicates(pattern)
                        .iter()
                        .map(|(_, is)| if *is { "is?" } else { "is-not?" }.to_string()),
                )
                .next();
            if let Some(operator) = general {
                return Err(fault(format!(
                    "`#{operator}` is not a text predicate the reader evaluates: use #eq?, #not-eq?, #match?, \
                     #not-match?, #any-of?, #not-any-of? (or their any- forms), anchor with `.` and strip text in \
                     the pack"
                )));
            }
            roles.push(role);
        }
        let answered = names
            .iter()
            .map(|name| role_of(name).is_none() && !name.starts_with('_'))
            .collect();
        Ok(SourceQuery {
            language,
            query,
            digest,
            roles,
            answered,
        })
    }

    /// The language it runs on.
    pub fn language(&self) -> Language {
        self.language
    }

    /// `sha256:<hex>` of its text: part of every cache key.
    pub fn digest(&self) -> &str {
        &self.digest
    }
}
