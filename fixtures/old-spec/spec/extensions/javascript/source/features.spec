// @javascript/source features

use "extensions/javascript/source/behaviors"

feature js_item_facts "JavaScript Item Facts" {
  status   in_progress
  problem  """
    A spec could cite a JavaScript test by its runner id but not the
    function, class or module export it is about: nothing read a
    JavaScript or TypeScript project's source into facts, so an item a
    requirement names was a string no check could resolve.
  """
  solution """
    @javascript/source has the host's reader parse each source file a
    named package owns, test files and generated output left out, with the
    pack's query: the typescript grammar for .ts .mts .cts, tsx for the
    rest. Every declaration at item scope and every member of a class or
    an interface there becomes an item fact
    js:<package>/<file>::<name>[.<member>] minted by specforge-code-facts,
    and every named package a pkg:npm/<name> package fact (ADR 0347).
  """
}
