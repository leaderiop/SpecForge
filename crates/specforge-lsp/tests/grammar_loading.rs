use specforge_test_macros::test as spec;

// -- load_extension_grammars_for_highlighting ---------------------------------

#[spec(behavior = "load_extension_grammars_for_highlighting", verify = "grammar loaded at LSP startup for registered contributions")]
#[test]
fn grammar_loaded_for_contributions() {
    let mut cache = specforge_lsp::GrammarCache::new();
    cache.register("behavior", "behavior_grammar.wasm");
    assert!(cache.has_grammar("behavior"));
}

#[spec(behavior = "load_extension_grammars_for_highlighting", verify = "grammar reloaded on extension configuration change")]
#[test]
fn grammar_reloaded_on_change() {
    let mut cache = specforge_lsp::GrammarCache::new();
    cache.register("behavior", "old.wasm");
    cache.register("behavior", "new.wasm");
    assert_eq!(cache.grammar_path("behavior"), Some("new.wasm"));
}

#[spec(behavior = "load_extension_grammars_for_highlighting", verify = "grammar conflict resolved per grammar_policy")]
#[test]
fn grammar_conflict_last_wins() {
    let mut cache = specforge_lsp::GrammarCache::new();
    cache.register("behavior", "ext_a.wasm");
    cache.register("behavior", "ext_b.wasm");
    // Default policy: last registration wins
    assert_eq!(cache.grammar_path("behavior"), Some("ext_b.wasm"));
}

#[spec(behavior = "load_extension_grammars_for_highlighting", verify = "grammar loading failure does not affect other kinds")]
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
