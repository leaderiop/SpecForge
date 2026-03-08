; Keywords
"use" @keyword
"spec" @keyword
"ref" @keyword
"define" @keyword
"verify" @keyword
"true" @keyword.boolean
"false" @keyword.boolean

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

; Numbers
(integer) @number
(date_literal) @number

; Comments
(comment) @comment

; Identifiers in lists
(list (identifier) @variable)

; Import paths
(import_path) @string.special
