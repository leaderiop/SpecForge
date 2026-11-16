// Constraints: a bound line with prose, a strict bound, a prose-only metric, a unitless bound, a one-name metric
// with a threshold, interoperability, an enforced_by value and a constrains list.

use "governance/invariants"
use "behaviors/b"

constraint write_latency "Write Latency" {
  description "A write reaches the disk quickly."
  category    performance
  priority    critical
  metric      """
    latency <= 95.0ms
    measured at the 99th percentile
  """
  constrains  [write_entry]
  enforced_by [write_entry]
  protects    [writes_are_durable]
  verify load "benchmark a thousand writes, assert the 99th percentile"
}

constraint retry_window "Retry Window" {
  description "A failed write is retried for a short while."
  category    reliability
  priority    medium
  metric      "wait < 30s"
}

constraint calm_wait "Calm Wait" {
  description "A person never waits long."
  category    usability
  priority    low
  metric      "Fast enough that nobody reaches for the button twice."
}

constraint bounded_retries "Bounded Retries" {
  description "A client retries a few times, then gives up."
  category    interoperability
  priority    high
  metric      "retries <= 3"
}

constraint write_throughput "Write Throughput" {
  description "The store keeps up with a busy day."
  category    performance
  priority    high
  metric      "throughput"
  threshold   ">= 200"
}
