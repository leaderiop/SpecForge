// Parsing behaviors — Tree-sitter grammar and AST construction

use invariants/core
use invariants/validation
use types/core
use types/errors
use ports/outbound

behavior parse_spec_file_to_ast "Parse Spec File to AST" {
  invariants [multi_error_collection]
  types      [SpecFile, ParseError, SourceSpan]
  ports      [SourceParser]

  contract """
    As the first stage of the compiler pipeline, given a syntactically
    valid .spec file, the parser MUST produce an AST containing all
    declared entities with their fields, source spans, and use imports.
    The AST MUST preserve source locations for every token.
  """

  verify unit "parse valid file produces complete AST"
  verify unit "AST source spans match original token positions"

  tests ["../crates/specforge-parser/tests/snapshot_tests.rs"]
}

behavior recover_from_syntax_errors "Recover From Syntax Errors" {
  invariants [multi_error_collection]
  types      [SpecFile, ParseError]
  ports      [SourceParser]

  contract """
    When a .spec file contains syntax errors, the parser MUST recover
    and continue parsing subsequent blocks. The parser MUST collect all
    parse errors with source locations. Syntactically valid blocks
    after an error MUST still appear in the AST.
  """

  verify unit "parser collects multiple errors from one file"
  verify unit "valid blocks after syntax error are still parsed"

  tests ["../crates/specforge-parser/tests/snapshot_tests.rs"]
}

behavior parse_use_imports "Parse Use Imports" {
  invariants [import_dag]
  types      [SpecFile]
  ports      [SourceParser]

  contract """
    The parser MUST recognize use directives at the top of .spec files.
    Both full imports (use path/to/file) and selective imports
    (use path/to/file { ID-1 }) MUST be parsed. Import paths MUST NOT
    include the .spec extension.
  """

  verify unit "parse full use import"
  verify unit "parse selective use import with braces"
  verify unit "reject use import with .spec extension"

  tests ["../crates/specforge-parser/tests/snapshot_tests.rs"]
}

behavior parse_all_block_types "Parse All Block Types" {
  invariants [multi_error_collection]
  types      [Entity, EntityKind, FieldMap, FieldEntry, FieldValue, StringValue, ReferenceList, StringList, Block, VerifyList, VerifyStatement, VerifyKind, ScenarioList, TestFileList]
  ports      [SourceParser]

  contract """
    The parser MUST recognize all 16 block types: spec, invariant,
    behavior, feature, event, type, port, ref, capability, deliverable,
    roadmap, library, glossary, decision, constraint, and failure_mode.
    Unknown block types MUST produce a parse error.
  """

  verify unit "parse each of the 16 block types"
  verify unit "unknown block type produces parse error"

  tests ["../crates/specforge-parser/tests/snapshot_tests.rs"]
}

behavior parse_triple_quoted_strings "Parse Triple-Quoted Strings" {
  types      [SpecFile]
  ports      [SourceParser]

  contract """
    The parser MUST handle triple-quoted strings (triple double-quotes).
    Leading whitespace common to all lines MUST be stripped (dedent).
    The content between delimiters MUST preserve internal newlines and
    relative indentation.
  """

  verify unit "triple-quoted string preserves newlines"
  verify unit "common leading whitespace is stripped"
  verify unit "relative indentation is preserved"

  tests ["../crates/specforge-parser/tests/snapshot_tests.rs"]
}

behavior parse_generic_entity_blocks "Parse Generic Entity Blocks" {
  invariants [multi_error_collection]
  types      [SpecFile, ParseError, SourceSpan]
  ports      [SourceParser]

  contract """
    The grammar MUST recognize identifier-name-body patterns that do
    not match any built-in entity keyword as generic_entity_block
    nodes. These blocks MUST produce clean AST nodes with a kind
    field containing the unknown keyword, a name field, an optional
    title, and standard block body items (fields, verify, scenario).
    The generic rule MUST have lower priority than all built-in entity
    rules. The resolver MUST validate whether the kind is registered
    by an installed plugin or define block — unregistered kinds MUST
    produce a diagnostic.
  """

  verify unit "unknown keyword produces generic_entity_block AST node"
  verify unit "built-in keywords still match their specific rules"
  verify unit "generic block preserves kind, name, title, and fields"
  verify unit "unregistered kind produces diagnostic in resolver"

  tests ["../crates/specforge-parser/tests/snapshot_tests.rs"]
}

behavior provide_syntax_highlighting_queries "Provide Syntax Highlighting Queries" {
  invariants [multi_error_collection]
  ports      [SourceParser]

  contract """
    The grammar MUST ship a highlights.scm query file that maps all
    node types to standard Tree-sitter capture names. Keywords MUST
    map to @keyword, strings to @string, entity IDs to @constant,
    types to @type, and comments to @comment. Generic entity blocks
    MUST be captured: the kind field as @keyword and the name field
    as @constant. The file MUST be loadable by any Tree-sitter-aware
    editor without an LSP server.
  """

  verify unit "highlights.scm captures all block keywords as @keyword"
  verify unit "highlights.scm captures strings and triple-quoted strings as @string"
  verify unit "highlights.scm captures entity IDs as @constant"
  verify unit "highlights.scm captures generic_entity_block kind as @keyword"

  tests ["../crates/specforge-parser/tests/snapshot_tests.rs"]
}

behavior provide_code_folding_queries "Provide Code Folding Queries" {
  invariants [multi_error_collection]
  ports      [SourceParser]

  contract """
    The grammar MUST ship a folds.scm query file that marks all
    brace-delimited blocks as foldable regions. Every block type,
    sub-block, nested block, and generic_entity_block MUST be
    foldable. The file MUST be loadable by any Tree-sitter-aware
    editor without an LSP server.
  """

  verify unit "folds.scm marks all 16 block types as @fold"
  verify unit "folds.scm marks sub-blocks (persona, surface, term) as @fold"
  verify unit "folds.scm marks generic_entity_block as @fold"

  tests ["../crates/specforge-parser/tests/snapshot_tests.rs"]
}

behavior provide_indentation_queries "Provide Indentation Queries" {
  invariants [multi_error_collection]
  ports      [SourceParser]

  contract """
    The grammar MUST ship an indents.scm query file that provides
    automatic indentation for brace-delimited and bracket-delimited
    blocks. Opening braces and brackets MUST trigger @indent,
    closing braces and brackets MUST trigger @dedent.
  """

  verify unit "indents.scm indents after opening brace"
  verify unit "indents.scm dedents on closing brace"

  tests ["../crates/specforge-parser/tests/snapshot_tests.rs"]
}

behavior parse_scenario_blocks "Parse Scenario Blocks" {
  invariants [multi_error_collection]
  types      [Scenario, ScenarioStep, ScenarioStepKind]
  ports      [SourceParser]

  contract """
    The parser MUST recognize scenario blocks with the syntax
    scenario "title" { given/when/then } within behavior and
    capability blocks. Each step MUST be parsed into a ScenarioStep
    with its kind and description. The parser MUST produce a
    ScenarioList in the AST for the enclosing entity.
  """

  verify unit "parse scenario block with all three step kinds"
  verify unit "parse scenario with multiple steps of same kind"
  verify unit "scenario outside behavior or capability produces parse error"

  tests ["../crates/specforge-parser/tests/snapshot_tests.rs"]
}
