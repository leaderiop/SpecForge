requirement FEATURE-153 "Playwright Test Facts" {
  category "Feature"
  tier     v2
  text     """
    A JavaScript project tested with Playwright had no runner pack: its
    end-to-end tests never became test facts a spec could cite by id.

    @javascript/playwright runs Playwright through npx --no, which never
    downloads it, when the project has a Playwright config, with its JUnit
    reporter; Playwright writes that report to standard output, which the
    host captures for the pack, and a saved report reads the same. The pack
    answers a test and a test_run fact per test, named
    js:<package>/<file>#<describe > test> after the nearest named
    package.json, as @javascript/node-test names a test; the file is
    relative to Playwright's testDir, as Playwright writes it. A failed,
    skipped or passed test keeps its status and time, and a test no named
    package owns is the collector's error.
  """
}
