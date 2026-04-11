use crate::ManifestV2;
use std::collections::BTreeMap;

/// Maps each DSL keyword to the list of extensions that declare it.
#[derive(Debug, Clone)]
pub struct KeywordExtensionIndex {
    pub entries: BTreeMap<String, Vec<String>>,
}

impl KeywordExtensionIndex {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn extensions_for_keyword(&self, keyword: &str) -> Option<&Vec<String>> {
        self.entries.get(keyword)
    }
}

/// Generate a keyword-to-extension index from a list of manifests.
/// Each `entity_kinds[].keyword` maps to the extensions that declare it.
/// Uses BTreeMap for deterministic ordering.
pub fn generate_keyword_extension_index(manifests: &[ManifestV2]) -> KeywordExtensionIndex {
    let mut entries: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for manifest in manifests {
        for kind in &manifest.entity_kinds {
            entries
                .entry(kind.keyword.clone())
                .or_default()
                .push(manifest.name.clone());
        }
    }

    KeywordExtensionIndex { entries }
}
