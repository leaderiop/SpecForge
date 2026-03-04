behavior auth_login "Auth Login" {
  contract """The system MUST authenticate users with valid credentials."""
  verify unit "accepts valid credentials"
  tests "tests/auth.test.ts"
}

behavior auth_logout "Auth Logout" {
  contract """The system MUST invalidate session on logout."""
  verify unit "clears session"
  tests "tests/auth.test.ts"
}
