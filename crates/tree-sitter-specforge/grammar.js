/// <reference types="tree-sitter-cli/dsl" />
// Tree-sitter grammar for the SpecForge .spec DSL
//
// Design principle: ZERO domain knowledge in core.
// Only 4 constructs have dedicated grammar rules:
//   - use (import syntax)
//   - spec (singleton project root)
//   - ref (scheme.kind:identifier compound ID)
//   - define (user-defined entity types)
//
// Everything else is parsed by the generic entity_block rule.
// The parser accepts ANY keyword — validation happens in the semantic phase.

module.exports = grammar({
  name: "specforge",

  extras: ($) => [/\s/, $.comment],

  word: ($) => $.identifier,

  conflicts: ($) => [],

  rules: {
    source_file: ($) =>
      repeat(choice($.use_import, $.pub_use_import, $._block)),

    // --- Comments ----------------------------------------------------
    comment: (_) => token(seq("//", /.*/)),

    // --- Use imports -------------------------------------------------
    import_binding: ($) =>
      seq($.identifier, optional(seq("as", field("alias", $.identifier)))),

    import_bindings: ($) =>
      seq("{", commaSep1($.import_binding), "}"),

    namespace_import: ($) =>
      seq("*", "as", field("alias", $.identifier)),

    use_import: ($) =>
      seq(
        "use",
        choice(
          // Full: use "path"
          field("path", $.string),
          // Selective: use { A, B } from "path"
          seq(field("bindings", $.import_bindings), "from", field("path", $.string)),
          // Namespace: use * as alias from "path"
          seq(field("namespace", $.namespace_import), "from", field("path", $.string)),
        ),
      ),

    pub_use_import: ($) =>
      seq(
        "pub",
        "use",
        choice(
          // Full: pub use "path"
          field("path", $.string),
          // Selective: pub use { A, B } from "path"
          seq(field("bindings", $.import_bindings), "from", field("path", $.string)),
          // Namespace: pub use * as alias from "path"
          seq(field("namespace", $.namespace_import), "from", field("path", $.string)),
        ),
      ),

    // --- Top-level blocks --------------------------------------------
    _block: ($) =>
      choice(
        $.spec_block,
        $.ref_block,
        $.define_block,
        $.union_block,
        $.entity_block,
      ),

    // --- Generic entity block (ANY keyword) --------------------------
    // This is the core of zero-domain-knowledge parsing.
    // Parses: keyword name ["title"] { fields }
    entity_block: ($) =>
      seq(
        field("kind", $.identifier),
        field("name", $.identifier),
        optional(field("title", $.string)),
        "{",
        repeat(choice($.field, $.verify_statement)),
        "}",
      ),

    // --- spec (singleton, dedicated rule) ----------------------------
    spec_block: ($) =>
      seq(
        "spec",
        field("name", $.string),
        "{",
        repeat(choice($.field, $.verify_statement)),
        "}",
      ),

    // --- ref (dedicated rule — compound ID syntax) -------------------
    ref_block: ($) =>
      choice($.ref_inline, $.ref_full),

    ref_inline: ($) =>
      seq(
        "ref",
        field("id", $.scheme_ref_id),
        field("title", $.string),
      ),

    ref_full: ($) =>
      seq(
        "ref",
        field("id", $.scheme_ref_id),
        field("title", $.string),
        "{",
        repeat($.field),
        "}",
      ),

    scheme_ref_id: (_) =>
      token(seq(
        /[a-zA-Z_][a-zA-Z0-9_]*/,
        ".",
        /[a-zA-Z_][a-zA-Z0-9_]*/,
        ":",
        /[^\s"{}()\[\],]+/,
      )),

    // --- define (dedicated rule) -------------------------------------
    define_block: ($) =>
      seq(
        "define",
        field("name", $.identifier),
        "{",
        repeat(choice($.field, $.verify_statement)),
        "}",
      ),

    // --- Union type block: keyword name = variant | variant | ... ---
    union_block: ($) =>
      seq(
        field("kind", $.identifier),
        field("name", $.identifier),
        "=",
        field("variants", $.union_variants),
      ),

    union_variants: ($) =>
      seq($._union_variant, repeat(seq("|", $._union_variant))),

    _union_variant: ($) =>
      choice($.string, $.negative_integer, $.integer, $.identifier),

    negative_integer: (_) => token(seq("-", /[0-9]+/)),

    // --- Shared rules ------------------------------------------------

    // Field: key value [annotations...]
    field: ($) =>
      seq(
        field("key", $.identifier),
        field("value", $._value),
        repeat($.annotation),
      ),

    annotation: ($) =>
      seq(token(seq("@", /[a-zA-Z_][a-zA-Z0-9_]*/)), optional($.string)),

    _value: ($) =>
      choice(
        $.triple_quoted_string,
        $.string,
        $.date_literal,
        $.integer,
        $.boolean,
        $.list,
        $.nested_block,
        $.array_type,
        $.identifier,
      ),

    // Type[] — array type suffix (e.g., ImportDeclaration[])
    array_type: ($) =>
      seq(field("element", $.identifier), token.immediate("[]")),

    // verify [kind] "description"
    verify_statement: ($) =>
      seq(
        "verify",
        optional(field("kind", $.identifier)),
        field("description", $.string),
      ),

    // --- Lists -------------------------------------------------------
    list: ($) =>
      seq(
        "[",
        optional(commaSep1($._list_item)),
        optional(","),
        "]",
      ),

    _list_item: ($) =>
      choice($.string, $.scheme_ref_id, $.integer, $.identifier),

    // Nested block: { key value ... }
    nested_block: ($) =>
      seq("{", repeat($.field), "}"),

    // --- Terminals ---------------------------------------------------

    identifier: (_) => /[a-zA-Z_][a-zA-Z0-9_]*/,

    string: (_) => token(seq('"', repeat(choice(/[^"\\]/, seq("\\", /./))), '"')),

    triple_quoted_string: (_) => {
      return token(seq(
        '"""',
        repeat(choice(
          /[^"]/,
          seq('"', /[^"]/),
          seq('""', /[^"]/),
        )),
        '"""',
      ));
    },

    date_literal: (_) => token(seq(/[0-9]{4}/, "-", /[0-9]{2}/, "-", /[0-9]{2}/)),

    integer: (_) => /[0-9]+/,

    boolean: (_) => choice("true", "false"),
  },
});

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}
