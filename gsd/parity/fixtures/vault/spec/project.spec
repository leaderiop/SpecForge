project vault "Vault" {
  name          "Vault"
  what          "A small encrypted store for secrets, driven from the command line"
  core_value    "A secret never touches the disk in the clear"
  context       "Built to prove the GSD security capability in SpecForge"
  key_decisions [D-01]
  constraints   [
    { kind tech_stack what "Node 20, no dependencies" why "it ships as one script" }
  ]
}
