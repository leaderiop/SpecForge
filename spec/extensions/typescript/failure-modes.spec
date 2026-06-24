// @specforge/typescript extension failure modes -- FMEA risk analysis
//
// Failure modes specific to the TypeScript/JavaScript language integration:
// entity mapping accuracy, React detection, barrel resolution, monorepo handling.

use "extensions/typescript/invariants"

failure_mode ts_pascal_case_collision "PascalCase to snake_case Collision" {
  invariant  ts_entity_mapping_precedence
  severity   high
  occurrence occasional
  detection  likely
  rpn        36

  cause      "Two symbols with different PascalCase names map to the same snake_case entity ID -- e.g., UserAuth class and useAuth hook both produce 'user_auth' or 'auth'"
  effect     "Entity mapping links wrong symbol to spec entity. Coverage report attributes test results to wrong entity."
  mitigation "Collision detection produces diagnostic with both symbols. Hook prefix removal is documented. @specforge JSDoc tag provides explicit override. File-scoped qualification available (auth_hook vs auth_service)."

  post_mitigation {
    severity   medium
    occurrence rare
    detection  certain
    rpn        4
  }
  verify unit "PascalCase collision failure mode is handled"
}

failure_mode ts_barrel_infinite_loop "Barrel Re-export Infinite Loop" {
  invariant  ts_barrel_resolution_correctness
  severity   high
  occurrence unlikely
  detection  likely
  rpn        18

  cause      "Circular barrel re-exports: a/index.ts re-exports from b/index.ts which re-exports from a/index.ts. Occurs in poorly structured codebases or during refactoring."
  effect     "Scanner enters infinite loop or stack overflow during barrel resolution."
  mitigation "Visited-set tracking during barrel traversal. Maximum depth limit (32 levels). Circular reference produces E-level diagnostic with the cycle path."

  post_mitigation {
    severity   low
    occurrence rare
    detection  certain
    rpn        2
  }
  verify unit "Barrel infinite loop failure mode is handled"
}

failure_mode ts_react_false_positive "React Component False Positive" {
  invariant  ts_react_component_detection
  severity   medium
  occurrence occasional
  detection  likely
  rpn        24

  cause      "A PascalCase function returning an object literal is misidentified as a React component. Occurs when JSX detection relies solely on naming convention without checking return type."
  effect     "Non-component function gets React-specific inference signals and props extraction, confusing the agent."
  mitigation "Require at least TWO signals for component classification: (1) PascalCase name AND (2) JSX return OR React type annotation OR framework config. Single-signal detection produces lower confidence (0.3) not positive classification."

  post_mitigation {
    severity   low
    occurrence rare
    detection  likely
    rpn        6
  }
  verify unit "React false positive failure mode is handled"
}

failure_mode ts_monorepo_package_miss "Monorepo Package Miss" {
  severity   medium
  occurrence occasional
  detection  undetectable
  rpn        50

  cause      "Workspace glob pattern in root package.json/pnpm-workspace.yaml does not match an unconventionally located package. Or Nx/Turborepo uses project.json files that the detector does not follow."
  effect     "Source files in the missed package are not scanned. Entity IDs from that package are missing from the scan result."
  mitigation "After glob-based detection, scan for orphan package.json files not matched by any workspace glob. Produce I-level diagnostic for packages discovered outside workspace globs."

  post_mitigation {
    severity   low
    occurrence rare
    detection  likely
    rpn        6
  }
  verify unit "Monorepo package miss failure mode is handled"
}

failure_mode ts_test_runner_mismatch "Test Runner Format Mismatch" {
  invariant  ts_entity_mapping_precedence
  severity   high
  occurrence occasional
  detection  likely
  rpn        36

  cause      "Auto-detected runner format does not match actual test output. E.g., project has both jest.config.ts and vitest.config.ts; detector picks Jest but output is from Vitest."
  effect     "Parser fails or produces incorrect test results. Entity coverage is wrong."
  mitigation "Validate JSON structure before parsing (Jest and Vitest JSON differ in keys). Fall back to alternative runner on parse failure. --runner flag provides explicit override. Diagnostic when auto-detection is ambiguous."

  post_mitigation {
    severity   medium
    occurrence rare
    detection  certain
    rpn        4
  }
  verify unit "Test runner mismatch failure mode is handled"
}
