// Diagnostic types — compiler messages and validation codes

use "types/core"
type Diagnostic {
  code       ValidationCode  @readonly
  severity   Severity
  message    string
  span       SourceSpan      @readonly
  context    string          @optional
  suggestion string          @optional
  verify unit "Diagnostic schema is valid"
}

// ValidationCode is a structured type with a display format: the prefix
// letter concatenated with the zero-padded number (e.g., E001, W012, I004).
// The canonical string form is used in diagnostics, documentation, and
// cross-references throughout the spec.
type ValidationCode {
  prefix     CodePrefix
  number     integer
  verify unit "ValidationCode schema is valid"
}

type CodePrefix = E | W | I

type Severity = error | warning | info

// Counts MUST equal the filtered length of the diagnostics array by severity:
// error_count == diagnostics.filter(d => d.severity == error).length, etc.
type DiagnosticBag {
  diagnostics Diagnostic[]
  error_count integer
  warn_count  integer
  info_count  integer
  verify unit "DiagnosticBag schema is valid"
}
