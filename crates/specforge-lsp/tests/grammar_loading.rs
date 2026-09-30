// -- load_extension_grammars_for_highlighting ---------------------------------

#[test]
fn grammar_loaded_for_contributions() {
    let mut cache = specforge_lsp::GrammarCache::new();
    cache.register("behavior", "behavior_grammar.wasm");
    assert!(cache.has_grammar("behavior"));
}

#[test]
fn grammar_reloaded_on_change() {
    let mut cache = specforge_lsp::GrammarCache::new();
    cache.register("behavior", "old.wasm");
    cache.register("behavior", "new.wasm");
    assert_eq!(cache.grammar_path("behavior"), Some("new.wasm"));
}

#[test]
fn grammar_conflict_last_wins() {
    let mut cache = specforge_lsp::GrammarCache::new();
    cache.register("behavior", "ext_a.wasm");
    cache.register("behavior", "ext_b.wasm");
    // Default policy: last registration wins
    assert_eq!(cache.grammar_path("behavior"), Some("ext_b.wasm"));
}

#[test]
fn grammar_failure_isolated() {
    let mut cache = specforge_lsp::GrammarCache::new();
    cache.register("behavior", "valid.wasm");
    cache.mark_failed("type", "Failed to load type grammar");

    // "behavior" grammar still available
    assert!(cache.has_grammar("behavior"));
    // "type" grammar marked as failed
    assert!(!cache.has_grammar("type"));
    assert!(cache.failure("type").is_some());
}
