; @rust/source's query over a Rust file (the tags convention, ADR 0344 D1): every definition an item may be, and
; the outer attributes and doc comments the pack attaches to the definition directly below them (ADR 0348 D6, D7).

; Definitions.
(mod_item (visibility_modifier)? @visibility name: (identifier) @name body: (declaration_list)? @body)
  @definition.module
(function_item (visibility_modifier)? @visibility name: (identifier) @name
  type_parameters: (type_parameters)? @signature parameters: (parameters) @signature return_type: (_)? @signature)
  @definition.function
(function_signature_item (visibility_modifier)? @visibility name: (identifier) @name
  type_parameters: (type_parameters)? @signature parameters: (parameters) @signature return_type: (_)? @signature)
  @definition.function
(struct_item (visibility_modifier)? @visibility name: (type_identifier) @name) @definition.struct
(enum_item (visibility_modifier)? @visibility name: (type_identifier) @name) @definition.enum
(union_item (visibility_modifier)? @visibility name: (type_identifier) @name) @definition.union
(trait_item (visibility_modifier)? @visibility name: (type_identifier) @name) @definition.trait
(type_item (visibility_modifier)? @visibility name: (type_identifier) @name) @definition.type
(associated_type name: (type_identifier) @name) @definition.type
(const_item (visibility_modifier)? @visibility name: (identifier) @name) @definition.const
(static_item (visibility_modifier)? @visibility name: (identifier) @name) @definition.static
(macro_definition name: (identifier) @name) @definition.macro
; Not an item: the owner of the functions, constants and types inside it.
(impl_item trait: (_)? @trait type: (_) @type) @definition.impl

; An outer attribute: its path, and the whole attribute (`cfg(unix)`).
(attribute_item (attribute . (_) @path) @attribute) @reference.attribute
; The file a module declaration names: `#[path = "…"]`, or each `path = "…"` of a `#[cfg_attr(…)]`.
(attribute_item
  (attribute . (identifier) @_path (#eq? @_path "path") value: (string_literal (string_content) @file)))
  @reference.module_file
(attribute_item
  (attribute . (identifier) @_cfg_attr (#eq? @_cfg_attr "cfg_attr")
    arguments: (token_tree (identifier) @_path (#eq? @_path "path") . (string_literal (string_content) @file))))
  @reference.module_file
; The first `<kind> = "<id>"` of a spec-link attribute: the entity a test names.
(attribute_item
  (attribute . (_) @path
    (#any-of? @path "specforge_test" "spec" "specforge_test_macros::test" "specforge_test::test" "specforge::test")
    arguments: (token_tree . (identifier) . (string_literal (string_content) @id))))
  @reference.entity
; An outer doc comment, `///` or `/** */`.
(line_comment outer: (outer_doc_comment_marker) doc: (doc_comment) @doc) @reference.doc
(block_comment outer: (outer_doc_comment_marker) doc: (doc_comment) @doc) @reference.doc
