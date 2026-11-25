requirement FEATURE-056 "JavaScript Item Facts" {
  category "Feature"
  tier     v2
  text     """
    A spec could cite a JavaScript test by its runner id but not the
    function, class or module export it is about: nothing read a
    JavaScript or TypeScript project's source into facts, so an item a
    requirement names was a string no check could resolve.

    @javascript/source has the host's reader parse each source file a
    named package owns, generated output left out, with the pack's query:
    the typescript grammar for .ts .mts .cts, tsx for the rest. Every
    declaration at item scope and every member of a class or an interface
    there becomes an item fact js:<package>/<file>::<name>[.<member>]
    minted by specforge-code-facts, and every named package a
    pkg:npm/<name> package fact (ADR 0347). A test file is read through a
    second query for its statically named test calls only: each is a
    function item with the attribute test. A call node reports as a test
    case carries the test_id js:<package>/<file>#<suite > ... > name>
    node-test gives it: a skipped call, or one with no test, suite or
    subtest call inside it. A test or suite holding one is reported as a
    suite and carries none, and nothing inside a skipped call is an item
    (ADR 0393).
  """
}
