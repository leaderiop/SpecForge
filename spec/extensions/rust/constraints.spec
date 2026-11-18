requirement RELIABILITY-32 "Test Coverage Accuracy" {
  category "Reliability"
  tier     v2
  text     """
    Ensures coverage percentages are accurate and merge operations produce correct deduplicated results.

    coverage_ratio_error <= 0
    coverage percentage matches actual verified/total ratio;
    merge produces correct deduplicated results
  """
}

requirement RELIABILITY-31 "Rust Collection Accuracy" {
  category "Reliability"
  tier     v2
  text     """
    Ensures entity mapping has zero false positives and every recorded result names a declared entity.

    entity mapping has zero false positives; every recorded result names a
    declared entity, and results that don't are reported as W115
  """
}
