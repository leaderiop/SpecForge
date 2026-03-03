; highlights.scm — Syntax highlighting for SpecForge .spec files
; Works with any Tree-sitter-aware editor (Neovim, Helix, Zed, Emacs)

; ── Comments ──────────────────────────────────────────────────
(comment) @comment @spell

; ── Block keywords ────────────────────────────────────────────
(spec_block "spec" @keyword)
(invariant_block "invariant" @keyword)
(behavior_block "behavior" @keyword)
(feature_block "feature" @keyword)
(event_block "event" @keyword)
(type_struct "type" @keyword)
(type_union "type" @keyword)
(port_block "port" @keyword)
(ref_inline "ref" @keyword)
(ref_full "ref" @keyword)
(capability_block "capability" @keyword)
(deliverable_block "deliverable" @keyword)
(roadmap_block "roadmap" @keyword)
(library_block "library" @keyword)
(glossary_block "glossary" @keyword)
(decision_block "decision" @keyword)
(constraint_block "constraint" @keyword)
(failure_mode_block "failure_mode" @keyword)

; ── Sub-block keywords ────────────────────────────────────────
(persona_def "persona" @keyword)
(surface_def "surface" @keyword)
(providers_block "providers" @keyword)
(coverage_block "coverage" @keyword)
(gen_block "gen" @keyword)
(term_def "term" @keyword)
(method_def "method" @keyword)
(verify_statement "verify" @keyword)
(scenario_block "scenario" @keyword)
(given_step "given" @keyword)
(when_step "when" @keyword)
(then_step "then" @keyword)

; ── Import keyword ────────────────────────────────────────────
(use_import "use" @keyword.import)

; ── Booleans ──────────────────────────────────────────────────
(boolean) @boolean

; ── Entity names (id fields) ──────────────────────────────────
(invariant_block
  id: (identifier) @constant)
(behavior_block
  id: (identifier) @constant)
(feature_block
  id: (identifier) @constant)
(event_block
  id: (identifier) @constant)
(capability_block
  id: (identifier) @constant)
(deliverable_block
  id: (identifier) @constant)
(roadmap_block
  id: (identifier) @constant)
(library_block
  id: (identifier) @constant)
(decision_block
  id: (identifier) @constant)
(constraint_block
  id: (identifier) @constant)
(failure_mode_block
  id: (identifier) @constant)

; ── Strings ───────────────────────────────────────────────────
(string) @string
(triple_quoted_string) @string

; ── Scheme refs ───────────────────────────────────────────────
(scheme_ref_id) @string.special

; ── Numbers and dates ─────────────────────────────────────────
(integer) @number
(date_literal) @number

; ── Annotations ───────────────────────────────────────────────
(annotation) @attribute

; ── Import paths ──────────────────────────────────────────────
(import_path) @module

; ── Key-value keys ────────────────────────────────────────────
(key_value
  key: (identifier) @property)

; ── Type names ────────────────────────────────────────────────
(type_struct
  name: (identifier) @type)
(type_union
  name: (identifier) @type)
(type_params
  (identifier) @type)
(generic_type
  (identifier) @type)
(array_type
  (identifier) @type)
(optional_type
  (identifier) @type)
(type_expr
  (identifier) @type)
(union_variants
  (identifier) @type)

; ── Type fields ───────────────────────────────────────────────
(type_field
  name: (identifier) @property)

; ── Methods ───────────────────────────────────────────────────
(method_def
  name: (identifier) @function.method)

; ── Parameters ────────────────────────────────────────────────
(param
  name: (identifier) @variable.parameter)

; ── Verify kind ───────────────────────────────────────────────
(verify_statement
  kind: (identifier) @type.qualifier)

; ── Brackets ──────────────────────────────────────────────────
["{" "}" "[" "]" "(" ")" "<" ">"] @punctuation.bracket

; ── Delimiters ────────────────────────────────────────────────
["," ":"] @punctuation.delimiter

; ── Special punctuation ───────────────────────────────────────
["->" "|" "="] @punctuation.special

; ── Operators ─────────────────────────────────────────────────
["?"] @operator
(array_type "[]" @operator)
