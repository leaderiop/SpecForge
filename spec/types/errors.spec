// Error types — typed errors for compiler operations

use types/core

type ParseError {
  _tag       "ParseError"      @literal
  message    string
  span       SourceSpan        @readonly
  expected   string            @optional
  found      string            @optional
}

type ResolutionError {
  _tag       "ResolutionError" @literal
  message    string
  span       SourceSpan        @readonly
  unresolved_id string
  suggestions  string[]        @optional
}

type CycleError {
  _tag       "CycleError"      @literal
  message    string
  span       SourceSpan        @readonly
  participants string[]
  // String representation of EdgeType.label for diagnostic display
  edge_type  string
}

type DuplicateIdError {
  _tag       "DuplicateIdError" @literal
  entity_id  string
  first_span SourceSpan        @readonly
  second_span SourceSpan       @readonly
}

type ValidationError {
  _tag       "ValidationError" @literal
  code       string
  message    string
  span       SourceSpan        @readonly
}

type EmitterError {
  _tag       "EmitterError"    @literal
  message    string
  output_path string            @optional
}

type ExtensionError {
  _tag            "ExtensionError"    @literal
  extension_name    string
  message         string
  wasm_trap       string            @optional
  lifecycle_state string            @optional
  peer_dependency string            @optional
}

type UnknownKindError {
  _tag         "UnknownKindError"  @literal
  code         string              @readonly
  message      string
  span         SourceSpan          @readonly
  keyword      string
  suggestions  string[]            @optional
}

type RegistryError {
  _tag          "RegistryError"     @literal
  message       string
  registry_url  string              @optional
  status_code   integer             @optional
}

type GrammarError {
  _tag            "GrammarError"    @literal
  message         string
  extension_name  string
  grammar_path    string            @optional
  abi_version     string            @optional
}

type BodyParserError {
  _tag              "BodyParserError" @literal
  message           string
  extension_name    string
  entity_kind       string          @optional
  raw_body          string          @optional
  timeout_exceeded  boolean         @optional
}
