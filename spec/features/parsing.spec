// Parsing features

use behaviors/parsing
use behaviors/error-reporting

// See also: zero_entity_bootstrap (features/zero-entity-core.spec) for
// collapse_grammar_to_generic_entity_block and two_phase_parse_structural

feature spec_file_parsing "Spec File Parsing" {
  behaviors [parse_spec_file_to_ast, parse_use_imports, parse_all_block_types, parse_triple_quoted_strings, parse_gherkin_statements, parse_verify_statements, parse_ref_blocks, parse_define_blocks]

  problem """
    .spec files need to be parsed into structured ASTs that preserve
    source locations for all tokens, supporting any keyword via a
    generic entity_block rule, with field syntax including triple-quoted
    strings and gherkin file references.
  """

  solution """
    A parser grammar with a single generic entity_block rule that
    parses any keyword name { fields } structure into an AST node
    carrying kind, name, and optional title. Only spec, ref, use,
    and define have dedicated grammar rules (ref uses scheme:identifier
    format). Use imports, reference lists, gherkin statements,
    and triple-quoted strings are parsed structurally. Keyword validation
    is deferred to the semantic phase after extensions populate the
    KindRegistry. Produces per-file ASTs with full source span
    information for every token.
  """
}

feature error_recovery_during_parsing "Error Recovery During Parsing" {
  behaviors [recover_from_syntax_errors, format_diagnostics_with_source_context]

  problem """
    A single syntax error in one block should not prevent the compiler
    from reporting errors in subsequent blocks. Users need to see all
    errors at once, not fix them one by one.
  """

  solution """
    The parser's error recovery produces partial ASTs even when syntax
    errors exist. The compiler collects all errors and continues parsing
    subsequent blocks, reporting all issues in a single pass.
  """
}

feature editor_query_files "Editor Query Files" {
  behaviors [provide_syntax_highlighting_queries, provide_code_folding_queries, provide_indentation_queries]

  problem """
    Without query files, .spec files appear as plain text in
    editors with structural query support like Neovim, Helix, Zed, and Emacs.
    Users get no syntax highlighting, code folding, or automatic
    indentation — even though the grammar is fully functional.
  """

  solution """
    Ship highlights.scm, folds.scm, and indents.scm alongside the
    grammar. The generic entity_block grammar rule parses all entity
    blocks uniformly — there are no "built-in" keywords at the grammar
    level. These standard query files enable any compatible
    editor to provide rich editing support for all entity types
    without requiring an LSP server.
  """
}
