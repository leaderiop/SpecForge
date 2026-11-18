// specforge-javascript-workspace behaviors

use "extensions/javascript/node-test/features"
use "extensions/javascript/source/features"
use "types/wasm"

behavior js_resolve_owning_package "Name the Package That Owns a File" {
  features [nt_node_test_collection, js_item_facts]
  category computation
  types    [CollectInput]
  contract """
    The extension library specforge-javascript-workspace MUST read every
    package.json the host serves with its text through serde_json; a file
    that could not be read, whose text is not JSON, or whose name is
    missing or not a string is no package. The owner of a project-relative
    file MUST be the deepest named package whose directory holds it (the
    root's, or a directory followed by `/`), with the path under that
    directory; none when no named package is at or above it. A package
    keeps its string version, its string engines and the names its four
    dependency tables list. Its id is pkg:npm/<name> for the shallowest
    package of its name (fewest segments, then lexically first) and
    pkg:npm/<name>#<dir> for every other, named <name>@<dir> (the dir
    percent-encoded) in a test or an item id. The packs that name a file after its package
    link it, so the tests and the items of one file name one package.
  """
  ensures {
    nearest_named "a file is owned by the deepest named package at or above it"
    whole_segment "a directory owns only the files under it, never a sibling sharing its prefix"
    one_rule      "every JavaScript pack names a file's package by this one rule"
  }
  verify unit "a file is owned by its nearest named package"
  verify unit "an unnamed package.json leaves the file to the one above"
  verify unit "an unread or non-JSON package.json is no package"
  verify unit "a file under no named package has no owner"
  verify unit "a sibling with a shared prefix does not own a file"
  verify unit "the root package owns what no nested one claims"
  verify unit "a package keeps its version, its engines and the names it depends on"
  verify unit "a second package of one name is named after its directory"
}
