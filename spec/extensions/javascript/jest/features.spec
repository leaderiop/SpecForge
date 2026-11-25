requirement FEATURE-152 "jest Test Facts" {
  category "Feature"
  tier     v2
  text     """
    A JavaScript project tested with jest had no runner pack: its tests
    never became test facts a spec could cite by id.

    @javascript/jest runs the project's jest through node --env-file, the
    file being the pack's config, which the host writes for the run and
    which sets jest-junit's options: the report path, the file attribute
    and the test naming. jest-junit is a dev dependency of the project. The
    pack answers a test and a test_run fact per test, named
    js:<package>/<file>#<describe > test> after the nearest named
    package.json, as @javascript/node-test names a test; a failed, skipped
    or passed test keeps its status and time, and a test no named package
    owns, or with no file, is the collector's error.
  """
}
