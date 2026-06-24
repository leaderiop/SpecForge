// @specforge/typescript extension invariants

invariant ts_entity_mapping_precedence "TypeScript Entity Mapping Precedence" {
  guarantee """
    Test-to-entity resolution MUST follow strict precedence: (1) tests field
    in .spec files (authoritative), (2) @specforge JSDoc tag (explicit),
    (3) naming convention with PascalCase-to-snake_case conversion (implicit).
    Higher levels MUST always override lower levels. Ambiguous mappings MUST
    produce diagnostics.
  """
  risk high

  verify property "tests field always overrides JSDoc tag and naming convention"
  verify unit "ambiguous mappings produce diagnostics"
}

invariant ts_barrel_resolution_correctness "Barrel Re-export Resolution" {
  guarantee """
    When a symbol is re-exported through a barrel file (index.ts), the source
    anchor MUST resolve to the original defining file, NOT the barrel file.
    Chains of re-exports (a/index.ts re-exports from b/index.ts which
    re-exports from c/impl.ts) MUST be fully resolved. Circular re-exports
    MUST produce an error diagnostic, not infinite recursion.
  """
  risk high

  verify unit "barrel re-exports resolve to original source"
  verify unit "chained re-exports are fully resolved"
  verify unit "circular re-exports produce error diagnostic"
}

invariant ts_file_role_accuracy "File Role Classification Accuracy" {
  guarantee """
    Test files, story files, config files, and generated files MUST NOT
    be classified as production code. The inverse MUST also hold: production
    source files MUST NOT be misclassified as test files. When multiple
    signals conflict (e.g., a file in __tests__/ that is also .generated.ts),
    the most specific signal MUST win.
  """
  risk medium

  verify unit "test files are never classified as production"
  verify unit "production files are never classified as test"
  verify unit "conflicting signals resolve by specificity"
}

invariant ts_react_component_detection "React Component Detection" {
  guarantee """
    All exported React components MUST be detected regardless of declaration
    style: function declarations, arrow function expressions, React.FC/FC
    annotations, forwardRef wrappers, memo wrappers, class components
    extending React.Component or React.PureComponent. Components MUST be
    distinguished from regular functions by at least one signal: JSX return
    type, React type annotation, or capitalized name returning JSX.
  """
  risk medium

  verify unit "function component with JSX return is detected"
  verify unit "arrow function component with FC type is detected"
  verify unit "forwardRef component is detected"
  verify unit "memo component is detected"
  verify unit "class component extending React.Component is detected"
  verify unit "regular function is not misidentified as component"
}

invariant ts_export_completeness "Export Completeness" {
  guarantee """
    The scanner MUST extract ALL publicly exported symbols from a source
    file. This includes: named exports, default exports, re-exports
    (export { x } from './y'), namespace re-exports (export * from './y'),
    and export assignments (module.exports = x). Internally-scoped symbols
    (not exported) MUST NOT appear in scan results.
  """
  risk high

  verify unit "named exports are extracted"
  verify unit "default exports are extracted"
  verify unit "re-exports are extracted"
  verify unit "namespace re-exports are tracked"
  verify unit "non-exported symbols are excluded"
}
