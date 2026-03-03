spec "test" {
  version "1.0"
  providers {
    gh "test" {
      package "@specforge/gh"
      kinds [issue, pr]
      id_patterns {
        issue "^\d+$"
        pr    "^\d+$"
      }
    }
  }
}

ref gh.issue:abc "Non-numeric Issue ID"
