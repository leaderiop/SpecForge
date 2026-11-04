// @specforge/checks behaviors

behavior ck_family_switches "Family Switches" {
  features [ck_value_checks]
  category command
  contract """
    @specforge/checks MUST declare one bool setting per family, on by
    default. While a family's setting is false its checkers MUST NOT be
    called, send no probe and add no scope to what the run asks approval
    for, and the setting MUST gate the codes only that family reports.
  """
  verify unit "the extension declares the scopes files, links and git, and five family switches on by default"
  verify unit "each switch gates only codes its family alone reports, and E103 under no switch"
  verify unit "every code a checks switch gates is catalogued with owner @specforge/checks"
}
