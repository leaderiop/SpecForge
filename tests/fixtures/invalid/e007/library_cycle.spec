spec "test" {
  version "1.0"
  plugins ["@specforge/product"]
}

library alpha "@test/alpha" {
  family core
  depends_on [beta]
}

library beta "@test/beta" {
  family core
  depends_on [alpha]
}
