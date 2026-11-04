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
  verify unit "a family whose switch is off reports nothing"
}

behavior ck_formats_family "Formats Family" {
  features [ck_value_checks]
  category command
  contract """
    Each formats checker MUST decide a value from its text alone, by the
    standard it names, and report E103 on the part of the value's span the
    kit points at. Each list item MUST be judged alone, and an empty value
    MUST be left to the kernel's `required`.
  """
  verify unit "a format that refuses a value reports E103 on the part of the value's span the kit points at"
}

behavior ck_digits_family "Digits Family" {
  features [ck_value_checks]
  category command
  contract """
    Each digits checker MUST decide a value from its text alone, MUST name
    the check digit it expected, and MUST need no scope and no approval.
  """
  verify unit "a digit checker needs no scope and asks no approval"
}
