spec "test" {
  version "1.0"
  providers {
    gh "test" {
      package "@specforge/gh"
      kinds [issue, pr]
    }
  }
}

ref gh.bogus:42 "Bad Kind Ref"
