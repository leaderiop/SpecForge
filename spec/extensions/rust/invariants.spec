// @specforge/rust extension invariants

use extensions/rust/behaviors

invariant entity_mapping_precedence "Entity Mapping Precedence" {
  guarantee """
    Test-to-entity resolution MUST follow strict precedence: tests field
    (1st) > proc macro attribute (2nd) > naming convention (3rd). A higher
    level MUST always override a lower level. No ambiguous mappings MUST
    exist after resolution — conflicts MUST produce diagnostics.
  """
  enforced_by [collect_rust_test_results, resolve_entity_mapping, validate_rust_entity_ids, record_test_via_drop_guard]
  risk high

  verify property "tests field always overrides proc macro and convention"
  verify unit "ambiguous mappings produce diagnostics"

}
