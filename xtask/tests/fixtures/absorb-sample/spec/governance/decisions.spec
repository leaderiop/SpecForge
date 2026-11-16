// A decision naming the seed feature joins; one naming only that decision is two steps out and stays out.

use "features/w"

decision w_component "Components" {
  status           accepted
  context          "Extensions need isolation."
  decision         "Run each extension as a component."
  affects_features [w_host]
}

decision old_component "Old Components" {
  status        superseded
  context       "Extensions needed isolation before."
  decision      "Run each extension in a process."
  superseded_by w_component
}
