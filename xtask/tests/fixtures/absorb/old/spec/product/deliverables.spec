// Two deliverables sharing a journey. A journey with features stays out of them: a milestone it lists with no
// phase covering those features is a target warning (gsd/W530).

use "product/journeys"
use "product/modules"

deliverable store_cli "store-cli" {
  artifact_type cli
  description   "The command-line tool."
  status        shipped
  journeys      [browse_entries]
  modules       [store_module]
  owner         "Alice Martin"
}

deliverable store_web "store-web" {
  artifact_type web_app
  description   "The page that lists entries."
  journeys      [browse_entries]
  depends_on    [store_cli]
}
