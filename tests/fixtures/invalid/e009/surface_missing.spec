spec "test" {
  version "1.0"
  plugins ["@specforge/product"]

  persona admin "Admin" {
    description "Administrator"
  }
}

behavior manage_users "Manage Users" {
  contract """admin can manage users"""
}

feature user_management "User Management" {
  behaviors [manage_users]
  problem """need user management"""
  solution """add management"""
  status active
}

capability admin_panel "Admin Panel" {
  persona admin
  surface dashboard
  features [user_management]
}
