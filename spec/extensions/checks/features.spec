// @specforge/checks features

use "extensions/checks/behaviors"

feature ck_value_checks "Generic Value Checks" {
  status   in_progress
  problem  """
    A project's values carry dates, versions, URLs, paths and identifiers,
    and nothing says they are well formed or point at something real unless
    each domain writes the same checks again.
  """
  solution """
    @specforge/checks holds the checkers a cross-industry standard decides,
    in families a setting switches: files, formats, digits, links, git,
    structured and media. A domain pack adds its own on the same kit (ADR 0338).
  """
}
