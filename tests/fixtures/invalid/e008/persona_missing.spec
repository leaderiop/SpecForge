spec "test" {
  version "1.0"
  plugins ["@specforge/product"]

  surface web "Web" {
    type browser
  }
}

behavior browse_items "Browse Items" {
  contract """user can browse"""
}

feature browsing "Browsing" {
  behaviors [browse_items]
  problem """need to browse"""
  solution """add browsing"""
  status active
}

capability browse_catalog "Browse Catalog" {
  persona shopper
  surface web
  features [browsing]
}
