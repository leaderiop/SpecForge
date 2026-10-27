; Keywords
"pub" @keyword
"use" @keyword
"spec" @keyword
"ref" @keyword
"define" @keyword
"verify" @keyword

; Generic entity block — kind is a keyword, name is a constant
(entity_block
  kind: (identifier) @keyword
  name: (identifier) @constant)

; Spec block
(spec_block
  name: (string) @constant)

; Define block
(define_block
  name: (identifier) @constant)

; Ref block
(ref_inline
  id: (scheme_ref_id) @constant
  title: (string) @string)
(ref_full
  id: (scheme_ref_id) @constant
  title: (string) @string)

; Verify statements
(verify_statement
  kind: (identifier) @type
  description: (string) @string)

; Fields
(field
  key: (identifier) @property)

; Entity title
(entity_block
  title: (string) @string)

; Strings
(string) @string
(triple_quoted_string) @string

; Numbers, dates and booleans are words; their shape decides the colour (ADR 0285)
(integer) @number
(field
  value: (identifier) @number
  (#match? @number "^-?[0-9]+$"))
(field
  value: (identifier) @number
  (#match? @number "^[0-9]{4}-[0-9]{2}-[0-9]{2}$"))
(field
  value: (identifier) @keyword.boolean
  (#match? @keyword.boolean "^(true|false)$"))
(list
  (identifier) @number
  (#match? @number "^-?[0-9]+$"))
(list
  (identifier) @number
  (#match? @number "^[0-9]{4}-[0-9]{2}-[0-9]{2}$"))
(list
  (identifier) @keyword.boolean
  (#match? @keyword.boolean "^(true|false)$"))

; Comments
(comment) @comment

; Identifiers in lists
(list (identifier) @variable)

; Union blocks
(union_block
  kind: (identifier) @keyword
  name: (identifier) @constant)
(union_variants (identifier) @type)

; Annotations
(annotation) @attribute

; Array types
(array_type
  element: (identifier) @type)

; Negative integers
(negative_integer) @number

; Import keywords
"from" @keyword
"as" @keyword
