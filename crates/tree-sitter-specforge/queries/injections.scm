; injections.scm — Language injection for SpecForge .spec files
; Highlight triple-quoted strings as markdown (contracts, descriptions, etc.)

((triple_quoted_string) @injection.content
 (#set! injection.language "markdown"))
