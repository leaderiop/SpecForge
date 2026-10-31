milestone v1.0 "Vault 1.0" {
  name "Vault 1.0"
}

phase 01 "Core" {
  milestone        v1.0
  name             "Core"
  slug             "core"
  goal             "Secrets can be stored and read back"
  requirements     [SEC-01, SEC-02]
  success_criteria [
    "A user can store a secret under a name",
    "A user can read a stored secret back by its name",
  ]
}

phase 02 "Sharing" {
  milestone        v1.0
  name             "Sharing"
  slug             "sharing"
  goal             "A secret can be shared with another person"
  depends_on       [01]
  requirements     [SHARE-01]
  success_criteria ["A user can export one secret, sealed for a recipient"]
}
