use std::collections::HashMap;

/// Cache for extension-contributed grammars, keyed by entity kind.
pub struct GrammarCache {
    grammars: HashMap<String, String>,
    failures: HashMap<String, String>,
}

impl GrammarCache {
    pub fn new() -> Self {
        Self {
            grammars: HashMap::new(),
            failures: HashMap::new(),
        }
    }

    pub fn register(&mut self, kind: &str, wasm_path: &str) {
        self.grammars
            .insert(kind.to_string(), wasm_path.to_string());
        self.failures.remove(kind);
    }

    pub fn has_grammar(&self, kind: &str) -> bool {
        self.grammars.contains_key(kind)
    }

    pub fn grammar_path(&self, kind: &str) -> Option<&str> {
        self.grammars.get(kind).map(|s| s.as_str())
    }

    pub fn mark_failed(&mut self, kind: &str, error: &str) {
        self.failures
            .insert(kind.to_string(), error.to_string());
    }

    pub fn failure(&self, kind: &str) -> Option<&str> {
        self.failures.get(kind).map(|s| s.as_str())
    }
}

impl Default for GrammarCache {
    fn default() -> Self {
        Self::new()
    }
}
