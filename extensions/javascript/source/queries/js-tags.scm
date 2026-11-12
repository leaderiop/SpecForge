; @javascript/source: the definitions of a JavaScript or TypeScript file, for the host's syntax import.
; Read by the typescript grammar (.ts .mts .cts) and the tsx grammar (.tsx .js .jsx .mjs .cjs).
;
; Captures (the tags convention, ADR 0344 D1: each pattern has exactly one role capture):
;   @definition.<kind>  the defining node: function class interface type enum namespace const variable method field,
;                       or scope (a block whose contents are no item: `declare module "x"`, `declare global`)
;   @name               its name, one per segment of a dotted namespace name; a definition with none is no item,
;                       unless it is a default export (`default`)
;   @part               a string member name's fragments and escape sequences, in order: the pack decodes them
;   @attr               a modifier token: export default declare abstract static async get set readonly override
;                       const let var * exports, or an accessibility (public private protected)
;   @signature          the parts whose text the pack digests: type parameters, parameters, return and declared
;                       types, heritage, a type alias's, an interface's or an enum's body; never a value
;   @reference.doc      a /** comment, its text @doc: the pack gives it to the item that starts on its last line or
;                       the line after it
;   @lead               a decorator written before a class's `export` or a method: the item's first line, for its doc
;   @reference.export   an export list's or module.exports's entry: the item @name names is exported (its @attr
;                       `default` when it is the default export)
;   @reference.literal  a string's, a template's or a regular expression's own text holding whitespace: the pack
;                       keeps that whitespace as written in a signature it lies in
; A capture whose name starts with _ only feeds a predicate.
;
; Matches of one defining node merge in the pack (captures united; function and class win over const and variable).
; A declaration is an item only at item scope: the program or a namespace's body. Every pattern below is a declaration
; wrapped in one such context; a member is kept by the pack only when its nearest enclosing item is a class or an
; interface. Each block is one set of declarations of one role kind in each of the seven item-scope contexts (the
; program, a declare at the program, a namespace body, a module body, a declare in either body, an export, an export
; declare): a new declaration goes into every context of its set.

; ---------------------------------------------------------------- function
(program [
  (function_declaration name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (generator_function_declaration "*" @attr name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (function_signature name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
])
(program (ambient_declaration "declare" @attr [
  (function_declaration name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (generator_function_declaration "*" @attr name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (function_signature name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
]))
(internal_module body: (statement_block [
  (function_declaration name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (generator_function_declaration "*" @attr name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (function_signature name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
]))
(module body: (statement_block [
  (function_declaration name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (generator_function_declaration "*" @attr name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (function_signature name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
]))
(statement_block (ambient_declaration "declare" @attr [
  (function_declaration name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (generator_function_declaration "*" @attr name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (function_signature name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
]))
(export_statement "export" @attr ("default" @attr)? declaration: [
  (function_declaration name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (generator_function_declaration "*" @attr name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (function_signature name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
])
(export_statement "export" @attr declaration: (ambient_declaration "declare" @attr [
  (function_declaration name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (generator_function_declaration "*" @attr name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
  (function_signature name: (identifier) @name type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.function
]))
; ---------------------------------------------------------------- class
(program [
  (class_declaration name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
  (abstract_class_declaration "abstract" @attr name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
])
(program (ambient_declaration "declare" @attr [
  (class_declaration name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
  (abstract_class_declaration "abstract" @attr name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
]))
(internal_module body: (statement_block [
  (class_declaration name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
  (abstract_class_declaration "abstract" @attr name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
]))
(module body: (statement_block [
  (class_declaration name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
  (abstract_class_declaration "abstract" @attr name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
]))
(statement_block (ambient_declaration "declare" @attr [
  (class_declaration name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
  (abstract_class_declaration "abstract" @attr name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
]))
(export_statement (decorator)* @lead "export" @attr ("default" @attr)? declaration: [
  (class_declaration name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
  (abstract_class_declaration "abstract" @attr name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
])
(export_statement "export" @attr declaration: (ambient_declaration "declare" @attr [
  (class_declaration name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
  (abstract_class_declaration "abstract" @attr name: (type_identifier) @name type_parameters: (_)? @signature (class_heritage)? @signature) @definition.class
]))
; ---------------------------------------------------------------- interface
(program (interface_declaration name: (type_identifier) @name type_parameters: (_)? @signature (extends_type_clause)? @signature body: (_) @signature) @definition.interface)
(program (ambient_declaration "declare" @attr (interface_declaration name: (type_identifier) @name type_parameters: (_)? @signature (extends_type_clause)? @signature body: (_) @signature) @definition.interface))
(internal_module body: (statement_block (interface_declaration name: (type_identifier) @name type_parameters: (_)? @signature (extends_type_clause)? @signature body: (_) @signature) @definition.interface))
(module body: (statement_block (interface_declaration name: (type_identifier) @name type_parameters: (_)? @signature (extends_type_clause)? @signature body: (_) @signature) @definition.interface))
(statement_block (ambient_declaration "declare" @attr (interface_declaration name: (type_identifier) @name type_parameters: (_)? @signature (extends_type_clause)? @signature body: (_) @signature) @definition.interface))
(export_statement "export" @attr ("default" @attr)? declaration: (interface_declaration name: (type_identifier) @name type_parameters: (_)? @signature (extends_type_clause)? @signature body: (_) @signature) @definition.interface)
(export_statement "export" @attr declaration: (ambient_declaration "declare" @attr (interface_declaration name: (type_identifier) @name type_parameters: (_)? @signature (extends_type_clause)? @signature body: (_) @signature) @definition.interface))
; ---------------------------------------------------------------- type
(program (type_alias_declaration name: (type_identifier) @name type_parameters: (_)? @signature value: (_) @signature) @definition.type)
(program (ambient_declaration "declare" @attr (type_alias_declaration name: (type_identifier) @name type_parameters: (_)? @signature value: (_) @signature) @definition.type))
(internal_module body: (statement_block (type_alias_declaration name: (type_identifier) @name type_parameters: (_)? @signature value: (_) @signature) @definition.type))
(module body: (statement_block (type_alias_declaration name: (type_identifier) @name type_parameters: (_)? @signature value: (_) @signature) @definition.type))
(statement_block (ambient_declaration "declare" @attr (type_alias_declaration name: (type_identifier) @name type_parameters: (_)? @signature value: (_) @signature) @definition.type))
(export_statement "export" @attr ("default" @attr)? declaration: (type_alias_declaration name: (type_identifier) @name type_parameters: (_)? @signature value: (_) @signature) @definition.type)
(export_statement "export" @attr declaration: (ambient_declaration "declare" @attr (type_alias_declaration name: (type_identifier) @name type_parameters: (_)? @signature value: (_) @signature) @definition.type))
; ---------------------------------------------------------------- enum
(program (enum_declaration name: (identifier) @name body: (_) @signature) @definition.enum)
(program (ambient_declaration "declare" @attr (enum_declaration name: (identifier) @name body: (_) @signature) @definition.enum))
(internal_module body: (statement_block (enum_declaration name: (identifier) @name body: (_) @signature) @definition.enum))
(module body: (statement_block (enum_declaration name: (identifier) @name body: (_) @signature) @definition.enum))
(statement_block (ambient_declaration "declare" @attr (enum_declaration name: (identifier) @name body: (_) @signature) @definition.enum))
(export_statement "export" @attr ("default" @attr)? declaration: (enum_declaration name: (identifier) @name body: (_) @signature) @definition.enum)
(export_statement "export" @attr declaration: (ambient_declaration "declare" @attr (enum_declaration name: (identifier) @name body: (_) @signature) @definition.enum))
; ---------------------------------------------------------------- namespace
(program [(expression_statement (internal_module name: [(identifier) @name (nested_identifier)]) @definition.namespace) (module name: [(identifier) @name (nested_identifier)]) @definition.namespace])
(program (ambient_declaration "declare" @attr [
  (internal_module name: [(identifier) @name (nested_identifier)]) @definition.namespace
  (module name: [(identifier) @name (nested_identifier)]) @definition.namespace
]))
(internal_module body: (statement_block [
  (internal_module name: [(identifier) @name (nested_identifier)]) @definition.namespace
  (module name: [(identifier) @name (nested_identifier)]) @definition.namespace
]))
(module body: (statement_block [
  (internal_module name: [(identifier) @name (nested_identifier)]) @definition.namespace
  (module name: [(identifier) @name (nested_identifier)]) @definition.namespace
]))
(statement_block (ambient_declaration "declare" @attr [
  (internal_module name: [(identifier) @name (nested_identifier)]) @definition.namespace
  (module name: [(identifier) @name (nested_identifier)]) @definition.namespace
]))
(export_statement "export" @attr ("default" @attr)? declaration: [
  (internal_module name: [(identifier) @name (nested_identifier)]) @definition.namespace
  (module name: [(identifier) @name (nested_identifier)]) @definition.namespace
])
(export_statement "export" @attr declaration: (ambient_declaration "declare" @attr [
  (internal_module name: [(identifier) @name (nested_identifier)]) @definition.namespace
  (module name: [(identifier) @name (nested_identifier)]) @definition.namespace
]))
; ---------------------------------------------------------------- a dotted namespace name: one @name per segment
; These merge into the namespace's match above, whatever its context (the pack drops one at no item scope).
; shortcut: a name of more than six segments has none and is no item: a query has no pattern that reaches a segment
; at any depth, and the pack never splits the name's text (a space or a comment between segments would fool it);
; upgrade with a pattern per further segment.
[(internal_module name: (nested_identifier object: (identifier) @name property: (property_identifier) @name)) (module name: (nested_identifier object: (identifier) @name property: (property_identifier) @name))] @definition.namespace
[(internal_module name: (nested_identifier object: (member_expression object: (identifier) @name property: (property_identifier) @name) property: (property_identifier) @name)) (module name: (nested_identifier object: (member_expression object: (identifier) @name property: (property_identifier) @name) property: (property_identifier) @name))] @definition.namespace
[(internal_module name: (nested_identifier object: (member_expression object: (member_expression object: (identifier) @name property: (property_identifier) @name) property: (property_identifier) @name) property: (property_identifier) @name)) (module name: (nested_identifier object: (member_expression object: (member_expression object: (identifier) @name property: (property_identifier) @name) property: (property_identifier) @name) property: (property_identifier) @name))] @definition.namespace
[(internal_module name: (nested_identifier object: (member_expression object: (member_expression object: (member_expression object: (identifier) @name property: (property_identifier) @name) property: (property_identifier) @name) property: (property_identifier) @name) property: (property_identifier) @name)) (module name: (nested_identifier object: (member_expression object: (member_expression object: (member_expression object: (identifier) @name property: (property_identifier) @name) property: (property_identifier) @name) property: (property_identifier) @name) property: (property_identifier) @name))] @definition.namespace
[(internal_module name: (nested_identifier object: (member_expression object: (member_expression object: (member_expression object: (member_expression object: (identifier) @name property: (property_identifier) @name) property: (property_identifier) @name) property: (property_identifier) @name) property: (property_identifier) @name) property: (property_identifier) @name)) (module name: (nested_identifier object: (member_expression object: (member_expression object: (member_expression object: (member_expression object: (identifier) @name property: (property_identifier) @name) property: (property_identifier) @name) property: (property_identifier) @name) property: (property_identifier) @name) property: (property_identifier) @name))] @definition.namespace
; ---------------------------------------------------------------- variable
(program [
  (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
])
(program (ambient_declaration "declare" @attr [
  (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
]))
(internal_module body: (statement_block [
  (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
]))
(module body: (statement_block [
  (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
]))
(statement_block (ambient_declaration "declare" @attr [
  (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
]))
(export_statement "export" @attr ("default" @attr)? declaration: [
  (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
])
(export_statement "export" @attr declaration: (ambient_declaration "declare" @attr [
  (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @definition.function)
]))
(program (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: (class)) @definition.class))
(program (ambient_declaration "declare" @attr (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: (class)) @definition.class)))
(internal_module body: (statement_block (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: (class)) @definition.class)))
(module body: (statement_block (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: (class)) @definition.class)))
(statement_block (ambient_declaration "declare" @attr (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: (class)) @definition.class)))
(export_statement "export" @attr ("default" @attr)? declaration: (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: (class)) @definition.class))
(export_statement "export" @attr declaration: (ambient_declaration "declare" @attr (lexical_declaration kind: _ @attr (variable_declarator name: (identifier) @name value: (class)) @definition.class)))
(program (lexical_declaration kind: "const" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.const))
(program (ambient_declaration "declare" @attr (lexical_declaration kind: "const" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.const)))
(internal_module body: (statement_block (lexical_declaration kind: "const" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.const)))
(module body: (statement_block (lexical_declaration kind: "const" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.const)))
(statement_block (ambient_declaration "declare" @attr (lexical_declaration kind: "const" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.const)))
(export_statement "export" @attr ("default" @attr)? declaration: (lexical_declaration kind: "const" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.const))
(export_statement "export" @attr declaration: (ambient_declaration "declare" @attr (lexical_declaration kind: "const" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.const)))
(program [
  (lexical_declaration kind: "let" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
])
(program (ambient_declaration "declare" @attr [
  (lexical_declaration kind: "let" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
]))
(internal_module body: (statement_block [
  (lexical_declaration kind: "let" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
]))
(module body: (statement_block [
  (lexical_declaration kind: "let" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
]))
(statement_block (ambient_declaration "declare" @attr [
  (lexical_declaration kind: "let" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
]))
(export_statement "export" @attr ("default" @attr)? declaration: [
  (lexical_declaration kind: "let" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
])
(export_statement "export" @attr declaration: (ambient_declaration "declare" @attr [
  (lexical_declaration kind: "let" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
  (variable_declaration "var" @attr (variable_declarator name: (identifier) @name type: (_)? @signature) @definition.variable)
]))
; ---------------------------------------------------------------- default exports with no name: the item `default`
(export_statement "export" @attr "default" @attr value: [(function_expression) (arrow_function) (generator_function)] @definition.function)
(export_statement (decorator)* @lead "export" @attr "default" @attr value: (class) @definition.class)
(export_statement "export" @attr "default" @attr value: [(object) (array) (call_expression) (new_expression) (string) (number) (template_string) (member_expression) (as_expression) (satisfies_expression)] @definition.const)

; ---------------------------------------------------------------- blocks whose contents are no item
(module name: (string) @name) @definition.scope
(ambient_declaration "global" @name) @definition.scope

; ---------------------------------------------------------------- members (kept under a class or an interface)
(class_body (method_definition name: [(property_identifier) @name (private_property_identifier) @name (string)] type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.method)
(class_body (method_signature name: [(property_identifier) @name (private_property_identifier) @name (string)] type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.method)
(class_body (abstract_method_signature name: [(property_identifier) @name (private_property_identifier) @name (string)] type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.method)
(class_body (public_field_definition name: [(property_identifier) @name (private_property_identifier) @name (string)] type: (_)? @signature) @definition.field)
(interface_body (method_signature name: [(property_identifier) @name (string)] type_parameters: (_)? @signature parameters: (_) @signature return_type: (_)? @signature) @definition.method)
(interface_body (property_signature name: [(property_identifier) @name (string)] type: (_)? @signature) @definition.field)
; A string member name's parts (a quantified capture inside an alternation keeps only its first node): they merge.
(class_body (method_definition name: (string [(string_fragment) (escape_sequence)]* @part)) @definition.method)
(class_body (method_signature name: (string [(string_fragment) (escape_sequence)]* @part)) @definition.method)
(class_body (abstract_method_signature name: (string [(string_fragment) (escape_sequence)]* @part)) @definition.method)
(class_body (public_field_definition name: (string [(string_fragment) (escape_sequence)]* @part)) @definition.field)
(interface_body (method_signature name: (string [(string_fragment) (escape_sequence)]* @part)) @definition.method)
(interface_body (property_signature name: (string [(string_fragment) (escape_sequence)]* @part)) @definition.field)
; A method's decorators are its class body's children before it (a field's are its own): the first starts it.
(class_body (decorator)+ @lead . (method_definition) @definition.method)

; ---------------------------------------------------------------- modifiers: matches that merge into their definition
(function_declaration "async" @attr) @definition.function
(generator_function_declaration "async" @attr) @definition.function
(method_definition "static" @attr) @definition.method
(method_definition "async" @attr) @definition.method
(method_definition "get" @attr) @definition.method
(method_definition "set" @attr) @definition.method
(method_definition "*" @attr) @definition.method
(method_definition (accessibility_modifier) @attr) @definition.method
(method_definition (override_modifier) @attr) @definition.method
(method_signature (accessibility_modifier) @attr) @definition.method
(abstract_method_signature "abstract" @attr) @definition.method
(public_field_definition "static" @attr) @definition.field
(public_field_definition "readonly" @attr) @definition.field
(public_field_definition "declare" @attr) @definition.field
(public_field_definition "abstract" @attr) @definition.field
(public_field_definition (accessibility_modifier) @attr) @definition.field
(enum_declaration "const" @attr) @definition.enum

; ---------------------------------------------------------------- exports by name
(export_statement !source (export_clause (export_specifier name: (identifier) @name) @reference.export))
(export_statement "default" @attr value: (identifier) @name) @reference.export
(export_statement !source (export_clause (export_specifier name: (identifier) @name alias: (_) @attr) @reference.export)
  (#eq? @attr "default"))
(program (expression_statement (assignment_expression
  left: (member_expression object: (identifier) @_module property: (property_identifier) @_exports)
  right: (object [(shorthand_property_identifier) @name (pair value: (identifier) @name)] @reference.export))
  (#eq? @_module "module") (#eq? @_exports "exports")))

; ---------------------------------------------------------------- CommonJS: exports.x = … and module.exports.x = …
(program (expression_statement (assignment_expression
  left: (member_expression object: (identifier) @attr property: (property_identifier) @name)
  right: [(function_expression parameters: (_) @signature) (arrow_function) (generator_function parameters: (_) @signature)]) @definition.function)
  (#eq? @attr "exports"))
(program (expression_statement (assignment_expression
  left: (member_expression object: (identifier) @attr property: (property_identifier) @name)
  right: (_)) @definition.const)
  (#eq? @attr "exports"))
(program (expression_statement (assignment_expression
  left: (member_expression object: (member_expression object: (identifier) @_module property: (property_identifier) @attr) property: (property_identifier) @name)
  right: [(function_expression parameters: (_) @signature) (arrow_function) (generator_function parameters: (_) @signature)]) @definition.function)
  (#eq? @_module "module") (#eq? @attr "exports"))
(program (expression_statement (assignment_expression
  left: (member_expression object: (member_expression object: (identifier) @_module property: (property_identifier) @attr) property: (property_identifier) @name)
  right: (_)) @definition.const)
  (#eq? @_module "module") (#eq? @attr "exports"))

; ---------------------------------------------------------------- documentation
((comment) @doc @reference.doc (#match? @doc "^/\\*\\*"))

; ---------------------------------------------------------------- literal text a signature keeps as written
([(string_fragment) (regex_pattern)] @_text @reference.literal (#match? @_text "\\s"))
