// Forms of the language the repository's spec does not write, read by
// lexer_agrees_with_the_grammar, the form-start tests, body_statements_are_the_grammars
// and the LSP's token agreement test. It must parse without an error.
use "./other.spec"

behavior multi "Strings spanning lines" {
  contract "a regular string
    may run on across lines, as the grammar reads it"
  description "an escaped \" quote, and a \\ backslash"
  notes """a triple-quoted string with "one" and ""two"" quotes"""
  refs [gh.issue:42, web.page:https://x.io/a//b?q=1]
}

ref web.page:https://example.org/a//b "A page"

type state = "open" | "closed" | -1 | 2 | ready

type verify_typed "A field named verify typed by a name" {
  verify string
}

type verify_triple "A field named verify holding a triple-quoted string" {
  verify """not a description"""
}

type verify_bool "A field named verify holding a boolean" {
  verify true
}

type verify_number "A field named verify holding a number" {
  verify 42
}

type nested_members "Nested block members named verify and method" {
  meta {
    verify string
    method string
  }
  verify unit "a statement with a kind"
  verify "a statement with no kind"
  method get(id: string) -> string
  method put(id: string)
}

use { AUTH-01, v1.0 as release, 03 } from "./gsd.spec"

phase 10 "A number as a name" {
  after 03-02
  plan 03.1
  points 2-1
  padded 007
  fraction 1.5
  signed -5
  day 2026-10-26
  flag true
  refs [AUTH-01, v1.0, 2026-10-26, 03, 10, -5, 007, gh.issue:7]
  gate expr { a-b < 3 }
}

requirement D-01 "Dashed" {
}

requirement G-03-1 "Dashed twice" {
}

requirement H-01 "Typed members" {
  tags [threshold, jump]
  requires [
    { phase 02 plan 03-02 }
    { phase 03 note "x" },
  ]
  gate {
    inner { score 5 }
  }
  empty []
}

requirement H-02 "x" {
  r [ { plan 03-02 } ]
}

requirement H-03 "Verdicts" {
  verdict gsd/W311 passed {
    judge "j"
    cited [H-01, "src/a.ts"]
    inputs { plan "sha256:1" }
  }
  verdict W312 issues {
    verify string
  }
  verdict ValidatorVerdictKind
}
