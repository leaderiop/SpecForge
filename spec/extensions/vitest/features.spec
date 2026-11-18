requirement FEATURE-132 "vitest Collection" {
  category "Feature"
  tier     v2
  text     """
    TypeScript projects tested with vitest had no way to tell SpecForge
    which entity a test proves, so coverage could only count obligations,
    never proof.

    @specforge/vitest is the vitest runner extension (ADR 0002). Tests name
    the entity they prove in vitest's own test metadata; `specforge
    collect` runs vitest with the user's consent and hands the JSON report
    to the extension's pure mapping.
  """
}
