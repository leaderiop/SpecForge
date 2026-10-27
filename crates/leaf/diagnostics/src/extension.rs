//! Why an extension may not report a code ([`CodeMisuse`]). The host checks the
//! codes an extension states as text (a rule descriptor's `code`, a pass
//! diagnostic's `code`) where they enter ([`crate::Codebook::check`]); a
//! misuse is W150, and the rule or finding is kept, under the extension's own
//! namespace.

use crate::Level;

/// Why an extension may not report a code. `Display` is the reason W150
/// gives, a noun phrase: `a code core owns ("Parse error")`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeMisuse {
    /// The catalog gives the code to another owner (`"core"` or an
    /// extension).
    NotItsCode {
        owner: &'static str,
        title: &'static str,
    },
    /// The extension's own code, reported at another level than the
    /// catalog's.
    WrongLevel { catalogued: Level },
    /// A retired code (never reused for another meaning).
    Retired { replaced_by: Option<&'static str> },
    /// A code its extension's catalog does not list: the host's catalog for
    /// a first-party extension, its `codes` for any other.
    Uncatalogued,
    /// A third-party code whose `E`/`W`/`I` letter contradicts the level
    /// reported.
    PrefixContradictsLevel,
    /// A third-party code that is not `[EWI]` and three digits (a `/`, an
    /// `A` code): an extension reports only its local codes.
    NotALocalCode,
}

impl std::fmt::Display for CodeMisuse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodeMisuse::NotItsCode { owner, title } => {
                write!(f, "a code {owner} owns (\"{title}\")")
            }
            CodeMisuse::WrongLevel { catalogued } => write!(
                f,
                "its own code, reported at another level than the catalog's ({})",
                catalogued.describe()
            ),
            CodeMisuse::Retired { replaced_by: None } => {
                write!(f, "a retired code, which is never reused")
            }
            CodeMisuse::Retired {
                replaced_by: Some(new),
            } => write!(f, "a retired code (replaced by {new})"),
            CodeMisuse::Uncatalogued => write!(f, "a code its extension's catalog does not list"),
            CodeMisuse::NotALocalCode => write!(
                f,
                "not a local code; an extension reports E, W or I and three digits, and the host adds its namespace"
            ),
            CodeMisuse::PrefixContradictsLevel => write!(
                f,
                "a code whose letter contradicts the level reported; E states an error, W a warning and I an info"
            ),
        }
    }
}
