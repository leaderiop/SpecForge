// Parsing features

use behaviors/parsing

feature spec_file_parsing "Spec File Parsing" {
  behaviors [parse_spec_file_to_ast, parse_use_imports, parse_all_block_types, parse_triple_quoted_strings]

  problem """
    .spec files need to be parsed into structured ASTs that preserve
    source locations for all tokens, supporting all 16 block types
    with their specific field syntax including triple-quoted strings.
  """

  solution """
    Tree-sitter grammar that recognizes all block types, use imports,
    reference lists, and triple-quoted strings. Produces per-file ASTs
    with full source span information for every token.
  """
}

feature error_recovery_during_parsing "Error Recovery During Parsing" {
  behaviors [recover_from_syntax_errors]

  problem """
    A single syntax error in one block should not prevent the compiler
    from reporting errors in subsequent blocks. Users need to see all
    errors at once, not fix them one by one.
  """

  solution """
    Tree-sitter's error recovery produces partial ASTs even when syntax
    errors exist. The compiler collects all errors and continues parsing
    subsequent blocks, reporting all issues in a single pass.
  """
}

feature scenario_declaration "Scenario Declaration" {
  behaviors [parse_scenario_blocks]

  problem """
    Behavior and capability blocks need a way to declare structured
    acceptance criteria using given/when/then steps. This requires
    the parser to recognize scenario blocks as a new syntax construct
    within entity blocks.
  """

  solution """
    Tree-sitter grammar recognizes scenario, given, when, and then
    keywords within behavior and capability blocks. Scenario blocks
    are parsed into ScenarioList field values in the AST.
  """
}

feature editor_query_files "Editor Query Files" {
  behaviors [provide_syntax_highlighting_queries, provide_code_folding_queries, provide_indentation_queries, parse_generic_entity_blocks]

  problem """
    Without query files, .spec files appear as plain text in
    Tree-sitter-aware editors like Neovim, Helix, Zed, and Emacs.
    Users get no syntax highlighting, code folding, or automatic
    indentation — even though the grammar is fully functional.
    Plugin and custom entities produce ERROR nodes, breaking all
    editor features for non-built-in entity types.
  """

  solution """
    Ship highlights.scm, folds.scm, and indents.scm alongside the
    grammar. A generic_entity_block grammar rule catches plugin and
    custom entity blocks that don't match built-in keywords, producing
    clean AST nodes instead of ERROR nodes. These standard query files
    enable any Tree-sitter-aware editor to provide rich editing support
    for both built-in and plugin entities without requiring an LSP
    server.
  """
}
