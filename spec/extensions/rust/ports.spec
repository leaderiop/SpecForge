// @specforge/rust extension ports

use types/errors
use extensions/coverage/types
use extensions/rust/types

port RustTestOutputParser {
  direction outbound
  category  "testing/rust"

  method parseJunitXml(path: string) -> Result<TestResultEntry[], EmitterError>
  method parseLibtestJson(input: string) -> Result<TestResultEntry[], EmitterError>
  method parseCargoText(input: string) -> Result<TestResultEntry[], EmitterError>
  method readMappingFiles(dir: string) -> Result<EntityMappingEntry[], EmitterError>
}
