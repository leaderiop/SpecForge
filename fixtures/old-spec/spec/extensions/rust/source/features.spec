// @rust/source features

use "extensions/rust/source/behaviors"

feature rs_item_facts "Rust Item Facts" {
  status   in_progress
  problem  """
    A spec could cite a Rust test by its runner id but not the function,
    type or module it is about: nothing read a crate's items into facts, so
    an item a requirement names was a string no check could resolve.
  """
  solution """
    @rust/source walks each cargo target's module tree from its root by
    rustc's rules and has the host's reader parse each file with the pack's
    query. Every definition at module level, in inline modules and in impl
    and trait blocks becomes an item fact minted by specforge-code-facts,
    with its kind, visibility, signature digest, doc, cfg attributes and
    places (ADR 0348 D6).
  """
}
