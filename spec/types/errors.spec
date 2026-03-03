// Error types — typed errors for compiler operations

use types/core

type ParseError {
  _tag       "ParseError"      @literal
  message    string
  span       SourceSpan
  expected   string            @optional
  found      string            @optional
}

type ResolutionError {
  _tag       "ResolutionError" @literal
  message    string
  span       SourceSpan
  unresolvedId string
  suggestions  string[]        @optional
}

type CycleError {
  _tag       "CycleError"      @literal
  message    string
  participants string[]
  edgeType   string
}

type DuplicateIdError {
  _tag       "DuplicateIdError" @literal
  entityId   string
  firstSpan  SourceSpan
  secondSpan SourceSpan
}

type ValidationError {
  _tag       "ValidationError" @literal
  code       string
  message    string
  span       SourceSpan
}

type EmitterError {
  _tag       "EmitterError"    @literal
  message    string
  outputPath string            @optional
}

type PackageError {
  _tag            "PackageError"    @literal
  packageName     string
  message         string
  wasmTrap        string            @optional
  lifecycleState  string            @optional
  peerDependency  string            @optional
}
