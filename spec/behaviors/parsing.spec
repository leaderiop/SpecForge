// Parsing behaviors — Tree-sitter grammar and AST construction

use invariants/core
use invariants/zero-entity-core
use invariants/wasm
use types/core
use types/errors
use types/wasm
use ports/outbound
use events/compilation

behavior parse_spec_file_to_ast "Parse Spec File to AST" {
  invariants [multi_error_collection, string_interning_consistency, zero_domain_knowledge_core, source_span_completeness]
  types      [SpecFile, ParseError, SourceSpan]
  ports      [SourceParser]
  produces   [file_parsed, all_files_parsed]

  requires {
    source_parser_available "SourceParser port is initialized and ready to accept input"
    valid_utf8_input "Input buffer is valid UTF-8"
  }

  ensures {
    ast_produced "A complete AST is produced containing all declared entities with fields, source spans, and use imports"
    source_spans_complete "Every token in the AST has an accurate source location"
    file_parsed_emitted "file_parsed event is emitted for each successfully parsed file"
  }

  contract """
    As the first stage of the compiler pipeline, given a syntactically
    valid .spec file, the parser MUST produce an AST containing all
    declared entities with their fields, source spans, and use imports.
    The AST MUST preserve source locations for every token.
  """

  verify unit "parse valid file produces complete AST"
  verify unit "AST source spans match original token positions"
  verify contract "requires/ensures consistency for spec file parsing"

}

// The following behaviors execute as part of parse_spec_file_to_ast
// and contribute to the file_parsed and all_files_parsed events.
// They do not produce events independently.

behavior recover_from_syntax_errors "Recover From Syntax Errors" {
  invariants [multi_error_collection, zero_domain_knowledge_core, source_span_completeness]
  types      [SpecFile, ParseError]
  ports      [SourceParser]

  contract """
    When a .spec file contains syntax errors, the parser MUST recover
    and continue parsing subsequent blocks. The parser MUST collect all
    parse errors with source locations. Syntactically valid blocks
    after an error MUST still appear in the AST.
  """

  requires {
    error_recovery_enabled "SourceParser is initialized with error-recovery mode enabled"
    valid_utf8_input "Input buffer is valid UTF-8"
  }

  ensures {
    valid_blocks_preserved "All syntactically valid blocks following an error site appear in the AST"
    errors_collected "ParseError list is populated with one entry per recovery point"
  }

  verify unit "parser collects multiple errors from one file"
  verify unit "valid blocks after syntax error are still parsed"
  verify contract "requires/ensures consistency for syntax error recovery"

}

behavior parse_use_imports "Parse Use Imports" {
  invariants [import_dag, zero_domain_knowledge_core, string_interning_consistency]
  types      [SpecFile, ImportDeclaration, SourceSpan]
  ports      [SourceParser]

  requires {
    source_parser_available "SourceParser port is initialized and ready to accept input"
  }

  ensures {
    imports_extracted "All use directives are parsed into ImportDeclaration entries with paths and optional selective IDs"
    extension_rejected "Import paths containing .spec extension are rejected with a diagnostic"
  }

  contract """
    The parser MUST recognize use directives at the top of .spec files.
    Both full imports (use path/to/file) and selective imports
    (use path/to/file { ID-1 }) MUST be parsed. Import paths MUST NOT
    include the .spec extension.
  """

  verify unit "parse full use import"
  verify unit "parse selective use import with braces"
  verify unit "reject use import with .spec extension"
  verify contract "requires/ensures consistency for use import parsing"

}

behavior parse_all_block_types "Parse All Block Types" {
  invariants [multi_error_collection, zero_domain_knowledge_core, source_span_completeness, string_interning_consistency]
  types      [Entity, EntityKind, FieldMap, FieldEntry, FieldValue, StringValue, ReferenceList, StringList, Block, VerifyList, VerifyStatement, VerifyKind, SourceSpan]
  ports      [SourceParser]

  requires {
    source_parser_available "SourceParser port is initialized and ready to accept input"
  }

  ensures {
    generic_blocks_parsed "All keyword blocks are parsed as generic entity_block AST nodes regardless of keyword"
    unknown_keywords_accepted "Unknown keywords are parsed without error — rejection deferred to semantic phase"
    raw_body_preserved "Raw body text of each entity block is preserved verbatim before field parsing"
  }

  contract """
    The parser MUST use a single generic entity_block rule that parses
    any keyword name [title] { fields } structure. Only spec, ref, use,
    and define have dedicated grammar rules due to unique structural
    syntax (ref uses scheme:identifier format). All other keywords MUST
    be parsed generically — the parser MUST NOT reject unknown keywords.
    Keyword validation happens in the semantic phase after extensions
    populate the KindRegistry. The parser MUST capture the raw body text
    of each entity block to support Phase 1.5 extension body parsing.
    Raw body text MUST be preserved verbatim before any field parsing
    occurs.
  """

  verify unit "parse any keyword as generic entity_block"
  verify unit "spec block uses dedicated grammar rule"
  verify unit "ref block uses dedicated grammar rule"
  verify unit "define block uses dedicated grammar rule"
  verify unit "unknown keyword parsed without error"
  verify unit "any keyword produces generic entity_block AST node"
  verify unit "generic block preserves kind, name, title, and fields"
  verify unit "parse string field values correctly"
  verify contract "requires/ensures consistency for block type parsing"

}

behavior parse_triple_quoted_strings "Parse Triple-Quoted Strings" {
  invariants [multi_error_collection, string_interning_consistency, zero_domain_knowledge_core]
  types      [SpecFile, StringValue]
  ports      [SourceParser]

  requires {
    source_parser_available "SourceParser port is initialized and ready to accept input"
  }

  ensures {
    newlines_preserved "Internal newlines within triple-quoted strings are preserved"
    dedent_applied "Common leading whitespace is stripped from all lines"
    relative_indent_kept "Relative indentation between lines is preserved after dedent"
  }

  contract """
    The parser MUST handle triple-quoted strings (triple double-quotes).
    Leading whitespace common to all lines MUST be stripped (dedent).
    The content between delimiters MUST preserve internal newlines and
    relative indentation.
  """

  verify unit "triple-quoted string preserves newlines"
  verify unit "common leading whitespace is stripped"
  verify unit "relative indentation is preserved"
  verify unit "recover from unclosed triple-quoted string with diagnostic"
  verify contract "requires/ensures consistency for triple-quoted string parsing"

}

behavior provide_syntax_highlighting_queries "Provide Syntax Highlighting Queries" {
  // Query file behaviors describe static .scm artifacts shipped with the grammar — no runtime types or events needed
  invariants [zero_domain_knowledge_core, query_file_grammar_consistency]

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
  verify integration "highlights.scm loads in Tree-sitter runtime and matches expected captures"

}

behavior provide_code_folding_queries "Provide Code Folding Queries" {
  invariants [zero_domain_knowledge_core, query_file_grammar_consistency]

  contract """
    The grammar MUST ship a folds.scm query file that marks all
    brace-delimited blocks as foldable regions. Every block type,
    sub-block, nested block, and generic_entity_block MUST be
    foldable. The file MUST be loadable by any Tree-sitter-aware
    editor without an LSP server.
  """

  verify unit "folds.scm marks generic entity_block as @fold"
  verify unit "folds.scm marks all brace-delimited sub-blocks within spec and define blocks as @fold"
  verify unit "folds.scm marks spec and define blocks as @fold"
  verify unit "folds.scm marks ref blocks as collapsible regions"
  verify integration "folds.scm loads in Tree-sitter runtime and produces expected fold regions"

}

// Phase 1 (parsing) treats all field values as raw strings; Phase 2 (semantic
// validation) applies type coercion rules. See types/core.spec for the
// canonical FieldValue type and coercion documentation.
behavior parse_verify_statements "Parse Verify Statements" {
  invariants [multi_error_collection, zero_domain_knowledge_core, source_span_completeness, string_interning_consistency]
  types      [SpecFile, VerifyList, VerifyStatement, VerifyKind, SourceSpan]
  ports      [SourceParser]

  requires {
    source_parser_available "SourceParser port is initialized and ready to accept input"
  }

  ensures {
    verify_statements_extracted "Each verify statement is parsed into a VerifyStatement with kind and description fields"
    all_block_types_supported "Verify statements are parsed in entity blocks, spec blocks, and define blocks"
    semantic_validation_deferred "Verify kind validation is deferred to Phase 2 — no kind rejection during parsing"
  }

  contract """
    The core grammar MUST recognize verify statements with the syntax
    verify <kind> "<description>" within any entity block, spec block,
    and define block. Verify is a structural grammar construct — the
    parser MUST parse it in all block types without knowledge of which
    entity kinds support verify semantically. Semantic validation of
    whether the entity kind is testable and the verify kind is allowed
    is deferred to the semantic phase using the KindRegistry. Each
    verify statement MUST be parsed into a VerifyStatement entry with
    the kind and description fields.
    The verify kind token (e.g., unit, property) is parsed as a raw string in Phase 1. Validation against registered verify kinds occurs in Phase 2 semantic validation.
  """

  verify unit "parse verify statement in any entity block"
  verify unit "parse multiple verify statements in same entity"
  verify unit "verify parsed in spec block"
  verify unit "verify parsed in define block"
  verify unit "verify kind and description extracted correctly"
  verify contract "requires/ensures consistency for verify statement parsing"

}

behavior parse_ref_blocks "Parse Ref Blocks" {
  invariants [multi_error_collection, zero_domain_knowledge_core, source_span_completeness, string_interning_consistency]
  types      [SpecFile, ParseError, SourceSpan]
  ports      [SourceParser]
  // Ref components stored as FieldEntry in FieldMap — no dedicated ref type needed

  requires {
    source_parser_available "SourceParser port is initialized and ready to accept input"
  }

  ensures {
    ref_components_extracted "Scheme, kind, and identifier are extracted from compound ID format"
    both_forms_handled "Both block and one-line ref syntax produce consistent AST nodes"
    malformed_refs_rejected "Ref blocks with missing scheme or identifier are rejected with a diagnostic"
  }

  contract """
    The core grammar MUST recognize ref blocks with the syntax
    ref <scheme>.<kind>:<identifier> [title] { fields } as a dedicated
    grammar rule. The parser MUST extract the scheme, kind, and identifier
    components from the compound ID format. Ref blocks also support the
    one-line syntax ref <scheme>.<kind>:<identifier> "title". The parser
    MUST handle both forms and produce consistent AST nodes.
  """

  verify unit "parse ref block with scheme.kind:identifier format"
  verify unit "parse one-line ref syntax"
  verify unit "ref block extracts scheme, kind, and identifier components"
  verify unit "ref block supports optional title and body fields"
  verify unit "reject ref block with missing scheme or identifier"
  verify contract "requires/ensures consistency for ref block parsing"

}

behavior parse_define_blocks "Parse Define Blocks" {
  invariants [multi_error_collection, zero_domain_knowledge_core, source_span_completeness, string_interning_consistency]
  types      [SpecFile, ParseError, SourceSpan, FieldMap, FieldEntry, FieldValue]
  ports      [SourceParser]

  requires {
    source_parser_available "SourceParser port is initialized and ready to accept input"
  }

  ensures {
    define_block_parsed "Define blocks are parsed with name and body using standard field syntax"
    no_extension_knowledge_required "Define blocks are parsed as core grammar constructs without extension input"
  }

  contract """
    The core grammar MUST recognize define blocks with the syntax
    define <name> { fields } as a dedicated grammar rule. Define blocks
    introduce user-defined entity types beyond those provided by
    extensions. The parser MUST parse define blocks identically to other
    structural blocks — they are core grammar constructs, not extension-
    contributed. Define block fields MUST support the same field syntax
    as all other blocks.
  """

  verify unit "parse define block with name and body"
  verify unit "define block supports standard field syntax"
  verify unit "define block parsed without extension knowledge"
  verify contract "requires/ensures consistency for define block parsing"

}

behavior provide_indentation_queries "Provide Indentation Queries" {
  invariants [zero_domain_knowledge_core, query_file_grammar_consistency]

  contract """
    The grammar MUST ship an indents.scm query file that provides
    automatic indentation for brace-delimited and bracket-delimited
    blocks. Opening braces and brackets MUST trigger @indent,
    closing braces and brackets MUST trigger @dedent.
  """

  verify unit "indents.scm indents after opening brace"
  verify unit "indents.scm dedents on closing brace"
  verify unit "indents.scm indents after opening bracket"
  verify unit "indents.scm dedents on closing bracket"
  verify integration "indents.scm loads in Tree-sitter runtime and produces expected indent/dedent"

}

// -- Extension Body Parsing ---------------------------------------------------

behavior delegate_body_parsing_to_extension "Delegate Body Parsing to Extension" {
  invariants [zero_domain_knowledge_core, body_parser_output_conformance]
  types      [Entity, FieldMap, BodyParserContribution, BodyParserError]
  ports      [WasmRuntime]

  requires {
    all_files_parsed_ready "All entity blocks have been parsed with raw body text preserved"
    wasm_runtime_available "WasmRuntime port is initialized and body parser registry is populated"
  }

  ensures {
    body_parsing_delegated "Entities with registered body parsers have raw body replaced by structured JSON fields"
    default_parsing_preserved "Entities without registered body parsers retain default field parsing unchanged"
  }

  contract """
    During Phase 1.5, this behavior orchestrates body parsing delegation.
    For each parsed entity block, the system MUST check the body parser
    registry. If a body parser contribution is registered for the entity's
    kind, the system MUST pass the raw body text to dispatch_body_parser
    (behaviors/wasm-lifecycle.spec) for Wasm execution. The returned
    structured JSON fields MUST replace the raw body in the entity's
    FieldMap before Phase 2 semantic validation. If no body parser is
    registered for an entity kind, the existing field parser MUST be used
    unchanged. All error handling, timeout enforcement, and fallback logic
    is owned by dispatch_body_parser — this behavior only handles
    iteration, registry lookup, and FieldMap replacement.
  """

  verify unit "entity with body parser delegates to dispatch_body_parser"
  verify unit "entity without body parser uses default field parsing"
  verify unit "structured fields replace raw body in FieldMap"
  verify contract "requires/ensures consistency for body parsing delegation"

}
