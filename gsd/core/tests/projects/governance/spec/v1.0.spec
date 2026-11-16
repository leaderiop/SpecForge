milestone v1.0 "Keystore 1.0" {
  name "Keystore 1.0"
}

phase 01 "Store" {
  milestone        v1.0
  name             "Store"
  slug             "store"
  goal             "Keys are stored encrypted and can be rotated"
  requirements     [GOV-01, GOV-02]
  success_criteria ["A key is encrypted at rest", "A key can be rotated"]
}
