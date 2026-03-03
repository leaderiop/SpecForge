behavior login_flow "Login Flow" {
  contract """user can login"""
  scenario "no when step" {
    given "user is on login page"
    then "user sees dashboard"
  }
}
