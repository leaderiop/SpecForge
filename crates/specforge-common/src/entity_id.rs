use serde::{Deserialize, Serialize};
use std::fmt;

/// Reserved keywords that cannot be used as entity names (the 16 entity keywords).
const RESERVED_WORDS: &[&str] = &[
    "spec",
    "invariant",
    "behavior",
    "feature",
    "event",
    "type",
    "port",
    "ref",
    "capability",
    "deliverable",
    "roadmap",
    "library",
    "glossary",
    "decision",
    "constraint",
    "failure_mode",
];

/// DSL syntax keywords that cannot be used as entity kind names.
const DSL_SYNTAX_KEYWORDS: &[&str] = &[
    "define", "use", "verify", "scenario", "given", "when", "then", "true", "false",
];

/// Entity identifier with two variants matching the SpecForge ID patterns.
///
/// - `Named`: variable-name identifiers for all named entities
///   (e.g., `data_persistence`, `auth_login`, `User`, `UserRepository`)
/// - `SchemeRef`: `scheme.kind:identifier` for `ref` entities
///   (e.g., `gh.issue:42`, `jira.epic:PROJ-123`)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityId {
    Named {
        name: String,
    },
    SchemeRef {
        scheme: String,
        kind: String,
        identifier: String,
        raw: String,
    },
}

impl EntityId {
    /// Parse a raw ID string into an `EntityId`.
    ///
    /// Tries scheme ref first (`scheme.kind:id`),
    /// then falls back to named identifier.
    pub fn parse(raw: &str) -> Self {
        // Try scheme ref: contains both '.' and ':'
        if let Some(dot_pos) = raw.find('.') {
            if let Some(colon_pos) = raw[dot_pos..].find(':') {
                let colon_pos = dot_pos + colon_pos;
                let scheme = &raw[..dot_pos];
                let kind = &raw[dot_pos + 1..colon_pos];
                let identifier = &raw[colon_pos + 1..];
                if !scheme.is_empty() && !kind.is_empty() && !identifier.is_empty() {
                    return EntityId::SchemeRef {
                        scheme: scheme.to_string(),
                        kind: kind.to_string(),
                        identifier: identifier.to_string(),
                        raw: raw.to_string(),
                    };
                }
            }
        }

        // Named identifier
        EntityId::Named {
            name: raw.to_string(),
        }
    }

    /// Return the raw string representation.
    pub fn raw(&self) -> &str {
        match self {
            EntityId::Named { name } => name,
            EntityId::SchemeRef { raw, .. } => raw,
        }
    }

    /// Return the name if this is a named identifier.
    pub fn name(&self) -> Option<&str> {
        match self {
            EntityId::Named { name } => Some(name),
            _ => None,
        }
    }

    /// Return the scheme if this is a scheme ref.
    pub fn scheme(&self) -> Option<&str> {
        match self {
            EntityId::SchemeRef { scheme, .. } => Some(scheme),
            _ => None,
        }
    }

    /// Returns true if this is a named identifier.
    pub fn is_named(&self) -> bool {
        matches!(self, EntityId::Named { .. })
    }

    /// Returns true if this is a scheme reference.
    pub fn is_scheme_ref(&self) -> bool {
        matches!(self, EntityId::SchemeRef { .. })
    }

    /// Check if a string is a valid identifier in any case style (2-60 chars).
    ///
    /// Accepts snake_case, PascalCase, camelCase, SCREAMING_SNAKE, etc.
    /// Rules: starts with a letter, then letters/digits/underscores, no double/leading/trailing
    /// underscores, 2-60 chars.
    pub fn is_valid_identifier(s: &str) -> bool {
        let len = s.len();
        if len < 2 || len > 60 {
            return false;
        }
        let mut chars = s.chars();
        // First char must be a letter
        match chars.next() {
            Some(c) if c.is_ascii_alphabetic() => {}
            _ => return false,
        }
        // Remaining chars: letters, digits, or underscore (no double/leading/trailing underscores)
        let mut prev_underscore = false;
        for c in chars {
            if c == '_' {
                if prev_underscore {
                    return false; // no double underscores
                }
                prev_underscore = true;
            } else if c.is_ascii_alphanumeric() {
                prev_underscore = false;
            } else {
                return false;
            }
        }
        // Must not end with underscore
        !prev_underscore
    }

    /// Check if a string is a reserved keyword.
    pub fn is_reserved_word(s: &str) -> bool {
        RESERVED_WORDS.contains(&s)
    }

    /// Check if a string is a reserved keyword OR a DSL syntax keyword.
    ///
    /// Used by the Wasm host function guard to reject entity kind names
    /// that would conflict with built-in keywords or DSL syntax.
    pub fn is_reserved_or_syntax_keyword(s: &str) -> bool {
        RESERVED_WORDS.contains(&s) || DSL_SYNTAX_KEYWORDS.contains(&s)
    }

    /// Auto-derive a human-readable title from an identifier.
    ///
    /// - `auth_login` → "Auth Login"
    /// - `data_persistence` → "Data Persistence"
    /// - `UserRepository` → "User Repository"
    pub fn auto_title(name: &str) -> String {
        if name.contains('_') {
            // snake_case: split on underscores, capitalize each word
            name.split('_')
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(c) => {
                            let upper: String = c.to_uppercase().collect();
                            upper + &chars.collect::<String>()
                        }
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            // PascalCase: insert spaces before uppercase letters
            let mut result = String::with_capacity(name.len() + 4);
            for (i, c) in name.chars().enumerate() {
                if i > 0 && c.is_uppercase() {
                    result.push(' ');
                }
                result.push(c);
            }
            result
        }
    }

}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.raw())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_named_snake_case() {
        let id = EntityId::parse("data_persistence");
        assert_eq!(
            id,
            EntityId::Named {
                name: "data_persistence".to_string(),
            }
        );
        assert_eq!(id.raw(), "data_persistence");
        assert_eq!(id.name(), Some("data_persistence"));
        assert!(id.is_named());
    }

    #[test]
    fn parse_named_pascal_case() {
        let id = EntityId::parse("UserRepository");
        assert_eq!(
            id,
            EntityId::Named {
                name: "UserRepository".to_string(),
            }
        );
        assert!(id.is_named());
        assert_eq!(id.raw(), "UserRepository");
    }

    #[test]
    fn parse_scheme_ref() {
        let id = EntityId::parse("gh.issue:42");
        assert_eq!(
            id,
            EntityId::SchemeRef {
                scheme: "gh".to_string(),
                kind: "issue".to_string(),
                identifier: "42".to_string(),
                raw: "gh.issue:42".to_string(),
            }
        );
        assert!(id.is_scheme_ref());
        assert_eq!(id.scheme(), Some("gh"));
    }

    #[test]
    fn parse_scheme_ref_complex_identifier() {
        let id = EntityId::parse("jira.epic:PROJ-123");
        assert_eq!(
            id,
            EntityId::SchemeRef {
                scheme: "jira".to_string(),
                kind: "epic".to_string(),
                identifier: "PROJ-123".to_string(),
                raw: "jira.epic:PROJ-123".to_string(),
            }
        );
    }

    #[test]
    fn display_roundtrip() {
        let cases = ["data_persistence", "auth_login", "User", "gh.issue:42"];
        for raw in cases {
            let id = EntityId::parse(raw);
            assert_eq!(id.to_string(), raw);
        }
    }

    #[test]
    fn valid_identifiers() {
        assert!(EntityId::is_valid_identifier("data_persistence"));
        assert!(EntityId::is_valid_identifier("UserRepository"));
        assert!(EntityId::is_valid_identifier("camelCase"));
        assert!(EntityId::is_valid_identifier("SCREAMING_SNAKE"));
        assert!(EntityId::is_valid_identifier("myThing_v2"));
        assert!(EntityId::is_valid_identifier("ab"));
        assert!(EntityId::is_valid_identifier("a1"));
        assert!(EntityId::is_valid_identifier("create_user_v2"));
        assert!(EntityId::is_valid_identifier("CreateUser"));
        assert!(EntityId::is_valid_identifier("User_Repo"));
    }

    #[test]
    fn invalid_identifiers() {
        assert!(!EntityId::is_valid_identifier("a")); // too short
        assert!(!EntityId::is_valid_identifier("_data")); // leading underscore
        assert!(!EntityId::is_valid_identifier("data_")); // trailing underscore
        assert!(!EntityId::is_valid_identifier("data__x")); // double underscore
        assert!(!EntityId::is_valid_identifier("1abc")); // leading digit
        assert!(!EntityId::is_valid_identifier("data-thing")); // hyphen
    }

    #[test]
    fn reserved_words() {
        assert!(EntityId::is_reserved_word("spec"));
        assert!(EntityId::is_reserved_word("behavior"));
        assert!(EntityId::is_reserved_word("failure_mode"));
        assert!(!EntityId::is_reserved_word("auth_login"));
        assert!(!EntityId::is_reserved_word("User"));
    }

    #[test]
    fn reserved_or_syntax_keywords() {
        // Built-in keywords
        assert!(EntityId::is_reserved_or_syntax_keyword("spec"));
        assert!(EntityId::is_reserved_or_syntax_keyword("behavior"));
        assert!(EntityId::is_reserved_or_syntax_keyword("failure_mode"));
        // DSL syntax keywords
        assert!(EntityId::is_reserved_or_syntax_keyword("define"));
        assert!(EntityId::is_reserved_or_syntax_keyword("verify"));
        assert!(EntityId::is_reserved_or_syntax_keyword("scenario"));
        assert!(EntityId::is_reserved_or_syntax_keyword("given"));
        assert!(EntityId::is_reserved_or_syntax_keyword("when"));
        assert!(EntityId::is_reserved_or_syntax_keyword("then"));
        assert!(EntityId::is_reserved_or_syntax_keyword("true"));
        assert!(EntityId::is_reserved_or_syntax_keyword("false"));
        assert!(EntityId::is_reserved_or_syntax_keyword("use"));
        // Valid names
        assert!(!EntityId::is_reserved_or_syntax_keyword("microservice"));
        assert!(!EntityId::is_reserved_or_syntax_keyword("epic"));
    }

    #[test]
    fn auto_title_snake_case() {
        assert_eq!(EntityId::auto_title("auth_login"), "Auth Login");
        assert_eq!(EntityId::auto_title("data_persistence"), "Data Persistence");
        assert_eq!(
            EntityId::auto_title("user_management"),
            "User Management"
        );
    }

    #[test]
    fn auto_title_pascal_case() {
        assert_eq!(EntityId::auto_title("UserRepository"), "User Repository");
        assert_eq!(EntityId::auto_title("EmailService"), "Email Service");
        assert_eq!(EntityId::auto_title("User"), "User");
    }
}
