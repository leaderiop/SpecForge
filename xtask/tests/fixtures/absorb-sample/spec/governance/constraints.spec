// One constraint referring to a through behavior (joins), one to an outside behavior (stays out).

use "behaviors/w-a"
use "behaviors/other"

constraint w_fast "Fast Load" {
  description "An extension loads quickly."
  category    performance
  priority    high
  metric      "load_time < 50ms"
  constrains  [w_run]
}

constraint other_fast "Other Fast" {
  description "Something else is quick."
  category    performance
  priority    low
  metric      "other_time < 5ms"
  constrains  [other_run]
}
