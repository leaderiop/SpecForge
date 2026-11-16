plan 01-01 "Encrypted store" {
  phase          v1.0/01
  type           execute
  files_modified ["src/store.js"]
  requirements   [GOV-01, GOV-02]
  objective      "Keys are encrypted before they are written, and can be rotated"
}

task T1 "Encrypt on write" {
  plan   v1.0/01/01-01
  type   auto
  files  ["src/store.js"]
  action "Write src/store.js with put() that encrypts and get() that decrypts"
  verify [
    { automated "node --test test/store.test.js" fails_when "the test run exits non-zero" }
  ]
  done   "A key is encrypted at rest"
}

task T2 "Rotate a key" {
  plan   v1.0/01/01-01
  type   auto
  files  ["src/store.js"]
  action "Add rotate() to src/store.js, re-encrypting under the new key"
  verify [
    { automated "node --test test/store.test.js" fails_when "the test run exits non-zero" }
  ]
  done   "A key can be rotated"
}

truth TR1 "A key read back equals the key written" {
  plan v1.0/01/01-01
  text "A key read back equals the key written"
}

artifact AR1 "Key store" {
  plan     v1.0/01/01-01
  path     "src/store.js"
  provides "put(), get() and rotate()"
}

key_link KL1 "The store encrypts" {
  plan v1.0/01/01-01
  from "src/store.js"
  to   "node:crypto"
  via  "import"
}
