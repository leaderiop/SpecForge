// Core domain types — fundamental data shapes

type SpecFile {
  path       string    @readonly
  imports    string[]
  entities   Entity[]
  errors     ParseError[]  @optional
}

type Entity {
  id         EntityId  @readonly
  kind       EntityKind
  title      string
  fields     FieldMap
  sourceSpan SourceSpan  @readonly
}

type EntityId {
  kind       EntityKind @readonly
  raw        string     @readonly  @unique
}

type EntityKind = spec | invariant | behavior | feature | event
               | type_def | port | ref | capability | deliverable
               | roadmap | library | glossary | decision | constraint
               | failure_mode

type FieldMap {
  entries    FieldEntry[]
}

type FieldEntry {
  key        string
  value      FieldValue
}

type FieldValue = StringValue | ReferenceList | StringList | Block | VerifyList | ScenarioList | TestFileList

type StringValue {
  _tag       "StringValue"    @literal
  content    string
}

type ReferenceList {
  _tag       "ReferenceList"  @literal
  ids        EntityId[]
}

type StringList {
  _tag       "StringList"     @literal
  items      string[]
}

type Block {
  _tag       "Block"          @literal
  entries    FieldEntry[]
}

type VerifyList {
  _tag       "VerifyList"     @literal
  items      VerifyStatement[]
}

type VerifyStatement {
  kind       VerifyKind
  description string
}

type VerifyKind = unit | integration | property | load | e2e

type Scenario {
  title       string    @readonly
  steps       ScenarioStep[]
}

type ScenarioStep {
  kind        ScenarioStepKind
  description string
}

type ScenarioStepKind = given | when | then

type ScenarioList {
  _tag       "ScenarioList"    @literal
  items      Scenario[]
}

type TestFileRef {
  path         string    @readonly
  functionRef  string    @optional
}

type TestFileList {
  _tag       "TestFileList"    @literal
  items      TestFileRef[]
}

type SourceSpan {
  file       string    @readonly
  startLine  integer   @readonly
  startCol   integer   @readonly
  endLine    integer   @readonly
  endCol     integer   @readonly
}

type Persona {
  id          string   @readonly  @unique
  displayName string
  description string   @optional
}

type Surface {
  id          string   @readonly  @unique
  displayName string
  surfaceType string   @optional
}
