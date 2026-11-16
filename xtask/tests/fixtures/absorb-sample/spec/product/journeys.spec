// A journey of the seed feature joins; one of nothing sampled stays out.

use "product/personas"
use "features/w"

journey install_one "Install One" {
  persona  operator
  features [w_host]
  flow     ["Operator installs an extension", "Operator runs one task"]
}

journey wander "Wander" {
  persona operator
  flow    ["Operator wanders"]
}
