// Test fixture spec file

spec "test-project" {
  version "0.1.0"
}

type user "User account" {
  status draft
}

behavior authenticate_user "Authenticate a user with credentials" {
  status   draft
  contract "Given valid credentials, returns an auth token"
  types    [user]

  verify unit "rejects invalid password"
  verify unit "returns token on success"
}

event user_logged_in "User successfully logged in" {
  payload user
}
