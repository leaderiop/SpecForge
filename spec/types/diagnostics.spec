// Diagnostic types — compiler messages and validation codes

use types/core

type Diagnostic {
  code       ValidationCode  @readonly
  severity   Severity
  message    string
  span       SourceSpan      @readonly
  context    string          @optional
  suggestion string          @optional
}

type ValidationCode {
  prefix     CodePrefix
  number     integer
}

type CodePrefix = E | W | I

type Severity = error | warning | info

type DiagnosticBag {
  diagnostics Diagnostic[]
  errorCount  integer
  warnCount   integer
  infoCount   integer
}
