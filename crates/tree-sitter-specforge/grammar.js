/// <reference types="tree-sitter-cli/dsl" />
// Tree-sitter grammar for the SpecForge .spec DSL
// Supports all 16 block types across core + two official plugins

module.exports = grammar({
  name: "specforge",

  extras: ($) => [/\s/, $.comment],

  word: ($) => $.identifier,

  conflicts: ($) => [],

  rules: {
    source_file: ($) =>
      repeat(choice($.use_import, $._block)),

    // ─── Comments ──────────────────────────────────────────────
    comment: (_) => token(seq("//", /.*/)),

    // ─── Use imports ───────────────────────────────────────────
    use_import: ($) =>
      seq(
        "use",
        field("path", $.import_path),
        optional(field("selective", $.selective_import)),
      ),

    import_path: (_) =>
      token(seq(
        /[a-zA-Z_][a-zA-Z0-9_-]*/,
        repeat(seq("/", /[a-zA-Z_][a-zA-Z0-9_-]*/)),
      )),

    selective_import: ($) =>
      seq("{", commaSep1($.identifier), "}"),

    // ─── Top-level blocks ──────────────────────────────────────
    _block: ($) =>
      choice(
        $.spec_block,
        $.invariant_block,
        $.behavior_block,
        $.feature_block,
        $.event_block,
        $.type_block,
        $.port_block,
        $.ref_block,
        $.capability_block,
        $.deliverable_block,
        $.roadmap_block,
        $.library_block,
        $.glossary_block,
        $.decision_block,
        $.constraint_block,
        $.failure_mode_block,
        $.define_block,
        $.qualified_entity_block,
      ),

    // ─── Qualified entity block (@plugin/kind) ────────────────
    qualified_entity_block: ($) =>
      seq(
        field("qualified_keyword", $.qualified_entity_keyword),
        field("id", $.identifier),
        optional(field("title", $.string)),
        "{",
        repeat(choice($.key_value, $.verify_statement, $.scenario_block)),
        "}",
      ),

    qualified_entity_keyword: ($) =>
      seq(
        "@",
        field("plugin", $.identifier),
        "/",
        field("kind", $.identifier),
      ),

    // ─── spec (singleton) ──────────────────────────────────────
    spec_block: ($) =>
      seq(
        "spec",
        field("name", $.string),
        "{",
        repeat($._spec_field),
        "}",
      ),

    _spec_field: ($) =>
      choice(
        $.key_value,
        $.persona_def,
        $.surface_def,
        $.providers_block,
        $.coverage_block,
        $.gen_block,
      ),

    persona_def: ($) =>
      seq(
        "persona",
        field("id", $.identifier),
        field("display_name", $.string),
        "{",
        repeat($.key_value),
        "}",
      ),

    surface_def: ($) =>
      seq(
        "surface",
        field("id", $.identifier),
        field("display_name", $.string),
        "{",
        repeat($.key_value),
        "}",
      ),

    providers_block: ($) =>
      seq("providers", "{", repeat($.provider_instance), "}"),

    provider_instance: ($) =>
      seq(
        field("scheme", $.identifier),
        field("alias", $.string),
        "{",
        repeat($.key_value),
        "}",
      ),

    coverage_block: ($) =>
      seq("coverage", "{", repeat($.key_value), "}"),

    gen_block: ($) =>
      seq(
        "gen",
        field("name", $.identifier),
        "{",
        repeat($.key_value),
        "}",
      ),

    // ─── invariant ─────────────────────────────────────────────
    invariant_block: ($) =>
      seq(
        "invariant",
        field("id", $.identifier),
        optional(field("title", $.string)),
        "{",
        repeat(choice($.key_value, $.verify_statement)),
        "}",
      ),

    // ─── behavior ──────────────────────────────────────────────
    behavior_block: ($) =>
      seq(
        "behavior",
        field("id", $.identifier),
        optional(field("title", $.string)),
        "{",
        repeat(choice($.key_value, $.verify_statement, $.scenario_block)),
        "}",
      ),

    // ─── feature ───────────────────────────────────────────────
    feature_block: ($) =>
      seq(
        "feature",
        field("id", $.identifier),
        optional(field("title", $.string)),
        "{",
        repeat($.key_value),
        "}",
      ),

    // ─── event ─────────────────────────────────────────────────
    event_block: ($) =>
      seq(
        "event",
        field("id", $.identifier),
        optional(field("title", $.string)),
        "{",
        repeat(choice($.key_value, $.verify_statement)),
        "}",
      ),

    // ─── type ──────────────────────────────────────────────────
    type_block: ($) =>
      choice($.type_struct, $.type_union),

    type_struct: ($) =>
      seq(
        "type",
        field("name", $.identifier),
        optional($.type_params),
        "{",
        repeat($.type_field),
        "}",
      ),

    type_union: ($) =>
      seq(
        "type",
        field("name", $.identifier),
        optional($.type_params),
        "=",
        $.union_variants,
      ),

    type_params: ($) =>
      seq("<", commaSep1($.identifier), ">"),

    type_field: ($) =>
      seq(
        field("name", $.identifier),
        field("type", $.type_expr),
        repeat($.annotation),
      ),

    type_expr: ($) =>
      choice(
        $.generic_type,
        $.array_type,
        $.optional_type,
        $.string,
        $.identifier,
      ),

    generic_type: ($) =>
      seq(
        $.identifier,
        "<",
        commaSep1(choice($.type_expr, $.error_union)),
        ">",
      ),

    array_type: ($) =>
      seq($.identifier, "[]"),

    optional_type: ($) =>
      seq($.identifier, "?"),

    error_union: ($) =>
      seq($.type_expr, repeat1(seq("|", $.type_expr))),

    union_variants: ($) =>
      seq($.identifier, repeat(seq("|", $.identifier))),

    annotation: (_) =>
      token(seq("@", /[a-zA-Z_][a-zA-Z0-9_]*/)),

    // ─── port ──────────────────────────────────────────────────
    port_block: ($) =>
      seq(
        "port",
        field("name", $.identifier),
        "{",
        repeat(choice($.key_value, $.method_def)),
        "}",
      ),

    method_def: ($) =>
      seq(
        "method",
        field("name", $.identifier),
        "(",
        optional(commaSep1($.param)),
        ")",
        optional(seq("->", field("return_type", $.type_expr))),
      ),

    param: ($) =>
      seq(
        field("name", $.identifier),
        ":",
        field("type", $.type_expr),
      ),

    // ─── ref ───────────────────────────────────────────────────
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
        repeat($.key_value),
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

    // ─── capability (@specforge/product) ───────────────────────
    capability_block: ($) =>
      seq(
        "capability",
        field("id", $.identifier),
        optional(field("title", $.string)),
        "{",
        repeat(choice($.key_value, $.scenario_block)),
        "}",
      ),

    // ─── deliverable (@specforge/product) ──────────────────────
    deliverable_block: ($) =>
      seq(
        "deliverable",
        field("id", $.identifier),
        optional(field("title", $.string)),
        "{",
        repeat($.key_value),
        "}",
      ),

    // ─── roadmap (@specforge/product) ──────────────────────────
    roadmap_block: ($) =>
      seq(
        "roadmap",
        field("id", $.identifier),
        optional(field("title", $.string)),
        "{",
        repeat($.key_value),
        "}",
      ),

    // ─── library (@specforge/product) ──────────────────────────
    library_block: ($) =>
      seq(
        "library",
        field("id", $.identifier),
        optional(field("title", $.string)),
        "{",
        repeat($.key_value),
        "}",
      ),

    // ─── glossary (singleton, @specforge/product) ──────────────
    glossary_block: ($) =>
      seq(
        "glossary",
        "{",
        repeat($.term_def),
        "}",
      ),

    term_def: ($) =>
      seq(
        "term",
        field("name", $.string),
        "{",
        repeat($.key_value),
        "}",
      ),

    // ─── decision (@specforge/governance) ──────────────────────
    decision_block: ($) =>
      seq(
        "decision",
        field("id", $.identifier),
        optional(field("title", $.string)),
        "{",
        repeat($.key_value),
        "}",
      ),

    // ─── constraint (@specforge/governance) ────────────────────
    constraint_block: ($) =>
      seq(
        "constraint",
        field("id", $.identifier),
        optional(field("title", $.string)),
        "{",
        repeat(choice($.key_value, $.verify_statement)),
        "}",
      ),

    // ─── failure_mode (@specforge/governance) ──────────────────
    failure_mode_block: ($) =>
      seq(
        "failure_mode",
        field("id", $.identifier),
        optional(field("title", $.string)),
        "{",
        repeat($.key_value),
        "}",
      ),

    // ─── define (custom entity types) ─────────────────────────
    // define my_entity {
    //   testable true
    //   field_name field_type
    // }
    define_block: ($) =>
      seq(
        "define",
        field("name", $.identifier),
        "{",
        repeat($.key_value),
        "}",
      ),

    // ─── Shared rules ──────────────────────────────────────────

    // Key-value pair: `field_name value`
    key_value: ($) =>
      seq(
        field("key", $.identifier),
        field("value", $._value),
      ),

    _value: ($) =>
      choice(
        $.triple_quoted_string,
        $.string,
        $.date_literal,
        $.integer,
        $.boolean,
        $.list,
        $.nested_block,
        $.array_identifier,
        $.identifier,
      ),

    // identifier[] — used in event payload blocks (e.g., `files string[]`)
    array_identifier: ($) =>
      seq($.identifier, "[]"),

    // verify unit "description"
    verify_statement: ($) =>
      seq(
        "verify",
        field("kind", $.identifier),
        field("description", $.string),
      ),

    // scenario "title" { given/when/then steps }
    scenario_block: ($) =>
      seq(
        "scenario",
        field("title", $.string),
        "{",
        repeat($._scenario_step),
        "}",
      ),

    _scenario_step: ($) =>
      choice($.given_step, $.when_step, $.then_step),

    given_step: ($) =>
      seq("given", field("description", $.string)),

    when_step: ($) =>
      seq("when", field("description", $.string)),

    then_step: ($) =>
      seq("then", field("description", $.string)),

    // ─── Lists ─────────────────────────────────────────────────
    list: ($) =>
      seq(
        "[",
        optional(commaSep1($._list_item)),
        optional(","),
        "]",
      ),

    _list_item: ($) =>
      choice($.string, $.scheme_ref_id, $.integer, $.identifier),

    // Nested block: `post_mitigation { ... }` or event `payload { ... }`
    nested_block: ($) =>
      seq("{", repeat($.key_value), "}"),

    // ─── Terminals ─────────────────────────────────────────────

    identifier: (_) => /[a-zA-Z_][a-zA-Z0-9_]*/,

    string: (_) => token(seq('"', repeat(choice(/[^"\\]/, seq("\\", /./
    ))), '"')),

    triple_quoted_string: (_) => {
      // Match """ followed by any content (including newlines) until the next """
      // Tree-sitter regexes don't support [^] or dotall, so we match character classes
      return token(seq(
        '"""',
        repeat(choice(
          /[^"]/,           // any non-quote character
          seq('"', /[^"]/), // a single quote not starting ""
          seq('""', /[^"]/), // two quotes not starting """
        )),
        '"""',
      ));
    },

    // Date literal: YYYY-MM-DD (must be before integer to take priority)
    date_literal: (_) => token(seq(/[0-9]{4}/, "-", /[0-9]{2}/, "-", /[0-9]{2}/)),

    integer: (_) => /[0-9]+/,

    boolean: (_) => choice("true", "false"),
  },
});

/**
 * Comma-separated list with at least one element.
 */
function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}
