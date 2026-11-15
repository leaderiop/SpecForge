project keystore "Keystore" {
  name        "Keystore"
  what        "A small store for secrets on disk"
  core_value  "A secret is never lost or leaked"
  context     "Built to prove the risk kind in SpecForge"
  constraints [
    { kind tech_stack what "Node 20, node:crypto only" why "no key leaves the process" }
  ]
}

requirement GOV-01 "Keys at rest" {
  text "A key is encrypted before it is written"
  area "Secrets"
  tier v1
}

requirement GOV-02 "Keys rotate" {
  text "A key can be rotated without losing what it encrypted"
  area "Secrets"
  tier v1
}
