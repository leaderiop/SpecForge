// One module: no kind holds it yet.

use "product/features"

module store_module "store" {
  family      core
  description "The code that reads and writes entries."
  features    [store_entries]
  tags        ["leaf"]
}
