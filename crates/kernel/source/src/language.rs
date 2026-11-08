//! The languages the reader has a grammar for.

/// A grammar the reader parses with. JavaScript is read with [`Language::TypeScript`], JSX with [`Language::Tsx`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Language {
    Rust,
    TypeScript,
    Tsx,
}

impl Language {
    pub const ALL: [Language; 3] = [Language::Rust, Language::TypeScript, Language::Tsx];

    /// The language a request names: `rust`, `typescript` or `tsx`.
    pub fn named(name: &str) -> Option<Language> {
        Language::ALL
            .into_iter()
            .find(|language| language.name() == name)
    }

    /// The name a request uses.
    pub const fn name(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::TypeScript => "typescript",
            Language::Tsx => "tsx",
        }
    }

    /// The grammar crate and its version (`tree-sitter-rust@0.24.2`): part of every cache key, so a grammar upgrade
    /// parses again. A test holds it to `Cargo.lock`.
    pub const fn grammar(self) -> &'static str {
        match self {
            Language::Rust => "tree-sitter-rust@0.24.2",
            Language::TypeScript => "tree-sitter-typescript@0.23.2/typescript",
            Language::Tsx => "tree-sitter-typescript@0.23.2/tsx",
        }
    }

    pub(crate) fn tree_sitter(self) -> tree_sitter::Language {
        match self {
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        }
    }
}
