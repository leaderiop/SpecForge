; @javascript/source: the test calls of a JavaScript or TypeScript test file, for the host's syntax import (ADR 0393 D4).
; Run only on test files. Read by the typescript grammar (.ts .mts .cts) and the tsx grammar (.tsx .js .jsx .mjs .cjs).
;
; Captures (the tags convention, ADR 0344 D1: each pattern has exactly one role capture):
;   @definition.test     a call of `test` or `it`, or of one of their members (`test.skip`, `it.concurrent`)
;   @definition.suite    a call of `describe` or `suite`, or of one of their members; a call that a runner's member
;                        call returns (`test.each([…])("n %s", fn)`)
;   @definition.subtest  a call `<name>.test(…)`: the pack counts it only inside a test
;   @name                the first argument, when it is a string or a template: the call's name is static
;   @part                that string's or template's fragments and escape sequences, in order: the pack decodes them
;   @dynamic             a template's substitution: the name is not static
;   @modifier            `skip`, `todo` or `only` called on `test`, `it`, `describe` or `suite`
;   @callback            a function written as an argument after the first
;   @skip_option         an options object argument with a `skip` key (`{ skip: true }`, `{ skip }`): node may skip it
; A capture whose name starts with _ only feeds a predicate.
;
; Every call of a runner's name matches its plain pattern, whatever its arguments, so a call with a dynamic name still
; holds the calls inside it (the pack mints nothing under it); the matches of one call merge in the pack. A member
; call other than skip, todo and only (`.each`, `.concurrent`) gets no @name: it is never minted.

; ---------------------------------------------------------------- test, it
((call_expression function: (identifier) @_callee) @definition.test
  (#any-of? @_callee "test" "it"))
((call_expression function: (identifier) @_callee
  arguments: (arguments . (string [(string_fragment) (escape_sequence)]* @part) @name)) @definition.test
  (#any-of? @_callee "test" "it"))
((call_expression function: (identifier) @_callee
  arguments: (arguments . (template_string [(string_fragment) (escape_sequence)]* @part) @name)) @definition.test
  (#any-of? @_callee "test" "it"))
((call_expression function: (identifier) @_callee
  arguments: (arguments . (template_string (template_substitution) @dynamic))) @definition.test
  (#any-of? @_callee "test" "it"))
((call_expression function: (identifier) @_callee
  arguments: (arguments . (_) [(arrow_function) (function_expression) (generator_function)] @callback)) @definition.test
  (#any-of? @_callee "test" "it"))

; ---------------------------------------------------------------- test.skip, it.todo, test.only, it.concurrent, …
((call_expression function: (member_expression object: (identifier) @_callee)) @definition.test
  (#any-of? @_callee "test" "it"))
((call_expression function: (member_expression object: (identifier) @_callee property: (property_identifier) @modifier)
  arguments: (arguments . (string [(string_fragment) (escape_sequence)]* @part) @name)) @definition.test
  (#any-of? @_callee "test" "it") (#any-of? @modifier "skip" "todo" "only"))
((call_expression function: (member_expression object: (identifier) @_callee property: (property_identifier) @modifier)
  arguments: (arguments . (template_string [(string_fragment) (escape_sequence)]* @part) @name)) @definition.test
  (#any-of? @_callee "test" "it") (#any-of? @modifier "skip" "todo" "only"))
((call_expression function: (member_expression object: (identifier) @_callee property: (property_identifier) @modifier)
  arguments: (arguments . (template_string (template_substitution) @dynamic))) @definition.test
  (#any-of? @_callee "test" "it") (#any-of? @modifier "skip" "todo" "only"))

; ---------------------------------------------------------------- describe, suite
((call_expression function: (identifier) @_callee) @definition.suite
  (#any-of? @_callee "describe" "suite"))
((call_expression function: (identifier) @_callee
  arguments: (arguments . (string [(string_fragment) (escape_sequence)]* @part) @name)) @definition.suite
  (#any-of? @_callee "describe" "suite"))
((call_expression function: (identifier) @_callee
  arguments: (arguments . (template_string [(string_fragment) (escape_sequence)]* @part) @name)) @definition.suite
  (#any-of? @_callee "describe" "suite"))
((call_expression function: (identifier) @_callee
  arguments: (arguments . (template_string (template_substitution) @dynamic))) @definition.suite
  (#any-of? @_callee "describe" "suite"))

; ---------------------------------------------------------------- describe.skip, suite.only, describe.concurrent, …
((call_expression function: (member_expression object: (identifier) @_callee)) @definition.suite
  (#any-of? @_callee "describe" "suite"))
((call_expression function: (member_expression object: (identifier) @_callee property: (property_identifier) @modifier)
  arguments: (arguments . (string [(string_fragment) (escape_sequence)]* @part) @name)) @definition.suite
  (#any-of? @_callee "describe" "suite") (#any-of? @modifier "skip" "todo" "only"))
((call_expression function: (member_expression object: (identifier) @_callee property: (property_identifier) @modifier)
  arguments: (arguments . (template_string [(string_fragment) (escape_sequence)]* @part) @name)) @definition.suite
  (#any-of? @_callee "describe" "suite") (#any-of? @modifier "skip" "todo" "only"))
((call_expression function: (member_expression object: (identifier) @_callee property: (property_identifier) @modifier)
  arguments: (arguments . (template_string (template_substitution) @dynamic))) @definition.suite
  (#any-of? @_callee "describe" "suite") (#any-of? @modifier "skip" "todo" "only"))

; ---------------------------------------------------------------- test.each([…])(…), describe.skip.each`…`(…)
((call_expression function: (call_expression
  function: (member_expression object: [(identifier) @_callee (member_expression object: (identifier) @_callee)])))
  @definition.suite
  (#any-of? @_callee "test" "it" "describe" "suite"))

; ---------------------------------------------------------------- t.test (a subtest)
((call_expression function: (member_expression object: (identifier) property: (property_identifier) @_method)) @definition.subtest
  (#eq? @_method "test"))
((call_expression function: (member_expression object: (identifier) property: (property_identifier) @_method)
  arguments: (arguments . (string [(string_fragment) (escape_sequence)]* @part) @name)) @definition.subtest
  (#eq? @_method "test"))
((call_expression function: (member_expression object: (identifier) property: (property_identifier) @_method)
  arguments: (arguments . (template_string [(string_fragment) (escape_sequence)]* @part) @name)) @definition.subtest
  (#eq? @_method "test"))
((call_expression function: (member_expression object: (identifier) property: (property_identifier) @_method)
  arguments: (arguments . (template_string (template_substitution) @dynamic))) @definition.subtest
  (#eq? @_method "test"))
((call_expression function: (member_expression object: (identifier) property: (property_identifier) @_method)
  arguments: (arguments . (_) [(arrow_function) (function_expression) (generator_function)] @callback))
  @definition.subtest
  (#eq? @_method "test"))

; ---------------------------------------------------------------- { skip: … } options
((call_expression function: [(identifier) @_callee (member_expression object: (identifier) @_callee)]
  arguments: (arguments (object [(pair key: (property_identifier) @_key) (pair key: (string (string_fragment) @_key))
    (shorthand_property_identifier) @_key]) @skip_option)) @definition.test
  (#any-of? @_callee "test" "it") (#eq? @_key "skip"))
((call_expression function: [(identifier) @_callee (member_expression object: (identifier) @_callee)]
  arguments: (arguments (object [(pair key: (property_identifier) @_key) (pair key: (string (string_fragment) @_key))
    (shorthand_property_identifier) @_key]) @skip_option)) @definition.suite
  (#any-of? @_callee "describe" "suite") (#eq? @_key "skip"))
((call_expression function: (member_expression object: (identifier) property: (property_identifier) @_method)
  arguments: (arguments (object [(pair key: (property_identifier) @_key) (pair key: (string (string_fragment) @_key))
    (shorthand_property_identifier) @_key]) @skip_option)) @definition.subtest
  (#eq? @_method "test") (#eq? @_key "skip"))
