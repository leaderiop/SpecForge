plan 01-01 "Typing" {
  phase          v1.0/01
  type           execute
  files_modified ["src/impl.txt"]
  requirements   [REQ-00]
  objective      "Implement D-01 typing"
}

truth TR1 "Tokens are cached" {
  plan v1.0/01/01-01
  text "D-02 tokens are cached"
}

artifact AR1 "The implementation" {
  plan     v1.0/01/01-01
  path     "src/impl.txt"
  provides "typing"
}

plan 02-01 "Web checks" {
  phase          v1.0/02
  type           execute
  files_modified ["web/app.js", "web/lint.js"]
  requirements   [WEB-01, WEB-02]
  objective      "Run the web tests and the lint"
}

task T1 "Run web tests" {
  plan   v1.0/02/02-01
  type   auto
  files  ["web/app.js"]
  action "Run the web tests"
  verify [
    { automated "cd web && npm test" fails_when "non-zero exit" }
  ]
  done   "The web tests run"
}

task T2 "Lint the missing package" {
  plan   v1.0/02/02-01
  type   auto
  files  ["web/lint.js"]
  action "Lint the missing package"
  verify [
    { automated "npm run lint --prefix ./missing" }
  ]
  done   "The lint runs"
}

truth TR1 "The web tests run" {
  plan v1.0/02/02-01
  text "The web tests run"
}

artifact AR1 "The web app" {
  plan     v1.0/02/02-01
  path     "web/app.js"
  provides "the web app"
}

plan 03-01 "Clean web checks" {
  phase          v1.0/03
  type           execute
  files_modified ["web/app.js"]
  requirements   [REQ-03]
  objective      "Run the web tests"
}

task T1 "Run web tests" {
  plan   v1.0/03/03-01
  type   auto
  files  ["web/app.js"]
  action "Run the web tests"
  verify [
    { automated "cd web && npm test" fails_when "non-zero exit" }
  ]
  done   "The web tests run"
}

truth TR1 "The web tests run" {
  plan v1.0/03/03-01
  text "The web tests run"
}

artifact AR1 "The web app" {
  plan     v1.0/03/03-01
  path     "web/app.js"
  provides "the web app"
}

plan 04-01 "Sign in" {
  phase          v1.0/04
  type           execute
  files_modified ["src/auth.js"]
  requirements   [REQ-01]
  objective      "Implement D-01 and REQ-01 sign in"
}

truth TR1 "Users can sign in" {
  plan v1.0/04/04-01
  text "Users can sign in"
}

artifact AR1 "The auth module" {
  plan     v1.0/04/04-01
  path     "src/auth.js"
  provides "sign in"
}
