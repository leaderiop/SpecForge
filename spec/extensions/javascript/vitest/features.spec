requirement FEATURE-151 "vitest Test Facts" {
  category "Feature"
  tier     v2
  text     """
    A JavaScript project tested with vitest had no runner pack for @code/core:
    the legacy @specforge/vitest gives entity results from vitest's JSON
    report, never test facts a spec could cite by id.

    @javascript/vitest runs vitest through npx --no, which never downloads
    it, when the project has a vitest config, with its JUnit reporter
    writing the report the pack reads. The pack answers a test and a
    test_run fact per test, named js:<package>/<file>#<describe > test>
    after the nearest named package.json, as @javascript/node-test names a
    test; a failed, skipped or passed test keeps its status and time, and a
    test no named package owns is the collector's error. The legacy pack
    keeps its collector vitest; this one is javascript-vitest.
  """
}
