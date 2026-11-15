// Two deliverables sharing a journey, and one listing the journey with features: the milestone it becomes has no
// phase, so the requirement the journey names is no target warning (gsd/W530 waits for the roadmap's first phase).

use "product/journeys"
use "product/modules"

deliverable store_cli "store-cli" {
  artifact_type cli
  description   "The command-line tool."
  status        shipped
  journeys      [browse_entries, record_an_entry]
  modules       [store_module]
  owner         "Alice Martin"
}

deliverable store_web "store-web" {
  artifact_type web_app
  description   "The page that lists entries."
  journeys      [browse_entries]
  depends_on    [store_cli]
}
