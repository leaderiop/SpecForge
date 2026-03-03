behavior checkout_flow "Checkout" {
  contract """user can checkout"""
  scenario "happy path" {
    given "user has items in cart"
    when "user clicks checkout"
    then "order is placed"
  }
  scenario "happy path" {
    given "user has one item"
    when "user checks out"
    then "order is created"
  }
}
